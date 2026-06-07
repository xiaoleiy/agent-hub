use crate::models::types::{AgentInfo, AgentType, AgentUsage, ModelUsage, Session, TokenUsage, UsageStats};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::collections::HashMap;
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
    let mut gui_version: Option<String> = None;

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
                        // Track GUI version from session files
                        if !sess.version.is_empty() && sess.entrypoint != "cli" && sess.entrypoint != "sdk-cli" {
                            gui_version = Some(sess.version.clone());
                        }
                    }
                }
            }
        }
    }

    // Get CLI version from the binary itself (more accurate than session files)
    let cli_version = get_claude_cli_version();

    let version = match (&cli_version, &gui_version) {
        (Some(cv), Some(gv)) if cv != gv => Some(format!("CLI {} / GUI {}", cv, gv)),
        (Some(cv), Some(_)) => Some(cv.clone()),
        (Some(cv), None) => Some(cv.clone()),
        (None, Some(gv)) => Some(gv.clone()),
        _ => None,
    };

    AgentInfo {
        name: "Claude Code".to_string(),
        agent_type: AgentType::ClaudeCode,
        installed,
        running,
        active_sessions: cli_sessions + gui_sessions,
        cli_sessions,
        gui_sessions,
        version,
        cli_version,
        gui_version,
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

fn get_claude_cli_version() -> Option<String> {
    let claude_bin = home().join(".local/bin/claude");
    let cmd = if claude_bin.exists() {
        claude_bin.to_string_lossy().to_string()
    } else {
        "claude".to_string()
    };
    if let Ok(output) = std::process::Command::new(&cmd)
        .arg("-v")
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            // Output is like "2.1.168 (Claude Code)" — take just the version part
            let raw = stdout.lines().next().unwrap_or("").trim();
            let version = raw.split_whitespace().next().unwrap_or(raw);
            if !version.is_empty() {
                return Some(version.to_string());
            }
        }
    }
    None
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

/// Get rich usage data by parsing Claude JSONL files for token breakdowns
pub fn get_rich_usage() -> AgentUsage {
    let history_path = claude_dir().join("history.jsonl");
    let projects_dir = claude_dir().join("projects");

    // Count interactions from history.jsonl
    let mut total_interactions = 0usize;
    let mut sessions_set = std::collections::HashSet::new();
    if let Ok(content) = fs::read_to_string(&history_path) {
        for line in content.lines() {
            if let Ok(entry) = serde_json::from_str::<HistoryEntry>(line) {
                total_interactions += 1;
                sessions_set.insert(entry.session_id.clone());
            }
        }
    }

    // Parse JSONL files in projects/ for token data
    let mut tokens = TokenUsage {
        input_tokens: 0,
        cache_read_tokens: 0,
        cache_create_tokens: 0,
        output_tokens: 0,
        total_tokens: 0,
    };
    let mut model_map: HashMap<String, (u64, u64, usize)> = HashMap::new(); // (input, output, count)

    if projects_dir.exists() {
        collect_jsonl_tokens(&projects_dir, &mut tokens, &mut model_map);
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
        agent: "Claude Code".to_string(),
        session_window: None, // Would need OAuth API call
        weekly_window: None,
        tokens: if has_tokens { Some(tokens) } else { None },
        model_breakdowns,
        total_interactions,
        total_sessions: sessions_set.len(),
    }
}

/// Recursively scan JSONL files under a directory for token usage data
fn collect_jsonl_tokens(
    dir: &PathBuf,
    tokens: &mut TokenUsage,
    model_map: &mut HashMap<String, (u64, u64, usize)>,
) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_dir() {
            collect_jsonl_tokens(&path, tokens, model_map);
        } else if path.extension().map(|e| e == "jsonl").unwrap_or(false) {
            parse_claude_jsonl(&path, tokens, model_map);
        }
    }
}

/// Parse a single Claude JSONL file for token usage from assistant messages
fn parse_claude_jsonl(
    path: &PathBuf,
    tokens: &mut TokenUsage,
    model_map: &mut HashMap<String, (u64, u64, usize)>,
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

        // Only process "assistant" type messages with usage data
        if v.get("type").and_then(|t| t.as_str()) != Some("assistant") {
            continue;
        }

        let usage = match v.pointer("/message/usage") {
            Some(u) => u,
            None => continue,
        };

        let model = v
            .pointer("/message/model")
            .and_then(|m| m.as_str())
            .unwrap_or("unknown")
            .to_string();

        let inp = usage
            .get("input_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let cache_read = usage
            .get("cache_read_input_tokens")
            .or_else(|| usage.get("cache_read_tokens"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let cache_create = usage
            .get("cache_creation_input_tokens")
            .or_else(|| usage.get("cache_creation_tokens"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let out = usage
            .get("output_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        if inp + cache_read + cache_create + out == 0 {
            continue;
        }

        tokens.input_tokens += inp;
        tokens.cache_read_tokens += cache_read;
        tokens.cache_create_tokens += cache_create;
        tokens.output_tokens += out;
        tokens.total_tokens += inp + cache_read + cache_create + out;

        let entry = model_map.entry(model).or_insert((0, 0, 0));
        entry.0 += inp + cache_read + cache_create;
        entry.1 += out;
        entry.2 += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claude_binary_exists() {
        let claude_bin = home().join(".local/bin/claude");
        assert!(claude_bin.exists(), "claude binary should exist at {:?}", claude_bin);
    }

    #[test]
    fn test_claude_cli_version_format() {
        let version = get_claude_cli_version();
        assert!(version.is_some(), "claude -v should return a value");
        let v = version.unwrap();
        // Should be like "2.1.168"
        let parts: Vec<&str> = v.split('.').collect();
        assert!(parts.len() >= 2, "version should have at least major.minor: {}", v);
        for part in &parts {
            assert!(part.chars().all(|c| c.is_ascii_digit()),
                "version part should be numeric: {} in {}", part, v);
        }
    }

    #[test]
    fn test_detect_returns_correct_fields() {
        let info = detect();
        assert_eq!(info.name, "Claude Code");
        assert_eq!(info.agent_type, AgentType::ClaudeCode);
        // CLI version should be present since claude is installed
        assert!(info.cli_version.is_some(), "cli_version should be set");
    }

    #[test]
    fn test_sessions_dir_exists() {
        let dir = sessions_dir();
        assert!(dir.exists(), "sessions directory should exist at {:?}", dir);
    }

    #[test]
    fn test_parse_window() {
        assert_eq!(parse_window("5h"), chrono::Duration::hours(5));
        assert_eq!(parse_window("1w"), chrono::Duration::weeks(1));
        assert_eq!(parse_window("1m"), chrono::Duration::days(30));
        assert_eq!(parse_window("other"), chrono::Duration::hours(5));
    }

    #[test]
    fn test_claude_session_json_parsing() {
        let json = r#"{"pid":12345,"session_id":"abc","cwd":"/tmp","started_at":1000,"version":"2.1.168","status":"busy","entrypoint":"cli"}"#;
        let sess: ClaudeSession = serde_json::from_str(json).unwrap();
        assert_eq!(sess.pid, 12345);
        assert_eq!(sess.session_id, "abc");
        assert_eq!(sess.version, "2.1.168");
        assert_eq!(sess.entrypoint, "cli");
    }

    #[test]
    fn test_claude_session_json_defaults() {
        let json = r#"{"pid":12345}"#;
        let sess: ClaudeSession = serde_json::from_str(json).unwrap();
        assert_eq!(sess.pid, 12345);
        assert_eq!(sess.session_id, "");
        assert_eq!(sess.version, "");
        assert_eq!(sess.entrypoint, "");
    }
}
