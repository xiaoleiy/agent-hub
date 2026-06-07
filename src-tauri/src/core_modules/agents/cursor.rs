use crate::models::types::{AgentInfo, AgentType, Session, UsageStats};
use chrono::{DateTime, Utc};
use rusqlite::Connection;
use std::path::PathBuf;

fn home() -> PathBuf {
    dirs_next::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

fn cursor_dir() -> PathBuf {
    home().join(".cursor")
}

fn tracking_db_path() -> PathBuf {
    cursor_dir().join("ai-tracking/ai-code-tracking.db")
}

/// Detect Cursor installation and running state
pub fn detect() -> AgentInfo {
    let app_path = PathBuf::from("/Applications/Cursor.app");
    let installed = app_path.exists() || cursor_dir().exists();
    let running = is_cursor_running();

    // Count recent sessions from DB as GUI sessions
    let gui_sessions = if running { count_active_conversations() } else { 0 };

    let gui_version = get_cursor_gui_version();
    let cli_version = get_cursor_cli_version();
    let version = gui_version.clone().or(cli_version.clone());

    AgentInfo {
        name: "Cursor".to_string(),
        agent_type: AgentType::Cursor,
        installed,
        running,
        active_sessions: gui_sessions,
        cli_sessions: 0,
        gui_sessions,
        version,
        cli_version,
        gui_version,
        install_path: if app_path.exists() {
            Some(app_path.to_string_lossy().to_string())
        } else {
            None
        },
    }
}

fn is_cursor_running() -> bool {
    std::process::Command::new("pgrep")
        .arg("-f")
        .arg("Cursor.app")
        .output()
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false)
}

fn count_active_conversations() -> usize {
    let db_path = tracking_db_path();
    if !db_path.exists() {
        return 1; // running but no DB access = at least 1
    }
    let conn = match Connection::open_with_flags(&db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY) {
        Ok(c) => c,
        Err(_) => return 1,
    };
    // Count conversations active in the last hour
    let cutoff = (Utc::now() - chrono::Duration::hours(1)).timestamp_millis();
    conn.query_row(
        "SELECT COUNT(*) FROM conversation_summaries WHERE timestamp >= ?1",
        [cutoff],
        |row| row.get(0),
    )
    .unwrap_or(1)
}

fn get_cursor_gui_version() -> Option<String> {
    let plist = PathBuf::from("/Applications/Cursor.app/Contents/Info.plist");
    if plist.exists() {
        if let Ok(output) = std::process::Command::new("defaults")
            .arg("read")
            .arg(&plist)
            .arg("CFBundleShortVersionString")
            .output()
        {
            let v = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !v.is_empty() {
                return Some(v);
            }
        }
    }
    None
}

fn get_cursor_cli_version() -> Option<String> {
    // Try cursor-agent first (the CLI/TUI variant)
    let candidate = home().join(".local/bin/cursor-agent");
    let cmd = if candidate.exists() {
        candidate.to_string_lossy().to_string()
    } else {
        "cursor-agent".to_string()
    };
    if let Ok(output) = std::process::Command::new(&cmd)
        .arg("--version")
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let first_line = stdout.lines().next().unwrap_or("").trim();
            if !first_line.is_empty() {
                return Some(first_line.to_string());
            }
        }
    }
    // Fallback to cursor --version
    if let Ok(output) = std::process::Command::new("cursor")
        .arg("--version")
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let first_line = stdout.lines().next().unwrap_or("").trim();
            if !first_line.is_empty() {
                return Some(first_line.to_string());
            }
        }
    }
    None
}

/// Get active Cursor sessions (from conversation_summaries if DB accessible)
pub fn get_sessions() -> Vec<Session> {
    let db_path = tracking_db_path();
    if !db_path.exists() {
        return vec![];
    }

    let conn = match Connection::open_with_flags(&db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY) {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    let mut stmt = match conn.prepare(
        "SELECT conversationId, source, timestamp FROM conversation_summaries ORDER BY timestamp DESC LIMIT 20",
    ) {
        Ok(s) => s,
        Err(_) => return vec![],
    };

    let sessions: Vec<Session> = stmt
        .query_map([], |row| {
            let id: String = row.get(0).unwrap_or_default();
            let source: String = row.get(1).unwrap_or_default();
            let ts: i64 = row.get(2).unwrap_or(0);

            let started = DateTime::from_timestamp_millis(ts)
                .map(|d| d.to_rfc3339())
                .unwrap_or_default();

            Ok(Session {
                id,
                agent: "Cursor".to_string(),
                status: "completed".to_string(),
                started_at: Some(started),
                working_dir: None,
                model: Some(source.clone()),
                pid: None,
                entrypoint: source,
            })
        })
        .map(|rows| rows.filter_map(|r| r.ok()).collect())
        .unwrap_or_default();

    sessions
}

/// Get usage stats from ai-tracking database
pub fn get_usage(window: &str) -> UsageStats {
    let db_path = tracking_db_path();
    let window_secs = window_seconds(window);
    let cutoff_ts = (Utc::now().timestamp_millis() as u64)
        .saturating_sub(window_secs * 1000) as i64;

    let mut total = 0usize;
    let mut conversations = std::collections::HashSet::new();

    if db_path.exists() {
        if let Ok(conn) =
            Connection::open_with_flags(&db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        {
            if let Ok(mut stmt) = conn.prepare(
                "SELECT conversationId FROM ai_code_hashes WHERE timestamp >= ?1",
            ) {
                if let Ok(rows) = stmt.query_map([cutoff_ts], |row| {
                    let conv_id: String = row.get(0).unwrap_or_default();
                    Ok(conv_id)
                }) {
                    for row in rows.flatten() {
                        total += 1;
                        conversations.insert(row);
                    }
                }
            }
        }
    }

    UsageStats {
        agent: "Cursor".to_string(),
        window: window.to_string(),
        total_sessions: conversations.len(),
        total_interactions: total,
        first_activity: None,
        last_activity: None,
    }
}

fn window_seconds(window: &str) -> u64 {
    match window {
        "5h" => 5 * 3600,
        "1w" => 7 * 86400,
        "1m" => 30 * 86400,
        _ => 5 * 3600,
    }
}
