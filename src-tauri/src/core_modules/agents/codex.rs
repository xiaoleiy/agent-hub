use crate::models::types::{AgentInfo, AgentType, AgentUsage, ModelUsage, RateWindow, Session, TokenUsage, UsageStats};
use chrono::Utc;
use rusqlite::Connection;
use std::collections::HashMap;
use std::fs;
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
    let codex_app = PathBuf::from("/Applications/Codex.app");
    let installed = codex_bin.exists() || codex_dir().exists() || codex_app.exists();

    // Check live processes
    let live_processes = get_live_pids();
    let running = !live_processes.is_empty() || is_codex_running();

    // Count CLI vs GUI sessions from DB
    let (cli_sessions, gui_sessions) = count_sessions_by_mode();

    let cli_version = get_codex_cli_version();
    let gui_version = get_codex_desktop_version();
    let version = match (&cli_version, &gui_version) {
        (Some(cv), Some(gv)) if cv != gv => Some(format!("CLI {} / GUI {}", cv, gv)),
        (Some(cv), Some(_)) => Some(cv.clone()),
        (Some(cv), None) => Some(cv.clone()),
        (None, Some(gv)) => Some(gv.clone()),
        _ => None,
    };

    AgentInfo {
        name: "Codex".to_string(),
        agent_type: AgentType::Codex,
        installed,
        running,
        active_sessions: cli_sessions + gui_sessions,
        cli_sessions,
        gui_sessions,
        version,
        cli_version,
        gui_version,
        install_path: if codex_bin.exists() {
            Some(codex_bin.to_string_lossy().to_string())
        } else if codex_app.exists() {
            Some(codex_app.to_string_lossy().to_string())
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

fn get_codex_cli_version() -> Option<String> {
    let version_path = codex_dir().join("version.json");
    if let Ok(data) = std::fs::read_to_string(&version_path) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&data) {
            if let Some(ver) = v.get("latest_version").and_then(|v| v.as_str()) {
                return Some(ver.to_string());
            }
        }
    }
    if let Ok(output) = std::process::Command::new("codex").arg("--version").output() {
        if output.status.success() {
            return Some(String::from_utf8_lossy(&output.stdout).trim().to_string());
        }
    }
    None
}

fn get_codex_desktop_version() -> Option<String> {
    let plist = PathBuf::from("/Applications/Codex.app/Contents/Info.plist");
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

/// Get rich usage data by parsing Codex JSONL session files for token_count events
pub fn get_rich_usage() -> AgentUsage {
    let sessions_dir = codex_dir().join("sessions");
    let db_path = state_db_path();

    // Count sessions from DB
    let mut total_sessions = 0usize;
    if db_path.exists() {
        if let Ok(conn) = Connection::open_with_flags(&db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY) {
            if let Ok(count) = conn.query_row(
                "SELECT COUNT(*) FROM threads WHERE archived = 0",
                [],
                |row| row.get::<_, usize>(0),
            ) {
                total_sessions = count;
            }
        }
    }

    // Parse JSONL files for token data and rate limits
    let mut tokens = TokenUsage {
        input_tokens: 0,
        cache_read_tokens: 0,
        cache_create_tokens: 0,
        output_tokens: 0,
        total_tokens: 0,
    };
    let mut model_map: HashMap<String, (u64, u64, usize)> = HashMap::new();
    let mut session_window: Option<RateWindow> = None;
    let mut weekly_window: Option<RateWindow> = None;

    if sessions_dir.exists() {
        collect_codex_jsonl(&sessions_dir, &mut tokens, &mut model_map, &mut session_window, &mut weekly_window);
    }

    let model_breakdowns: Vec<ModelUsage> = model_map
        .into_iter()
        .map(|(model, (inp, out, count))| ModelUsage {
            model,
            input_tokens: inp,
            output_tokens: out,
            total_tokens: inp + out,
            request_count: count,
        })
        .collect();

    let has_tokens = tokens.total_tokens > 0;

    AgentUsage {
        agent: "Codex".to_string(),
        session_window,
        weekly_window,
        tokens: if has_tokens { Some(tokens) } else { None },
        model_breakdowns,
        total_interactions: 0, // Not meaningful for Codex
        total_sessions,
    }
}

/// Recursively scan JSONL files under the Codex sessions directory
fn collect_codex_jsonl(
    dir: &PathBuf,
    tokens: &mut TokenUsage,
    model_map: &mut HashMap<String, (u64, u64, usize)>,
    session_window: &mut Option<RateWindow>,
    weekly_window: &mut Option<RateWindow>,
) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_dir() {
            collect_codex_jsonl(&path, tokens, model_map, session_window, weekly_window);
        } else if path.extension().map(|e| e == "jsonl").unwrap_or(false) {
            parse_codex_jsonl(&path, tokens, model_map, session_window, weekly_window);
        }
    }
}

/// Parse a Codex JSONL file for token_count events
fn parse_codex_jsonl(
    path: &PathBuf,
    tokens: &mut TokenUsage,
    model_map: &mut HashMap<String, (u64, u64, usize)>,
    session_window: &mut Option<RateWindow>,
    weekly_window: &mut Option<RateWindow>,
) {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return,
    };

    for line in content.lines() {
        let v: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        // Look for event_msg with type "token_count"
        if v.get("type").and_then(|t| t.as_str()) != Some("event_msg") {
            continue;
        }
        let payload = match v.get("payload") {
            Some(p) => p,
            None => continue,
        };
        if payload.get("type").and_then(|t| t.as_str()) != Some("token_count") {
            continue;
        }

        // Extract token usage
        let info = match payload.get("info") {
            Some(i) => i,
            None => continue,
        };
        let total_usage = match info.get("total_token_usage") {
            Some(u) => u,
            None => continue,
        };

        let inp = total_usage.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
        let cached = total_usage.get("cached_input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
        let out = total_usage.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
        let reasoning = total_usage.get("reasoning_output_tokens").and_then(|v| v.as_u64()).unwrap_or(0);

        tokens.input_tokens += inp;
        tokens.cache_read_tokens += cached;
        tokens.output_tokens += out + reasoning;
        tokens.total_tokens += inp + out + reasoning;

        // Extract model from turn_context (look backwards in file for model)
        // For now, use "codex" as default model name
        let model = "codex".to_string();
        let entry = model_map.entry(model).or_insert((0, 0, 0));
        entry.0 += inp + cached;
        entry.1 += out + reasoning;
        entry.2 += 1;

        // Extract rate limits
        if let Some(rate_limits) = payload.get("rate_limits") {
            if let Some(primary) = rate_limits.get("primary") {
                let used = primary.get("used_percent").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let mins = primary.get("window_minutes").and_then(|v| v.as_u64()).unwrap_or(300);
                *session_window = Some(RateWindow {
                    used_percent: used,
                    window_minutes: mins,
                    resets_at: None, // Would need to compute from window_minutes
                });
            }
            if let Some(secondary) = rate_limits.get("secondary") {
                let used = secondary.get("used_percent").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let mins = secondary.get("window_minutes").and_then(|v| v.as_u64()).unwrap_or(10080);
                *weekly_window = Some(RateWindow {
                    used_percent: used,
                    window_minutes: mins,
                    resets_at: None,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_codex_dir_exists() {
        let dir = codex_dir();
        assert!(dir.exists(), "codex directory should exist at {:?}", dir);
    }

    #[test]
    fn test_codex_cli_version_format() {
        let version = get_codex_cli_version();
        assert!(version.is_some(), "codex CLI version should be available");
        let v = version.unwrap();
        // Should be like "0.137.0"
        let parts: Vec<&str> = v.split('.').collect();
        assert!(parts.len() >= 2, "version should have at least major.minor: {}", v);
    }

    #[test]
    fn test_codex_desktop_version_format() {
        let version = get_codex_desktop_version();
        if let Some(v) = version {
            // Desktop version is like "26.602.40724"
            assert!(v.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false),
                "desktop version should start with a digit: {}", v);
        }
    }

    #[test]
    fn test_codex_cli_and_desktop_versions_differ() {
        let cli = get_codex_cli_version();
        let desktop = get_codex_desktop_version();
        if let (Some(cv), Some(dv)) = (&cli, &desktop) {
            assert_ne!(cv, dv, "CLI version ({}) should differ from Desktop version ({})", cv, dv);
        }
    }

    #[test]
    fn test_detect_returns_correct_fields() {
        let info = detect();
        assert_eq!(info.name, "Codex");
        assert_eq!(info.agent_type, AgentType::Codex);
    }

    #[test]
    fn test_state_db_exists() {
        let db_path = state_db_path();
        assert!(db_path.exists(), "state_5.sqlite should exist at {:?}", db_path);
    }

    #[test]
    fn test_state_db_has_threads_table() {
        let db_path = state_db_path();
        if !db_path.exists() {
            return;
        }
        let conn = Connection::open_with_flags(&db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let result = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='threads'",
            [],
            |row| row.get::<_, i32>(0),
        );
        assert!(result.unwrap_or(0) > 0, "threads table should exist in state_5.sqlite");
    }

    #[test]
    fn test_threads_table_has_source_column() {
        let db_path = state_db_path();
        if !db_path.exists() {
            return;
        }
        let conn = Connection::open_with_flags(&db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        // Query to check if source column exists
        let result = conn.prepare("SELECT source FROM threads LIMIT 1");
        assert!(result.is_ok(), "threads table should have 'source' column");
    }

    #[test]
    fn test_count_sessions_by_mode() {
        let (cli, gui) = count_sessions_by_mode();
        // We know from the system that there are both CLI and GUI sessions
        assert!(cli + gui > 0, "should have at least some sessions");
    }
}
