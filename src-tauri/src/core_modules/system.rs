use crate::models::types::SystemStatus;
use std::sync::Mutex;
use std::time::Instant;
use sysinfo::{Networks, System};

struct NetworkSnapshot {
    tx_bytes: u64,
    rx_bytes: u64,
    timestamp: Instant,
}

static PREV_NETWORK: Mutex<Option<NetworkSnapshot>> = Mutex::new(None);

/// Gather current system status: CPU, RAM, uptime, network I/O, username.
/// Network rates (upload/download) are computed as bytes/sec since the last call.
pub fn get_system_status() -> SystemStatus {
    let mut sys = System::new_all();
    sys.refresh_all();
    std::thread::sleep(std::time::Duration::from_millis(200));
    sys.refresh_all();

    // CPU
    let cpu_usage = sys.global_cpu_usage();
    let cpu_cores = sys.cpus().len();

    // RAM
    let ram_total = sys.total_memory() as f64 / 1_073_741_824.0;
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

    // Compute upload/download rates (bytes/sec) from delta
    let now = Instant::now();
    let (upload_rate, download_rate) = {
        let prev = PREV_NETWORK.lock().unwrap();
        if let Some(ref snap) = *prev {
            let elapsed = now.duration_since(snap.timestamp).as_secs_f64();
            if elapsed > 0.01 {
                let tx_delta = total_tx.saturating_sub(snap.tx_bytes);
                let rx_delta = total_rx.saturating_sub(snap.rx_bytes);
                (tx_delta as f64 / elapsed, rx_delta as f64 / elapsed)
            } else {
                (0.0, 0.0)
            }
        } else {
            (0.0, 0.0)
        }
    };

    // Store current snapshot for next call
    {
        let mut prev = PREV_NETWORK.lock().unwrap();
        *prev = Some(NetworkSnapshot {
            tx_bytes: total_tx,
            rx_bytes: total_rx,
            timestamp: now,
        });
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
        network_upload_rate: (upload_rate * 100.0).round() / 100.0,
        network_download_rate: (download_rate * 100.0).round() / 100.0,
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
