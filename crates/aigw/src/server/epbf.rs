use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, ToSocketAddrs};

use aigw_core::{IpItem, IpList};
use aya::{
    Ebpf,
    maps::{HashMap, LpmTrie, lpm_trie::Key},
    programs::{Xdp, XdpFlags},
};
use ipnet::{Ipv4Net, Ipv6Net};
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

    Ok(EbpfHandler {
        ebpf: Mutex::new(ebpf),
        address: address.to_owned(),
    })
}

pub struct EbpfHandler {
    ebpf: Mutex<Ebpf>,
    address: String,
}

type Ipv4AndV6Iptems = (
    Vec<(u32, std::net::Ipv4Addr)>,
    Vec<(u32, std::net::Ipv6Addr)>,
);

impl EbpfHandler {
    ///
    /// Updating the IP list is a hazardous operation.
    ///
    /// When updating the IP blacklist, must strictly verify whether the configuration
    /// center's IP address is included in the blacklist. If it is present, that entry must be skipped.
    /// Failing to skip it will disrupt communication between the gateway and the configuration center,
    /// rendering all subsequent control commands from the configuration center ineffective.
    ///
    pub async fn handle_update(&self, ip_list: IpList) -> anyhow::Result<()> {
        let ebpf = &mut *self.ebpf.lock().await;

        let ip_data = ip_list
            .data
            .iter()
            .map(|i| (i.prefix_len, i.data.clone()))
            .collect();

        let (ipv4, ipv6) = self.parse_ip_entries(ip_data)?;
        match ip_list.item_type {
            1 => {
                //
                let (ipv4, ipv6) = self.filter_address_ip(ipv4, ipv6)?;
                self.update_maps(
                    ebpf,
                    &ipv4,
                    &ipv6,
                    "WHITELIST_IPV4_CIDR",
                    "WHITELIST_IPV6_CIDR",
                )
            }
            2 => self.update_maps(
                ebpf,
                &ipv4,
                &ipv6,
                "BLOCKLIST_IPV4_CIDR",
                "BLOCKLIST_IPV6_CIDR",
            ),
            _ => Ok(()), // ignore unsupported item types
        }
    }

    pub async fn handle_delete(&self, ip_list: IpList) -> anyhow::Result<()> {
        let ebpf = &mut *self.ebpf.lock().await;

        let ip_data = ip_list
            .data
            .iter()
            .map(|i| (i.prefix_len, i.data.clone()))
            .collect();

        let (ipv4, ipv6) = self.parse_ip_entries(ip_data)?;
        match ip_list.item_type {
            1 => self.delete_maps(
                ebpf,
                &ipv4,
                &ipv6,
                "WHITELIST_IPV4_CIDR",
                "WHITELIST_IPV6_CIDR",
            ),
            2 => self.delete_maps(
                ebpf,
                &ipv4,
                &ipv6,
                "BLOCKLIST_IPV4_CIDR",
                "BLOCKLIST_IPV6_CIDR",
            ),
            _ => Ok(()),
        }
    }

    ///
    /// When enabling IP whitelisting, it is extremely dangerous—
    /// you must ensure uninterrupted communication between the gateway and the configuration center.
    /// Therefore, the IP address of the configuration center must be added to the whitelist.
    /// If it is not included, any misconfiguration that disrupts communication between the configuration center
    /// and the gateway will prevent you from making further changes via the configuration center.
    /// In such a case, the only recovery option is to log in directly to the machine where the gateway is
    /// running and perform manual operations—such as stopping the gateway—to restore connectivity.
    ///
    /// Conversely, if IP whitelisting is disabled, the configuration center's IP address should be removed
    /// from the whitelist—although leaving it in place does not affect the gateway's operation.
    ///
    pub async fn handle_switch(
        &self,
        enable_white_list: bool,
        enable_block_list: bool,
    ) -> anyhow::Result<()> {
        //
        let ebpf = &mut *self.ebpf.lock().await;

        let mut map: HashMap<_, u32, u32> = HashMap::try_from(ebpf.map_mut("SWITCH").unwrap())?;
        let ip_list = self.get_address_ip_list()?;
        if enable_white_list {
            self.handle_update(ip_list).await?;
            map.insert(1, 1, 0)?;
        } else {
            self.handle_delete(ip_list).await?;
            map.remove(&1)?;
        }

        if enable_block_list {
            map.insert(2, 1, 0)?;
        } else {
            map.remove(&2)?;
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

    fn update_maps(
        &self,
        ebpf: &mut Ebpf,
        ipv4_entries: &[(u32, std::net::Ipv4Addr)],
        ipv6_entries: &[(u32, std::net::Ipv6Addr)],
        ipv4_map_name: &str,
        ipv6_map_name: &str,
    ) -> anyhow::Result<()> {
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

    fn delete_maps(
        &self,
        ebpf: &mut Ebpf,
        ipv4_entries: &[(u32, std::net::Ipv4Addr)],
        ipv6_entries: &[(u32, std::net::Ipv6Addr)],
        ipv4_map_name: &str,
        ipv6_map_name: &str,
    ) -> anyhow::Result<()> {
        if !ipv4_entries.is_empty() {
            let mut map: LpmTrie<_, [u8; 4], u32> =
                LpmTrie::try_from(ebpf.map_mut(ipv4_map_name).unwrap())?;
            for &(prefix_len, addr) in ipv4_entries {
                let key = Key::new(prefix_len, addr.octets());
                map.remove(&key)?;

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
                map.remove(&key)?;

                info!(target: "console",
                    "Removed ip {:?}/{} from {} list",addr, prefix_len, ipv6_map_name
                );
            }
        }

        Ok(())
    }

    fn filter_address_ip(
        &self,
        ipv4: Vec<(u32, Ipv4Addr)>,
        ipv6: Vec<(u32, Ipv6Addr)>,
    ) -> anyhow::Result<Ipv4AndV6Iptems> {
        let resolved_ips: Vec<IpAddr> = self.address.to_socket_addrs()?.map(|sa| sa.ip()).collect();
        if resolved_ips.is_empty() {
            return Ok((ipv4, ipv6));
        }

        let ipv4_nets: Vec<(u32, Ipv4Addr, Ipv4Net)> = ipv4
            .into_iter()
            .map(|(prefix, ip)| {
                let net = Ipv4Net::new(ip, prefix as u8)?;
                Ok((prefix, ip, net))
            })
            .collect::<anyhow::Result<_>>()?;

        let ipv6_nets: Vec<(u32, Ipv6Addr, Ipv6Net)> = ipv6
            .into_iter()
            .map(|(prefix, ip)| {
                let net = Ipv6Net::new(ip, prefix as u8)?;
                Ok((prefix, ip, net))
            })
            .collect::<anyhow::Result<_>>()?;

        let filtered_ipv4 = ipv4_nets
            .into_iter()
            .filter(|(_, _, net)| {
                !resolved_ips.iter().any(|ip| match ip {
                    IpAddr::V4(v4) => net.contains(v4),
                    IpAddr::V6(_) => false,
                })
            })
            .map(|(prefix, ip, _)| (prefix, ip))
            .collect();

        let filtered_ipv6 = ipv6_nets
            .into_iter()
            .filter(|(_, _, net)| {
                !resolved_ips.iter().any(|ip| match ip {
                    IpAddr::V4(_) => false,
                    IpAddr::V6(v6) => net.contains(v6),
                })
            })
            .map(|(prefix, ip, _)| (prefix, ip))
            .collect();

        Ok((filtered_ipv4, filtered_ipv6))
    }

    fn get_address_ip_list(&self) -> anyhow::Result<IpList> {
        let mut ip_list = IpList {
            item_type: 1,
            data: vec![],
        };
        let addrs = self.address.to_socket_addrs()?.collect::<Vec<_>>();
        if addrs.is_empty() {
            return Err(anyhow::anyhow!(
                "Failed to read the IP address of the configuration center server."
            ));
        }
        for addr in addrs {
            match addr.ip() {
                IpAddr::V4(ipv4_addr) => {
                    ip_list.data.push(IpItem {
                        prefix_len: 32,
                        data: ipv4_addr.to_string(),
                    });
                }
                IpAddr::V6(ipv6_addr) => {
                    ip_list.data.push(IpItem {
                        prefix_len: 128,
                        data: ipv6_addr.to_string(),
                    });
                }
            }
        }
        Ok(ip_list)
    }
}
