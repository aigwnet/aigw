use core::net::Ipv6Addr;

use aya_ebpf::{
    bindings::xdp_action::{XDP_DROP, XDP_PASS},
    maps::lpm_trie::Key,
    programs::XdpContext,
};
use aya_log_ebpf::info;
use network_types::ip::Ipv6Hdr;

use crate::{BLOCKLIST_IPV6_CIDR, SWITCH, WHITELIST_IPV6_CIDR};

pub fn handle_xdp(ctx: XdpContext, ip_hdr: *const Ipv6Hdr) -> Result<u32, i64> {
    let dst_addr = u128::from_be_bytes(unsafe { (*ip_hdr).dst_addr });
    let src_addr = u128::from_be_bytes(unsafe { (*ip_hdr).src_addr });

    //
    let action = unsafe {
        let key = Key::<[u8; 16]> {
            prefix_len: 64,
            data: (*ip_hdr).src_addr,
        };
        // use white list
        if SWITCH.get(&1).is_some() {
            if WHITELIST_IPV6_CIDR.get(key).is_some() {
                XDP_PASS
            } else {
                XDP_DROP
            }
        }
        // use black list
        else if SWITCH.get(&2).is_some() {
            if BLOCKLIST_IPV6_CIDR.get(key).is_some() {
                XDP_DROP
            } else {
                XDP_PASS
            }
        } else {
            XDP_PASS
        }
    };

    info!(
        &ctx,
        "received packet, SRC: {} -->DEST: {}, ACTION: {}",
        Ipv6Addr::from_bits(src_addr),
        Ipv6Addr::from_bits(dst_addr),
        action
    );

    Ok(action)
}
