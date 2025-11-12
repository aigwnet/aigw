use aya_ebpf::{
    bindings::xdp_action::{XDP_DROP, XDP_PASS},
    maps::lpm_trie::Key,
    programs::XdpContext,
};
use aya_log_ebpf::info;
use network_types::ip::Ipv4Hdr;

use crate::{BLOCKLIST_IPV4_CIDR, SWITCH, WHITELIST_IPV4_CIDR};

pub fn handle_xdp(ctx: XdpContext, ip_hdr: *const Ipv4Hdr) -> Result<u32, i64> {
    let dst_addr = u32::from_be_bytes(unsafe { (*ip_hdr).dst_addr });
    let src_addr = u32::from_be_bytes(unsafe { (*ip_hdr).src_addr });

    //
    let action = unsafe {
        let key = Key::<[u8; 4]> {
            prefix_len: 32,
            data: (*ip_hdr).src_addr,
        };
        // use white list
        if SWITCH.get(&1).is_some() {
            if WHITELIST_IPV4_CIDR.get(key).is_some() {
                XDP_PASS
            } else {
                XDP_DROP
            }
        }
        // use black list
        else if SWITCH.get(&2).is_some() {
            if BLOCKLIST_IPV4_CIDR.get(key).is_some() {
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
        "received packet, SRC: {:i} -->DEST: {:i}, ACTION: {}", src_addr, dst_addr, action
    );

    Ok(action)
}
