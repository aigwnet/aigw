use std::time::Duration;

use sysinfo::{Disks, Networks, System};

use crate::Statistics;

pub async fn statistics(
    tls: u64,
    pv: u64,
    rt: u64,
    error: u64,
    ext_info: String,
) -> anyhow::Result<Statistics> {
    let mut sys = System::new_all();

    sys.refresh_all();

    let mut io_read_0 = 0;
    let mut io_written_0 = 0;
    let disks = Disks::new_with_refreshed_list();
    for disk in &disks {
        io_read_0 += disk.usage().read_bytes;
        io_written_0 += disk.usage().written_bytes;
    }
    
    let mut networks = Networks::new_with_refreshed_list();
    tokio::time::sleep(Duration::from_millis(1000)).await;
    networks.refresh(true);

    let mut net_send = 0;
    let mut net_received = 0;
    for (_interface_name, data) in &networks {
        net_send += data.transmitted();
        net_received += data.received()
    }

    sys.refresh_all();
    let mut cpu_current_process = 0.0;
    let current_pid = std::process::id();

    for (pid, p) in sys.processes() {
        if pid.as_u32() == current_pid {
            cpu_current_process = p.cpu_usage() / sys.cpus().len() as f32;
            break;
        }
    }
    let global_cpu_usage = sys.global_cpu_usage() as f64;

    let mut disk_total = 0;
    let mut dist_free = 0;

    let mut io_read_1 = 0;
    let mut io_written_1 = 0;

    let disks = Disks::new_with_refreshed_list();
    for disk in &disks {
        disk_total += disk.total_space();
        dist_free += disk.available_space();

        io_read_1 += disk.usage().read_bytes;
        io_written_1 += disk.usage().written_bytes;
    }

    let io_read = io_read_1 - io_read_0;
    let io_written = io_written_1 - io_written_0;

    let load_avg = System::load_average();

    let s = Statistics {
        uptime: System::uptime(),
        cpu: global_cpu_usage,
        cpu_current_process: cpu_current_process as f64,
        cpu_load_one: load_avg.one,
        cpu_load_five: load_avg.five,
        cpu_load_fifteen: load_avg.fifteen,
        mem_used: sys.used_memory(),
        mem_free: sys.free_memory(),
        swap_used: sys.used_swap(),
        swap_free: sys.free_swap(),
        disk_used: dist_free,
        disk_free: disk_total - dist_free,
        io_read,
        io_written,
        net_send,
        net_received,
        tls,
        pv,
        rt,
        error,
        ext_info,
    };

    Ok(s)
}
