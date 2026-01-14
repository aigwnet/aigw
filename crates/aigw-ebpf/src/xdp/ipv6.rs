use aya_ebpf::{
    bindings::xdp_action::{XDP_DROP, XDP_PASS},
    maps::lpm_trie::Key,
    programs::XdpContext,
};
use aya_log_ebpf::info;
use core::net::Ipv6Addr;
use network_types::ip::Ipv6Hdr;

use crate::{BLOCKLIST_IPV6_CIDR, SWITCH, WHITELIST_IPV6, WHITELIST_IPV6_CIDR};

pub fn handle_xdp(ctx: XdpContext, ip_hdr: *const Ipv6Hdr) -> Result<u32, i64> {
    let dst_addr = u128::from_be_bytes(unsafe { (*ip_hdr).dst_addr });
    let src_addr = u128::from_be_bytes(unsafe { (*ip_hdr).src_addr });

    //
    let action = unsafe {
        if WHITELIST_IPV6.get(&src_addr).is_some() || WHITELIST_IPV6.get(&dst_addr).is_some() {
            XDP_PASS
        } else {
            let key = Key::<[u8; 16]> {
                prefix_len: 128,
                data: (*ip_hdr).src_addr,
            };

            let mode = SWITCH.get(&0).copied().unwrap_or(0);
            match mode {
                1 => {
                    if WHITELIST_IPV6_CIDR.get(key).is_some() {
                        XDP_PASS
                    } else {
                        info!(
                            &ctx,
                            "not in whitelist (CIDR), dropping: {} -> {}.",
                            Ipv6Addr::from_bits(src_addr),
                            Ipv6Addr::from_bits(dst_addr),
                        );
                        XDP_DROP
                    }
                }
                2 => {
                    if BLOCKLIST_IPV6_CIDR.get(key).is_some() {
                        info!(
                            &ctx,
                            "in blacklist (CIDR), dropping: {} -> {}.",
                            Ipv6Addr::from_bits(src_addr),
                            Ipv6Addr::from_bits(dst_addr),
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
