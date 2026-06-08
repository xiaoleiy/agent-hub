use crate::models::types::KeepAliveStatus;
use std::fs;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

/// State persisted to disk for keep-alive
#[derive(Debug, Clone, Serialize, Deserialize)]
struct KeepAliveState {
    active: bool,
    mode: String,
    started_at: String,
    expires_at: Option<String>,
    pid: Option<u32>,
}

static KEEPALIVE_CHILD: Mutex<Option<Child>> = Mutex::new(None);

fn state_path() -> PathBuf {
    let home = dirs().unwrap_or_else(|| PathBuf::from("."));
    home.join(".agent-hub").join("keepalive.json")
}

fn dirs() -> Option<PathBuf> {
    dirs_next::home_dir()
}

fn load_state() -> KeepAliveState {
    let path = state_path();
    if let Ok(data) = fs::read_to_string(&path) {
        if let Ok(state) = serde_json::from_str::<KeepAliveState>(&data) {
            return state;
        }
    }
    KeepAliveState {
        active: false,
        mode: "off".to_string(),
        started_at: String::new(),
        expires_at: None,
        pid: None,
    }
}

fn save_state(state: &KeepAliveState) {
    let path = state_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(state) {
        let _ = fs::write(&path, json);
    }
}

/// Start keep-alive with the given duration mode.
/// Modes: "30m", "1h", "3h", "forever"
pub fn start_keepalive(mode: &str) -> Result<(), String> {
    // Stop any existing keep-alive first
    stop_keepalive_internal();

    let duration_flag = match mode {
        "30m" => Some("1800"),
        "1h" => Some("3600"),
        "3h" => Some("10800"),
        "forever" => None,
        _ => return Err(format!("Invalid keep-alive mode: {}", mode)),
    };

    let mut cmd = Command::new("caffeinate");
    cmd.arg("-i"); // prevent idle sleep

    if let Some(secs) = duration_flag {
        cmd.arg("-t").arg(secs);
    }

    let child = cmd
        .spawn()
        .map_err(|e| format!("Failed to start caffeinate: {}", e))?;

    let pid = child.id();
    let now = chrono::Utc::now().to_rfc3339();
    let expires = if mode == "forever" {
        None
    } else {
        // Compute approximate expiry
        let secs: u64 = duration_flag.unwrap_or("0").parse().unwrap_or(0);
        let expiry = chrono::Utc::now() + chrono::Duration::seconds(secs as i64);
        Some(expiry.to_rfc3339())
    };

    let state = KeepAliveState {
        active: true,
        mode: mode.to_string(),
        started_at: now,
        expires_at: expires,
        pid: Some(pid),
    };
    save_state(&state);

    *KEEPALIVE_CHILD.lock().unwrap() = Some(child);

    Ok(())
}

/// Stop the current keep-alive process
pub fn stop_keepalive() -> Result<(), String> {
    stop_keepalive_internal();
    let state = KeepAliveState {
        active: false,
        mode: "off".to_string(),
        started_at: String::new(),
        expires_at: None,
        pid: None,
    };
    save_state(&state);
    Ok(())
}

fn stop_keepalive_internal() {
    // Kill tracked child process
    if let Ok(mut guard) = KEEPALIVE_CHILD.lock() {
        if let Some(ref mut child) = *guard {
            let _ = child.kill();
        }
        *guard = None;
    }

    // Also check persisted state for orphaned caffeinate processes
    let state = load_state();
    if let Some(pid) = state.pid {
        // Only kill if the PID is still actually caffeinate — PIDs get recycled,
        // so a stale stored PID could otherwise point at an unrelated process.
        if is_caffeinate(pid) {
            let _ = Command::new("kill").arg(pid.to_string()).output();
        }
    }
}

/// True if `pid` is alive and its process name is caffeinate.
fn is_caffeinate(pid: u32) -> bool {
    Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "comm="])
        .output()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .trim()
                .ends_with("caffeinate")
        })
        .unwrap_or(false)
}

/// Get current keep-alive status
pub fn get_keepalive_status() -> KeepAliveStatus {
    let state = load_state();

    // Verify the process is actually still running
    if state.active {
        if let Some(pid) = state.pid {
            let alive = Command::new("kill")
                .arg("-0")
                .arg(pid.to_string())
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);

            if !alive {
                // Process died, clean up state
                let clean = KeepAliveState {
                    active: false,
                    mode: "off".to_string(),
                    started_at: String::new(),
                    expires_at: None,
                    pid: None,
                };
                save_state(&clean);
                return KeepAliveStatus {
                    active: false,
                    mode: None,
                    started_at: None,
                    expires_at: None,
                    pid: None,
                };
            }
        }
    }

    KeepAliveStatus {
        active: state.active,
        mode: if state.active { Some(state.mode) } else { None },
        started_at: if state.active {
            Some(state.started_at)
        } else {
            None
        },
        expires_at: state.expires_at,
        pid: state.pid,
    }
}
