#![no_std]
#![no_main]

use aya_ebpf::{
    bindings::xdp_action::XDP_ABORTED,
    macros::{map, xdp},
    maps::{HashMap, LpmTrie},
    programs::XdpContext,
};

mod xdp;

#[map]
static SWITCH: HashMap<u32, u32> = HashMap::<u32, u32>::with_max_entries(16, 0);
#[map]
static BLOCKLIST_IPV4_CIDR: LpmTrie<[u8; 4], u32> = LpmTrie::with_max_entries(65536, 0);
#[map]
static WHITELIST_IPV4_CIDR: LpmTrie<[u8; 4], u32> = LpmTrie::with_max_entries(65536, 0);
#[map]
static BLOCKLIST_IPV6_CIDR: LpmTrie<[u8; 16], u32> = LpmTrie::with_max_entries(65536, 0);
#[map]
static WHITELIST_IPV6_CIDR: LpmTrie<[u8; 16], u32> = LpmTrie::with_max_entries(65536, 0);
#[map]
static WHITELIST_IPV4: HashMap<u32, u32> = HashMap::<u32, u32>::with_max_entries(64, 0);
#[map]
static WHITELIST_IPV6: HashMap<u128, u32> = HashMap::<u128, u32>::with_max_entries(16, 0);
#[xdp]
pub fn aigw(ctx: XdpContext) -> u32 {
    match xdp::try_xdp(ctx) {
        Ok(ret) => ret,
        Err(_e) => XDP_ABORTED,
    }
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[unsafe(link_section = "license")]
#[unsafe(no_mangle)]
static LICENSE: [u8; 13] = *b"Dual MIT/GPL\0";
