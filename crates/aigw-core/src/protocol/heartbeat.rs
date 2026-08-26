use crate::{LogPoint, Statistics, protocol::pb};

#[derive(Clone, Debug)]
pub struct Ping {
    pub ts: i64,
    pub log_points: Vec<LogPoint>,
    pub statistics: Statistics,
}

#[derive(Clone, Debug)]
pub struct Pong {
    pub ts: i64,
}

impl TryFrom<pb::Ping> for Ping {
    type Error = anyhow::Error;

    fn try_from(value: pb::Ping) -> Result<Self, Self::Error> {
        Ok(Ping {
            ts: value.ts,
            log_points: value
                .log_points
                .into_iter()
                .map(LogPoint::try_from)
                .collect::<Result<_, _>>()?,
            statistics: Statistics {
                uptime: value.uptime,
                cpu: value.cpu,
                cpu_current_process: value.cpu_current_process,
                cpu_load_one: value.cpu_load_one,
                cpu_load_five: value.cpu_load_five,
                cpu_load_fifteen: value.cpu_load_fifteen,
                mem_used: value.mem_used,
                mem_free: value.mem_free,
                swap_used: value.swap_used,
                swap_free: value.swap_free,
                disk_used: value.disk_used,
                disk_free: value.disk_free,
                io_read: value.io_read,
                io_written: value.io_written,
                net_send: value.net_send,
                net_received: value.net_received,
                tls: value.tls,
                pv: value.pv,
                rt: value.rt,
                error: value.error,
                ext_info: value.ext_info,
            },
        })
    }
}
