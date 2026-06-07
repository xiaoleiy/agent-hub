use crate::models::types::SystemStatus;
use sysinfo::{Networks, System};

/// Gather current system status: CPU, RAM, uptime, network I/O, username.
pub fn get_system_status() -> SystemStatus {
    let mut sys = System::new_all();
    // Need two refreshes separated by a short sleep to get CPU usage
    sys.refresh_all();
    std::thread::sleep(std::time::Duration::from_millis(200));
    sys.refresh_all();

    // CPU usage (average across all cores)
    let cpu_usage = sys.global_cpu_usage();
    let cpu_cores = sys.cpus().len();

    // RAM
    let ram_total = sys.total_memory() as f64 / 1_073_741_824.0; // bytes → GB
    let ram_used = sys.used_memory() as f64 / 1_073_741_824.0;
    let ram_percent = if ram_total > 0.0 {
        (ram_used / ram_total) * 100.0
    } else {
        0.0
    };

    // Uptime
    let uptime_secs = System::uptime();
    let uptime_formatted = format_uptime(uptime_secs);

    // Username
    let username = whoami::username();

    // Network I/O (aggregate all interfaces)
    let networks = Networks::new_with_refreshed_list();
    let mut total_tx: u64 = 0;
    let mut total_rx: u64 = 0;
    for (_name, data) in networks.iter() {
        total_tx += data.total_transmitted();
        total_rx += data.total_received();
    }

    SystemStatus {
        cpu_usage,
        cpu_cores,
        ram_total_gb: (ram_total * 100.0).round() / 100.0,
        ram_used_gb: (ram_used * 100.0).round() / 100.0,
        ram_usage_percent: (ram_percent * 100.0).round() / 100.0,
        uptime_seconds: uptime_secs,
        uptime_formatted,
        username,
        network_upload_bytes: total_tx,
        network_download_bytes: total_rx,
        network_upload_rate: 0.0,  // rate computed by frontend via polling
        network_download_rate: 0.0,
    }
}

fn format_uptime(secs: u64) -> String {
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    let minutes = (secs % 3600) / 60;

    if days > 0 {
        format!("{}d {}h {}m", days, hours, minutes)
    } else if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else {
        format!("{}m", minutes)
    }
}
