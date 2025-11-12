use aya_ebpf::{
    bindings::xdp_action::{XDP_DROP, XDP_PASS}, programs::XdpContext
};
use network_types::{
    eth::{EthHdr, EtherType},
    ip::{IpProto, Ipv4Hdr, Ipv6Hdr},
};

mod ipv4;
mod ipv6;


pub fn try_xdp(ctx: XdpContext) -> Result<u32, i64> {
    let eth_hdr: *const EthHdr = unsafe { ptr_at(&ctx, 0) }?;
    let ether_type = EtherType::try_from(unsafe { *eth_hdr }.ether_type)?;
    match ether_type {
        EtherType::Ipv4 => {
            let ipv4hdr: *const Ipv4Hdr = unsafe { ptr_at(&ctx, EthHdr::LEN)? };
            match unsafe { *ipv4hdr }.proto {
                IpProto::Icmp => ipv4::handle_xdp(ctx, ipv4hdr),
                IpProto::Tcp => ipv4::handle_xdp(ctx, ipv4hdr),
                IpProto::Udp => ipv4::handle_xdp(ctx, ipv4hdr),
                _ => Ok(XDP_PASS),
            }
        }
        EtherType::Ipv6 => {
            let ipv6hdr: *const Ipv6Hdr = unsafe { ptr_at(&ctx, EthHdr::LEN)? };
            match unsafe { *ipv6hdr }.next_hdr {
                IpProto::Icmp => ipv6::handle_xdp(ctx, ipv6hdr),
                IpProto::Tcp => ipv6::handle_xdp(ctx, ipv6hdr),
                IpProto::Udp => ipv6::handle_xdp(ctx, ipv6hdr),
                _ => Ok(XDP_PASS),
            }
        }
        _ => Ok(XDP_PASS),
    }
}

#[inline(always)]
unsafe fn ptr_at<T>(ctx: &XdpContext, offset: usize) -> Result<*const T, i64> {
    let start = ctx.data();
    let end = ctx.data_end();
    let len = core::mem::size_of::<T>();

    if start + offset + len > end {
        return Err(XDP_DROP.into());
    }

    let ptr = (start + offset) as *const T;
    Ok(unsafe { &*ptr })
}
