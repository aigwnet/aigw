use std::net::{IpAddr, ToSocketAddrs};

use aigw_core::IpList;
use aya::{
    Ebpf,
    maps::{HashMap, LpmTrie, lpm_trie::Key},
    programs::{Xdp, XdpFlags},
};
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

pub struct EpbfConfig {
    pub iface: String,
    pub path: Option<String>,
}

pub fn run(config: &EpbfConfig, address: &str) -> anyhow::Result<EbpfHandler> {
    // Bump the memlock rlimit. This is needed for older kernels that don't use the
    // new memcg based accounting, see https://lwn.net/Articles/837122/
    let rlim = libc::rlimit {
        rlim_cur: libc::RLIM_INFINITY,
        rlim_max: libc::RLIM_INFINITY,
    };
    let ret = unsafe { libc::setrlimit(libc::RLIMIT_MEMLOCK, &rlim) };
    if ret != 0 {
        debug!("remove limit on locked memory failed, ret is: {ret}");
    }

    let mut ebpf = if let Some(epbf) = &config.path {
        aya::Ebpf::load_file(epbf)?
    } else {
        aya::Ebpf::load(aya::include_bytes_aligned!(concat!(
            env!("OUT_DIR"),
            "/aigwe"
        )))?
    };
    match aya_log::EbpfLogger::init(&mut ebpf) {
        Err(e) => {
            // This can happen if you remove all log statements from your eBPF program.
            warn!("failed to initialize eBPF logger: {e}");
        }
        Ok(logger) => {
            let mut logger =
                tokio::io::unix::AsyncFd::with_interest(logger, tokio::io::Interest::READABLE)?;
            tokio::task::spawn(async move {
                loop {
                    let mut guard = logger.readable_mut().await.unwrap();
                    guard.get_inner_mut().flush();
                    guard.clear_ready();
                }
            });
        }
    }

    let program: &mut Xdp = ebpf.program_mut("aigw").unwrap().try_into()?;
    program.load()?;
    if let Err(e) = program.attach(&config.iface, XdpFlags::default()) {
        error!("Native XDP attach failed ({}), falling back to SKB mode", e);
        program.attach(&config.iface, XdpFlags::SKB_MODE)?;
    }

    Ok(EbpfHandler::new(ebpf, address)?)
}

pub struct EbpfHandler {
    ebpf: Mutex<Ebpf>,
}

type Ipv4AndV6Iptems = (
    Vec<(u32, std::net::Ipv4Addr)>,
    Vec<(u32, std::net::Ipv6Addr)>,
);

impl EbpfHandler {
    pub fn new(mut ebpf: Ebpf, address: &str) -> anyhow::Result<Self> {
        let addrs = address.to_socket_addrs()?.collect::<Vec<_>>();

        {
            let mut map_ipv4: HashMap<_, u32, u32> =
                HashMap::try_from(ebpf.map_mut("WHITELIST_IPV4").unwrap())?;

            for addr in &addrs {
                match addr.ip() {
                    IpAddr::V4(ipv4_addr) => {
                        info!(target: "console",
                            "Add ip {:?} to WHITELIST_IPV4  list",ipv4_addr
                        );
                        let ip = u32::from_be_bytes(ipv4_addr.octets());
                        map_ipv4.insert(ip, 1, 0)?;
                    }
                    IpAddr::V6(_) => {}
                }
            }
        }

        {
            let mut map_ipv6: HashMap<_, u128, u32> =
                HashMap::try_from(ebpf.map_mut("WHITELIST_IPV6").unwrap())?;
            for addr in &addrs {
                match addr.ip() {
                    IpAddr::V4(_) => {}
                    IpAddr::V6(ipv6_addr) => {
                        info!(target: "console",
                            "Add ip {:?} to WHITELIST_IPV6  list",ipv6_addr
                        );
                        let ip = u128::from_be_bytes(ipv6_addr.octets());
                        map_ipv6.insert(ip, 1, 0)?;
                    }
                }
            }
        }

        Ok(Self {
            ebpf: Mutex::new(ebpf),
        })
    }

    ///
    /// Updating the IP list is a hazardous operation.
    ///
    pub async fn handle_update(&self, ip_list: IpList) -> anyhow::Result<()> {
        let ip_data = ip_list
            .data
            .iter()
            .map(|i| (i.prefix_len, i.data.clone()))
            .collect();

        let (ipv4, ipv6) = self.parse_ip_entries(ip_data)?;
        match ip_list.item_type {
            1 => {
                //
                self.update_maps(&ipv4, &ipv6, "WHITELIST_IPV4_CIDR", "WHITELIST_IPV6_CIDR")
                    .await
            }
            2 => {
                self.update_maps(&ipv4, &ipv6, "BLOCKLIST_IPV4_CIDR", "BLOCKLIST_IPV6_CIDR")
                    .await
            }
            _ => Ok(()), // ignore unsupported item types
        }
    }

    pub async fn handle_delete(&self, ip_list: IpList) -> anyhow::Result<()> {
        let ip_data = ip_list
            .data
            .iter()
            .map(|i| (i.prefix_len, i.data.clone()))
            .collect();

        let (ipv4, ipv6) = self.parse_ip_entries(ip_data)?;
        match ip_list.item_type {
            1 => {
                self.delete_maps(&ipv4, &ipv6, "WHITELIST_IPV4_CIDR", "WHITELIST_IPV6_CIDR")
                    .await
            }
            2 => {
                self.delete_maps(&ipv4, &ipv6, "BLOCKLIST_IPV4_CIDR", "BLOCKLIST_IPV6_CIDR")
                    .await
            }
            _ => Ok(()),
        }
    }

    ///
    /// When enabling IP whitelisting, it is extremely dangerous—
    /// you must ensure uninterrupted communication between the gateway and the configuration center.
    ///
    pub async fn handle_switch(
        &self,
        enable_white_list: bool,
        enable_block_list: bool,
    ) -> anyhow::Result<()> {
        //
        let ebpf = &mut *self.ebpf.lock().await;
        let mut map: HashMap<_, u32, u32> = HashMap::try_from(ebpf.map_mut("SWITCH").unwrap())?;

        if enable_white_list {
            map.insert(1, 1, 0)?;
        } else {
            let _ = map.remove(&1);
        }

        if enable_block_list {
            map.insert(2, 1, 0)?;
        } else {
            let _ = map.remove(&2);
        }
        Ok(())
    }

    fn parse_ip_entries(&self, items: Vec<(u32, String)>) -> anyhow::Result<Ipv4AndV6Iptems> {
        let mut ipv4 = vec![];
        let mut ipv6 = vec![];
        for (prefix_len, item) in items {
            let ip_addr: IpAddr = item.parse()?;
            match ip_addr {
                IpAddr::V4(addr) => ipv4.push((prefix_len, addr)),
                IpAddr::V6(addr) => ipv6.push((prefix_len, addr)),
            }
        }
        Ok((ipv4, ipv6))
    }

    async fn update_maps(
        &self,
        ipv4_entries: &[(u32, std::net::Ipv4Addr)],
        ipv6_entries: &[(u32, std::net::Ipv6Addr)],
        ipv4_map_name: &str,
        ipv6_map_name: &str,
    ) -> anyhow::Result<()> {
        let ebpf = &mut *self.ebpf.lock().await;
        if !ipv4_entries.is_empty() {
            let mut map: LpmTrie<_, [u8; 4], u32> =
                LpmTrie::try_from(ebpf.map_mut(ipv4_map_name).unwrap())?;
            for &(prefix_len, addr) in ipv4_entries {
                let key = Key::new(prefix_len, addr.octets());
                map.insert(&key, 1, 0)?;

                info!(target: "console",
                    "Add ip {:?}/{} to {} list",addr, prefix_len, ipv4_map_name
                );
            }
        }

        if !ipv6_entries.is_empty() {
            let mut map: LpmTrie<_, [u8; 16], u32> =
                LpmTrie::try_from(ebpf.map_mut(ipv6_map_name).unwrap())?;
            for &(prefix_len, addr) in ipv6_entries {
                let key = Key::new(prefix_len, addr.octets());
                map.insert(&key, 1, 0)?;

                info!(target: "console",
                    "Add ip {:?}/{} to {} list",addr, prefix_len, ipv6_map_name
                );
            }
        }

        Ok(())
    }

    async fn delete_maps(
        &self,
        ipv4_entries: &[(u32, std::net::Ipv4Addr)],
        ipv6_entries: &[(u32, std::net::Ipv6Addr)],
        ipv4_map_name: &str,
        ipv6_map_name: &str,
    ) -> anyhow::Result<()> {
        let ebpf = &mut *self.ebpf.lock().await;
        if !ipv4_entries.is_empty() {
            let mut map: LpmTrie<_, [u8; 4], u32> =
                LpmTrie::try_from(ebpf.map_mut(ipv4_map_name).unwrap())?;
            for &(prefix_len, addr) in ipv4_entries {
                let key = Key::new(prefix_len, addr.octets());
                let _ = map.remove(&key);

                info!(target: "console",
                    "Removed ip {:?}/{} from {} list",addr, prefix_len, ipv4_map_name
                );
            }
        }

        if !ipv6_entries.is_empty() {
            let mut map: LpmTrie<_, [u8; 16], u32> =
                LpmTrie::try_from(ebpf.map_mut(ipv6_map_name).unwrap())?;
            for &(prefix_len, addr) in ipv6_entries {
                let key = Key::new(prefix_len, addr.octets());
                let _ = map.remove(&key);

                info!(target: "console",
                    "Removed ip {:?}/{} from {} list",addr, prefix_len, ipv6_map_name
                );
            }
        }

        Ok(())
    }
}
