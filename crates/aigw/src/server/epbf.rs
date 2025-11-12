use std::{net::IpAddr, sync::Mutex};

use aigw_core::{IpDeleteList, IpUpdateList};
use anyhow::Context as _;
use aya::{
    Ebpf,
    maps::{LpmTrie, lpm_trie::Key},
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
    program.attach(&iface, XdpFlags::default())
        .context("failed to attach the XDP program with default flags - try changing XdpFlags::default() to XdpFlags::SKB_MODE")?;

    Ok(EbpfHandler {
        ebpf: Mutex::new(ebpf),
    })
}

pub struct EbpfHandler {
    ebpf: Mutex<Ebpf>,
}

impl EbpfHandler {
    pub fn handle_update(&self, ip_list: IpUpdateList) -> anyhow::Result<()> {
        let mut ebpf = self
            .ebpf
            .lock()
            .map_err(|_e| anyhow::anyhow!("ebpf get lock error."))?;

        if ip_list.item_type == 1 {
            let mut ipv4 = vec![];
            let mut ipv6 = vec![];
            for item in ip_list.data {
                let ip_addr: IpAddr = item.data.parse()?;
                match ip_addr {
                    IpAddr::V4(ipv4_addr) => {
                        ipv4.push((item.prefix_len, ipv4_addr));
                    }
                    IpAddr::V6(ipv6_addr) => {
                        ipv6.push((item.prefix_len, ipv6_addr));
                    }
                }
            }

            {
                let mut white_ipv4_map: LpmTrie<_, [u8; 4], u32> =
                    LpmTrie::try_from(ebpf.map_mut("WHITELIST_IPV4_CIDR").unwrap())?;
                for (prefix_len, ipv4_addr) in ipv4 {
                    let key = Key::new(prefix_len, ipv4_addr.octets());
                    white_ipv4_map.insert(&key, 1, 0)?;
                }
            }

            {
                let mut white_ipv6_map: LpmTrie<_, [u8; 16], u32> =
                    LpmTrie::try_from(ebpf.map_mut("WHITELIST_IPV6_CIDR").unwrap())?;
                for (prefix_len, ipv6_addr) in ipv6 {
                    let key = Key::new(prefix_len, ipv6_addr.octets());
                    white_ipv6_map.insert(&key, 1, 0)?;
                }
            }
        } else if ip_list.item_type == 2 {
            let mut ipv4 = vec![];
            let mut ipv6 = vec![];
            for item in ip_list.data {
                let ip_addr: IpAddr = item.data.parse()?;
                match ip_addr {
                    IpAddr::V4(ipv4_addr) => {
                        ipv4.push((item.prefix_len, ipv4_addr));
                    }
                    IpAddr::V6(ipv6_addr) => {
                        ipv6.push((item.prefix_len, ipv6_addr));
                    }
                }
            }

            {
                let mut block_ipv4_map: LpmTrie<_, [u8; 4], u32> =
                    LpmTrie::try_from(ebpf.map_mut("BLOCKLIST_IPV4_CIDR").unwrap())?;
                for (prefix_len, ipv4_addr) in ipv4 {
                    let key = Key::new(prefix_len, ipv4_addr.octets());
                    block_ipv4_map.insert(&key, 1, 0)?;
                }
            }

            {
                let mut block_ipv6_map: LpmTrie<_, [u8; 16], u32> =
                    LpmTrie::try_from(ebpf.map_mut("BLOCKLIST_IPV6_CIDR").unwrap())?;
                for (prefix_len, ipv6_addr) in ipv6 {
                    let key = Key::new(prefix_len, ipv6_addr.octets());
                    block_ipv6_map.insert(&key, 1, 0)?;
                }
            }
        }

        Ok(())
    }

    pub fn handle_delete(&self, ip_list: IpDeleteList) -> anyhow::Result<()> {
        let mut ebpf = self
            .ebpf
            .lock()
            .map_err(|_e| anyhow::anyhow!("ebpf get lock error."))?;

        if ip_list.item_type == 1 {
            let mut ipv4 = vec![];
            let mut ipv6 = vec![];
            for item in ip_list.data {
                let ip_addr: IpAddr = item.data.parse()?;
                match ip_addr {
                    IpAddr::V4(ipv4_addr) => {
                        ipv4.push((item.prefix_len, ipv4_addr));
                    }
                    IpAddr::V6(ipv6_addr) => {
                        ipv6.push((item.prefix_len, ipv6_addr));
                    }
                }
            }

            {
                let mut white_ipv4_map: LpmTrie<_, [u8; 4], u32> =
                    LpmTrie::try_from(ebpf.map_mut("WHITELIST_IPV4_CIDR").unwrap())?;
                for (prefix_len, ipv4_addr) in ipv4 {
                    let key = Key::new(prefix_len, ipv4_addr.octets());
                    white_ipv4_map.remove(&key)?;
                }
            }

            {
                let mut white_ipv6_map: LpmTrie<_, [u8; 16], u32> =
                    LpmTrie::try_from(ebpf.map_mut("WHITELIST_IPV6_CIDR").unwrap())?;
                for (prefix_len, ipv6_addr) in ipv6 {
                    let key = Key::new(prefix_len, ipv6_addr.octets());
                    white_ipv6_map.remove(&key)?;
                }
            }
        } else if ip_list.item_type == 2 {
            let mut ipv4 = vec![];
            let mut ipv6 = vec![];
            for item in ip_list.data {
                let ip_addr: IpAddr = item.data.parse()?;
                match ip_addr {
                    IpAddr::V4(ipv4_addr) => {
                        ipv4.push((item.prefix_len, ipv4_addr));
                    }
                    IpAddr::V6(ipv6_addr) => {
                        ipv6.push((item.prefix_len, ipv6_addr));
                    }
                }
            }

            {
                let mut block_ipv4_map: LpmTrie<_, [u8; 4], u32> =
                    LpmTrie::try_from(ebpf.map_mut("BLOCKLIST_IPV4_CIDR").unwrap())?;
                for (prefix_len, ipv4_addr) in ipv4 {
                    let key = Key::new(prefix_len, ipv4_addr.octets());
                    block_ipv4_map.remove(&key)?;
                }
            }

            {
                let mut block_ipv6_map: LpmTrie<_, [u8; 16], u32> =
                    LpmTrie::try_from(ebpf.map_mut("BLOCKLIST_IPV6_CIDR").unwrap())?;
                for (prefix_len, ipv6_addr) in ipv6 {
                    let key = Key::new(prefix_len, ipv6_addr.octets());
                    block_ipv6_map.remove(&key)?;
                }
            }
        }

        Ok(())
    }
}
