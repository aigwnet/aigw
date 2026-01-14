use aya_ebpf::{
    bindings::xdp_action::{XDP_DROP, XDP_PASS},
    maps::lpm_trie::Key,
    programs::XdpContext,
};
use aya_log_ebpf::info;
use core::net::Ipv4Addr;
use network_types::ip::Ipv4Hdr;

use crate::{BLOCKLIST_IPV4_CIDR, SWITCH, WHITELIST_IPV4, WHITELIST_IPV4_CIDR};

pub fn handle_xdp(ctx: XdpContext, ip_hdr: *const Ipv4Hdr) -> Result<u32, i64> {
    //
    let src_addr = u32::from_be_bytes(unsafe { (*ip_hdr).src_addr });
    let dst_addr = u32::from_be_bytes(unsafe { (*ip_hdr).dst_addr });

    let action = unsafe {
        if WHITELIST_IPV4.get(&src_addr).is_some() || WHITELIST_IPV4.get(&dst_addr).is_some() {
            XDP_PASS
        } else {
            let key = Key::<[u8; 4]> {
                prefix_len: 32,
                data: (*ip_hdr).src_addr,
            };

            let mode = SWITCH.get(&0).copied().unwrap_or(0);
            match mode {
                1 => {
                    if WHITELIST_IPV4_CIDR.get(key).is_some() {
                        XDP_PASS
                    } else {
                        info!(
                            &ctx,
                            "not in whitelist (CIDR), dropping: {} -> {}.",
                            Ipv4Addr::from_bits(src_addr),
                            Ipv4Addr::from_bits(dst_addr),
                        );
                        XDP_DROP
                    }
                }
                2 => {
                    if BLOCKLIST_IPV4_CIDR.get(key).is_some() {
                        info!(
                            &ctx,
                            "in blacklist (CIDR), dropping: {} -> {}.",
                            Ipv4Addr::from_bits(src_addr),
                            Ipv4Addr::from_bits(dst_addr),
                        );
                        XDP_DROP
                    } else {
                        XDP_PASS
                    }
                }
                _ => XDP_PASS,
            }
        }
    };
    Ok(action)
}
