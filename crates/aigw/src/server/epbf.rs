use std::{net::IpAddr, sync::Mutex};

use aigw_core::{IpDeleteList, IpUpdateList};
use aya::{
    Ebpf,
    maps::{HashMap, LpmTrie, lpm_trie::Key},
    programs::{Xdp, XdpFlags},
};
use tracing::{debug, warn};

pub fn run(iface: &str, epbf: Option<&String>) -> anyhow::Result<EbpfHandler> {
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

    let mut ebpf = if let Some(epbf) = epbf {
        aya::Ebpf::load_file(epbf)?
    } else {
        aya::Ebpf::load(aya::include_bytes_aligned!(concat!(
            env!("OUT_DIR"),
            "/aigw"
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
    if let Err(e) = program.attach(iface, XdpFlags::default()) {
        eprintln!("Native XDP attach failed ({}), falling back to SKB mode", e);
        program.attach(iface, XdpFlags::SKB_MODE)?;
    }

    Ok(EbpfHandler {
        ebpf: Mutex::new(ebpf),
    })
}

pub struct EbpfHandler {
    ebpf: Mutex<Ebpf>,
}

impl EbpfHandler {
    fn parse_ip_entries(
        &self,
        items: Vec<(u32, String)>,
    ) -> anyhow::Result<(
        Vec<(u32, std::net::Ipv4Addr)>,
        Vec<(u32, std::net::Ipv6Addr)>,
    )> {
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
            }
        }

        if !ipv6_entries.is_empty() {
            let mut map: LpmTrie<_, [u8; 16], u32> =
                LpmTrie::try_from(ebpf.map_mut(ipv6_map_name).unwrap())?;
            for &(prefix_len, addr) in ipv6_entries {
                let key = Key::new(prefix_len, addr.octets());
                map.insert(&key, 1, 0)?;
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
            }
        }

        if !ipv6_entries.is_empty() {
            let mut map: LpmTrie<_, [u8; 16], u32> =
                LpmTrie::try_from(ebpf.map_mut(ipv6_map_name).unwrap())?;
            for &(prefix_len, addr) in ipv6_entries {
                let key = Key::new(prefix_len, addr.octets());
                map.remove(&key)?;
            }
        }

        Ok(())
    }

    pub fn handle_update(&self, ip_list: IpUpdateList) -> anyhow::Result<()> {
        let mut ebpf = self
            .ebpf
            .lock()
            .map_err(|_| anyhow::anyhow!("ebpf get lock error."))?;

        let ip_data = ip_list
            .data
            .iter()
            .map(|i| (i.prefix_len, i.data.clone()))
            .collect();

        let (ipv4, ipv6) = self.parse_ip_entries(ip_data)?;

        match ip_list.item_type {
            1 => self.update_maps(
                &mut ebpf,
                &ipv4,
                &ipv6,
                "WHITELIST_IPV4_CIDR",
                "WHITELIST_IPV6_CIDR",
            ),
            2 => self.update_maps(
                &mut ebpf,
                &ipv4,
                &ipv6,
                "BLOCKLIST_IPV4_CIDR",
                "BLOCKLIST_IPV6_CIDR",
            ),
            _ => Ok(()), // ignore unsupported item types
        }
    }

    pub fn handle_delete(&self, ip_list: IpDeleteList) -> anyhow::Result<()> {
        let mut ebpf = self
            .ebpf
            .lock()
            .map_err(|_| anyhow::anyhow!("ebpf get lock error."))?;

        let ip_data = ip_list
            .data
            .iter()
            .map(|i| (i.prefix_len, i.data.clone()))
            .collect();

        let (ipv4, ipv6) = self.parse_ip_entries(ip_data)?;
        match ip_list.item_type {
            1 => self.delete_maps(
                &mut ebpf,
                &ipv4,
                &ipv6,
                "WHITELIST_IPV4_CIDR",
                "WHITELIST_IPV6_CIDR",
            ),
            2 => self.delete_maps(
                &mut ebpf,
                &ipv4,
                &ipv6,
                "BLOCKLIST_IPV4_CIDR",
                "BLOCKLIST_IPV6_CIDR",
            ),
            _ => Ok(()),
        }
    }

    pub fn handle_switch(
        &self,
        enable_white_list: bool,
        enable_block_list: bool,
    ) -> anyhow::Result<()> {
        //
        let mut ebpf = self
            .ebpf
            .lock()
            .map_err(|_| anyhow::anyhow!("ebpf get lock error."))?;

        let mut map: HashMap<_, u32, u32> = HashMap::try_from(ebpf.map_mut("SWITCH").unwrap())?;
        if enable_white_list {
            map.insert(1, 1, 0)?;
        } else {
            map.remove(&1)?;
        }

        if enable_block_list {
            map.insert(2, 1, 0)?;
        } else {
            map.remove(&2)?;
        }
        Ok(())
    }
}
