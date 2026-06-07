use crate::models::types::{AgentInfo, AgentType, Session, UsageStats};
use chrono::Utc;
use rusqlite::Connection;
use std::path::PathBuf;

fn home() -> PathBuf {
    dirs_next::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

fn codex_dir() -> PathBuf {
    home().join(".codex")
}

fn state_db_path() -> PathBuf {
    codex_dir().join("state_5.sqlite")
}

fn process_manager_path() -> PathBuf {
    codex_dir().join("process_manager/chat_processes.json")
}

/// Detect Codex CLI installation and running state
pub fn detect() -> AgentInfo {
    let codex_bin = PathBuf::from("/opt/homebrew/bin/codex");
    let installed = codex_bin.exists() || codex_dir().exists();

    // Check live processes
    let live_processes = get_live_pids();
    let running = !live_processes.is_empty() || is_codex_running();

    // Count CLI vs GUI sessions from DB
    let (cli_sessions, gui_sessions) = count_sessions_by_mode();

    AgentInfo {
        name: "Codex".to_string(),
        agent_type: AgentType::Codex,
        installed,
        running,
        active_sessions: cli_sessions + gui_sessions,
        cli_sessions,
        gui_sessions,
        version: get_codex_version(),
        install_path: if codex_bin.exists() {
            Some(codex_bin.to_string_lossy().to_string())
        } else {
            None
        },
    }
}

fn is_codex_running() -> bool {
    std::process::Command::new("pgrep")
        .arg("-f")
        .arg("codex")
        .output()
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false)
}

fn get_live_pids() -> Vec<u32> {
    let path = process_manager_path();
    if !path.exists() {
        return vec![];
    }
    if let Ok(data) = std::fs::read_to_string(&path) {
        if let Ok(procs) = serde_json::from_str::<Vec<ProcessEntry>>(&data) {
            return procs
                .iter()
                .filter(|p| is_pid_alive(p.pid))
                .map(|p| p.pid)
                .collect();
        }
    }
    vec![]
}

#[derive(serde::Deserialize)]
struct ProcessEntry {
    pid: u32,
}

fn is_pid_alive(pid: u32) -> bool {
    std::process::Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Count CLI vs GUI sessions from the threads table
fn count_sessions_by_mode() -> (usize, usize) {
    let db_path = state_db_path();
    if !db_path.exists() {
        return (0, 0);
    }
    let conn = match Connection::open_with_flags(&db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY) {
        Ok(c) => c,
        Err(_) => return (0, 0),
    };
    let mut stmt = match conn.prepare(
        "SELECT source FROM threads WHERE archived = 0",
    ) {
        Ok(s) => s,
        Err(_) => return (0, 0),
    };
    let mut cli = 0usize;
    let mut gui = 0usize;
    if let Ok(rows) = stmt.query_map([], |row| {
        let source: String = row.get(0).unwrap_or_default();
        Ok(source)
    }) {
        for row in rows.flatten() {
            if row == "cli" {
                cli += 1;
            } else {
                gui += 1;
            }
        }
    }
    (cli, gui)
}

fn get_codex_version() -> Option<String> {
    let version_path = codex_dir().join("version.json");
    if let Ok(data) = std::fs::read_to_string(&version_path) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&data) {
            return v
                .get("latest_version")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
        }
    }
    if let Ok(output) = std::process::Command::new("codex").arg("--version").output() {
        if output.status.success() {
            return Some(String::from_utf8_lossy(&output.stdout).trim().to_string());
        }
    }
    None
}

/// Get active Codex sessions from state_5.sqlite threads table
pub fn get_sessions() -> Vec<Session> {
    let db_path = state_db_path();
    if !db_path.exists() {
        return vec![];
    }

    let conn = match Connection::open_with_flags(&db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY) {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    let mut stmt = match conn.prepare(
        "SELECT id, title, source, model_provider, cwd, created_at, updated_at
         FROM threads WHERE archived = 0 ORDER BY updated_at DESC LIMIT 20",
    ) {
        Ok(s) => s,
        Err(_) => return vec![],
    };

    let sessions: Vec<Session> = stmt
        .query_map([], |row| {
            let id: String = row.get(0).unwrap_or_default();
            let title: String = row.get(1).unwrap_or_default();
            let source: String = row.get(2).unwrap_or_default();
            let model: String = row.get(3).unwrap_or_default();
            let cwd: String = row.get(4).unwrap_or_default();
            let updated_at: String = row.get(6).unwrap_or_default();

            Ok(Session {
                id: if title.is_empty() { id } else { title },
                agent: "Codex".to_string(),
                status: source.clone(),
                started_at: Some(updated_at),
                working_dir: if cwd.is_empty() { None } else { Some(cwd) },
                model: if model.is_empty() { None } else { Some(model) },
                pid: None,
                entrypoint: source,
            })
        })
        .map(|rows| rows.filter_map(|r| r.ok()).collect())
        .unwrap_or_default();

    sessions
}

/// Get usage stats from state_5.sqlite threads table
pub fn get_usage(window: &str) -> UsageStats {
    let db_path = state_db_path();
    let window_secs = window_seconds(window);
    let cutoff = Utc::now() - chrono::Duration::seconds(window_secs as i64);
    let cutoff_str = cutoff.format("%Y-%m-%dT%H:%M:%S").to_string();

    let mut total_interactions = 0usize;
    let mut session_count = 0usize;

    if db_path.exists() {
        if let Ok(conn) =
            Connection::open_with_flags(&db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        {
            if let Ok(mut stmt) = conn.prepare(
                "SELECT id, tokens_used FROM threads WHERE updated_at >= ?1 AND archived = 0",
            ) {
                if let Ok(rows) = stmt.query_map([&cutoff_str], |row| {
                    let tokens: i64 = row.get(1).unwrap_or(0);
                    Ok(tokens as usize)
                }) {
                    for row in rows.flatten() {
                        session_count += 1;
                        total_interactions += row;
                    }
                }
            }
        }
    }

    UsageStats {
        agent: "Codex".to_string(),
        window: window.to_string(),
        total_sessions: session_count,
        total_interactions,
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
