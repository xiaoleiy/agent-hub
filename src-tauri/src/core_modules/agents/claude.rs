use crate::models::types::{AgentInfo, AgentType, Session, UsageStats};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

fn home() -> PathBuf {
    dirs_next::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

fn claude_dir() -> PathBuf {
    home().join(".claude")
}

fn sessions_dir() -> PathBuf {
    claude_dir().join("sessions")
}

/// Detect Claude Code installation and running state
pub fn detect() -> AgentInfo {
    let claude_bin = home().join(".local/bin/claude");
    let installed = claude_bin.exists() || which_claude();
    let sessions = sessions_dir();
    let running = if sessions.exists() {
        // Check if any session file has a live PID
        if let Ok(entries) = fs::read_dir(&sessions) {
            entries.filter_map(|e| e.ok()).any(|e| {
                if let Ok(data) = fs::read_to_string(e.path()) {
                    if let Ok(sess) = serde_json::from_str::<ClaudeSession>(&data) {
                        is_pid_alive(sess.pid)
                    } else {
                        false
                    }
                } else {
                    false
                }
            })
        } else {
            false
        }
    } else {
        false
    };

    let active_sessions = if sessions.exists() {
        fs::read_dir(&sessions)
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .filter(|e| {
                        fs::read_to_string(e.path())
                            .ok()
                            .and_then(|d| serde_json::from_str::<ClaudeSession>(&d).ok())
                            .map(|s| is_pid_alive(s.pid))
                            .unwrap_or(false)
                    })
                    .count()
            })
            .unwrap_or(0)
    } else {
        0
    };

    // Try to get version from session files
    let version = sessions_dir()
        .read_dir()
        .ok()
        .and_then(|mut entries| entries.next()?.ok())
        .and_then(|e| fs::read_to_string(e.path()).ok())
        .and_then(|d| serde_json::from_str::<ClaudeSession>(&d).ok())
        .map(|s| s.version);

    AgentInfo {
        name: "Claude Code".to_string(),
        agent_type: AgentType::ClaudeCode,
        installed,
        running,
        active_sessions,
        version,
        install_path: if installed {
            Some(home().join(".local/bin/claude").to_string_lossy().to_string())
        } else {
            None
        },
    }
}

fn which_claude() -> bool {
    std::process::Command::new("which")
        .arg("claude")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn is_pid_alive(pid: u32) -> bool {
    // kill -0 checks if process exists without sending a signal
    std::process::Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[derive(Deserialize)]
struct ClaudeSession {
    pid: u32,
    #[serde(default)]
    session_id: String,
    #[serde(default)]
    cwd: String,
    #[serde(default)]
    started_at: Option<u64>,
    #[serde(default)]
    version: String,
    #[serde(default)]
    status: String,
}

/// Get active Claude Code sessions
pub fn get_sessions() -> Vec<Session> {
    let dir = sessions_dir();
    if !dir.exists() {
        return vec![];
    }

    fs::read_dir(&dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter_map(|e| {
                    let data = fs::read_to_string(e.path()).ok()?;
                    let sess: ClaudeSession = serde_json::from_str(&data).ok()?;
                    if !is_pid_alive(sess.pid) {
                        return None;
                    }
                    let started = sess.started_at.map(|ts| {
                        let dt: DateTime<Utc> =
                            DateTime::from_timestamp_millis(ts as i64).unwrap_or_default();
                        dt.to_rfc3339()
                    });
                    Some(Session {
                        id: sess.session_id,
                        agent: "Claude Code".to_string(),
                        status: sess.status,
                        started_at: started,
                        working_dir: Some(sess.cwd),
                        model: None,
                        pid: Some(sess.pid),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Get usage stats by parsing history.jsonl
pub fn get_usage(window: &str) -> UsageStats {
    let history_path = claude_dir().join("history.jsonl");
    let window_duration = parse_window(window);
    let cutoff = Utc::now() - window_duration;

    let mut total_interactions = 0usize;
    let mut sessions_set = std::collections::HashSet::new();
    let mut first_activity: Option<DateTime<Utc>> = None;
    let mut last_activity: Option<DateTime<Utc>> = None;

    if let Ok(content) = fs::read_to_string(&history_path) {
        for line in content.lines() {
            if let Ok(entry) = serde_json::from_str::<HistoryEntry>(line) {
                let ts = DateTime::from_timestamp_millis(entry.timestamp).unwrap_or_default();
                if ts >= cutoff {
                    total_interactions += 1;
                    sessions_set.insert(entry.session_id.clone());
                    first_activity = Some(first_activity.map_or(ts, |f: DateTime<Utc>| f.min(ts)));
                    last_activity = Some(last_activity.map_or(ts, |l: DateTime<Utc>| l.max(ts)));
                }
            }
        }
    }

    UsageStats {
        agent: "Claude Code".to_string(),
        window: window.to_string(),
        total_sessions: sessions_set.len(),
        total_interactions,
        first_activity: first_activity.map(|d| d.to_rfc3339()),
        last_activity: last_activity.map(|d| d.to_rfc3339()),
    }
}

#[derive(Deserialize)]
struct HistoryEntry {
    timestamp: i64,
    #[serde(default)]
    session_id: String,
}

fn parse_window(window: &str) -> chrono::Duration {
    match window {
        "5h" => chrono::Duration::hours(5),
        "1w" => chrono::Duration::weeks(1),
        "1m" => chrono::Duration::days(30),
        _ => chrono::Duration::hours(5),
    }
}
