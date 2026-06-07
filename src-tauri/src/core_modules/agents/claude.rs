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

    let mut cli_sessions = 0usize;
    let mut gui_sessions = 0usize;
    let mut running = false;
    let mut version: Option<String> = None;

    if sessions.exists() {
        if let Ok(entries) = fs::read_dir(&sessions) {
            for entry in entries.filter_map(|e| e.ok()) {
                if let Ok(data) = fs::read_to_string(entry.path()) {
                    if let Ok(sess) = serde_json::from_str::<ClaudeSession>(&data) {
                        if is_pid_alive(sess.pid) {
                            running = true;
                            match sess.entrypoint.as_str() {
                                "cli" | "sdk-cli" => cli_sessions += 1,
                                _ => gui_sessions += 1,
                            }
                        }
                        if version.is_none() && !sess.version.is_empty() {
                            version = Some(sess.version.clone());
                        }
                    }
                }
            }
        }
    }

    AgentInfo {
        name: "Claude Code".to_string(),
        agent_type: AgentType::ClaudeCode,
        installed,
        running,
        active_sessions: cli_sessions + gui_sessions,
        cli_sessions,
        gui_sessions,
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
    #[serde(default)]
    entrypoint: String,
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
                    let ep = if sess.entrypoint.is_empty() {
                        "cli".to_string()
                    } else {
                        sess.entrypoint.clone()
                    };
                    Some(Session {
                        id: sess.session_id,
                        agent: "Claude Code".to_string(),
                        status: sess.status,
                        started_at: started,
                        working_dir: Some(sess.cwd),
                        model: None,
                        pid: Some(sess.pid),
                        entrypoint: ep,
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
