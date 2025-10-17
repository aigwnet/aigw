use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Statistics {
    pub uptime: u64,
    pub cpu: f64,
    pub cpu_current_process: f64,
    /// Average load within one minute.
    pub cpu_load_one: f64,
    /// Average load within five minutes.
    pub cpu_load_five: f64,
    /// Average load within fifteen minutes.
    pub cpu_load_fifteen: f64,
    pub mem_used: u64,
    pub mem_free: u64,
    pub swap_used: u64,
    pub swap_free: u64,
    pub disk_used: u64,
    pub disk_free: u64,
    pub io_read: u64,
    pub io_written: u64,
    pub net_send: u64,
    pub net_received: u64,
    pub tls: u64,
    pub pv: u64,
    pub rt: u64,
    pub error: u64,
    pub ext_info: String,
}
