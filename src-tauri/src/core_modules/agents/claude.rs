use crate::models::types::{
    AccountInfo, AgentInfo, AgentType, AgentUsage, ModelUsage, RateWindow, Session, TokenUsage,
    UsageStats,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime};

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
    let mut cli_session_version: Option<(u64, String)> = None;

    if sessions.exists() {
        if let Ok(entries) = fs::read_dir(&sessions) {
            for entry in entries.filter_map(|e| e.ok()) {
                if let Ok(data) = fs::read_to_string(entry.path()) {
                    if let Ok(sess) = serde_json::from_str::<ClaudeSession>(&data) {
                        let is_cli = matches!(sess.entrypoint.as_str(), "cli" | "sdk-cli");
                        if is_pid_alive(sess.pid) {
                            running = true;
                            if is_cli {
                                cli_sessions += 1;
                            } else {
                                gui_sessions += 1;
                            }
                        }
                        if !sess.version.is_empty() {
                            if is_cli {
                                // Track the most-recently-started CLI session's version —
                                // this is the actually-running CLI version, independent of
                                // PATH / symlink quirks in the installed binary.
                                let ts = sess.started_at.unwrap_or(0);
                                if cli_session_version
                                    .as_ref()
                                    .map(|(t, _)| ts >= *t)
                                    .unwrap_or(true)
                                {
                                    cli_session_version = Some((ts, sess.version.clone()));
                                }
                            } else {
                                gui_version = Some(sess.version.clone());
                            }
                        }
                    }
                }
            }
        }
    }

    // CLI version: prefer the running CLI session's version, else the binary.
    let cli_version = cli_session_version
        .map(|(_, v)| v)
        .or_else(get_claude_cli_version);

    // GUI version: a running GUI session's version, else the Claude desktop app.
    let gui_version = gui_version.or_else(get_claude_desktop_version);

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
            Some(
                home()
                    .join(".local/bin/claude")
                    .to_string_lossy()
                    .to_string(),
            )
        } else {
            None
        },
        account: get_account(),
    }
}

// ~/.claude.json can be large; cache the parsed account by file mtime.
static CLAUDE_ACCOUNT_CACHE: Mutex<Option<(SystemTime, Option<AccountInfo>)>> = Mutex::new(None);

/// Logged-in account from ~/.claude.json `oauthAccount`.
fn get_account() -> Option<AccountInfo> {
    let path = home().join(".claude.json");
    let mtime = fs::metadata(&path).ok()?.modified().ok()?;
    if let Ok(guard) = CLAUDE_ACCOUNT_CACHE.lock() {
        if let Some((m, acc)) = guard.as_ref() {
            if *m == mtime {
                return acc.clone();
            }
        }
    }
    let acc = parse_claude_account(&path);
    if let Ok(mut guard) = CLAUDE_ACCOUNT_CACHE.lock() {
        *guard = Some((mtime, acc.clone()));
    }
    acc
}

fn parse_claude_account(path: &Path) -> Option<AccountInfo> {
    let data = fs::read_to_string(path).ok()?;
    let v: serde_json::Value = serde_json::from_str(&data).ok()?;
    let acc = v.get("oauthAccount")?;
    let s = |k: &str| acc.get(k).and_then(|x| x.as_str()).map(str::to_string);
    let info = AccountInfo {
        email: s("emailAddress"),
        display_name: s("displayName"),
        organization: s("organizationName"),
    };
    if info.email.is_none() && info.display_name.is_none() {
        None
    } else {
        Some(info)
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
    // Prefer `claude` on PATH (what the user actually runs). The ~/.local/bin
    // symlink can point at an older installed version, so it's only a fallback.
    let local_bin = home()
        .join(".local/bin/claude")
        .to_string_lossy()
        .to_string();
    for cmd in ["claude".to_string(), local_bin] {
        if let Ok(output) = std::process::Command::new(&cmd).arg("-v").output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                // Output is like "2.1.156 (Claude Code)" — take just the version part
                let raw = stdout.lines().next().unwrap_or("").trim();
                let version = raw.split_whitespace().next().unwrap_or(raw);
                if !version.is_empty() {
                    return Some(version.to_string());
                }
            }
        }
    }
    None
}

/// Candidate locations for the Claude desktop app (varies by install method).
fn claude_app_candidates() -> Vec<PathBuf> {
    vec![
        PathBuf::from("/Applications/Claude.app"),
        home().join("Applications/Claude.app"),
        home().join("Apps/Claude.app"),
    ]
}

/// Read the Claude desktop app's version from its bundle, if installed.
fn get_claude_desktop_version() -> Option<String> {
    let app = claude_app_candidates().into_iter().find(|p| p.exists())?;
    let plist = app.join("Contents/Info.plist");
    let output = std::process::Command::new("defaults")
        .arg("read")
        .arg(&plist)
        .arg("CFBundleShortVersionString")
        .output()
        .ok()?;
    let v = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if v.is_empty() {
        None
    } else {
        Some(v)
    }
}

fn is_pid_alive(pid: u32) -> bool {
    std::process::Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

// Claude writes these files with camelCase keys (sessionId, startedAt, ...).
// rename_all maps our snake_case fields onto them — without it, serde(default)
// silently leaves session_id/started_at empty and the data never shows up.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
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

// history.jsonl also uses camelCase (sessionId). timestamp is epoch millis.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
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

    // Rate limits: Claude Code doesn't persist them locally, but subscriber
    // accounts expose them via the OAuth usage endpoint (same as the CodexBar
    // approach). Best-effort + cached; returns (None, None) for API-key users
    // or if the call fails.
    let (session_window, weekly_window) = fetch_claude_rate_limits();

    AgentUsage {
        agent: "Claude Code".to_string(),
        session_window,
        weekly_window,
        tokens: if has_tokens { Some(tokens) } else { None },
        model_breakdowns,
        total_interactions,
        total_sessions: sessions_set.len(),
    }
}

/// Per-file token aggregate, cached so unchanged transcripts aren't re-parsed.
#[derive(Clone, Default)]
struct FileTokenAgg {
    input: u64,
    cache_read: u64,
    cache_create: u64,
    output: u64,
    /// (model, input+cache, output, count)
    models: Vec<(String, u64, u64, usize)>,
}

// Claude's projects/ dir holds every transcript ever; re-reading and re-parsing
// all of them on each refresh is the dominant cost. Cache each file's aggregate
// keyed by mtime — only changed (active) files are re-parsed.
static CLAUDE_TOKEN_CACHE: Mutex<Option<HashMap<PathBuf, (SystemTime, FileTokenAgg)>>> =
    Mutex::new(None);

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
            if let Some(agg) = file_token_agg(&path) {
                tokens.input_tokens += agg.input;
                tokens.cache_read_tokens += agg.cache_read;
                tokens.cache_create_tokens += agg.cache_create;
                tokens.output_tokens += agg.output;
                tokens.total_tokens += agg.input + agg.cache_read + agg.cache_create + agg.output;
                for (model, inp, out, count) in agg.models {
                    let e = model_map.entry(model).or_insert((0, 0, 0));
                    e.0 += inp;
                    e.1 += out;
                    e.2 += count;
                }
            }
        }
    }
}

/// Return a file's token aggregate, using the mtime cache when possible.
fn file_token_agg(path: &Path) -> Option<FileTokenAgg> {
    let mtime = fs::metadata(path).ok()?.modified().ok()?;
    if let Ok(guard) = CLAUDE_TOKEN_CACHE.lock() {
        if let Some(map) = guard.as_ref() {
            if let Some((cached_mtime, agg)) = map.get(path) {
                if *cached_mtime == mtime {
                    return Some(agg.clone());
                }
            }
        }
    }
    let agg = parse_claude_jsonl(path);
    if let Ok(mut guard) = CLAUDE_TOKEN_CACHE.lock() {
        guard
            .get_or_insert_with(HashMap::new)
            .insert(path.to_path_buf(), (mtime, agg.clone()));
    }
    Some(agg)
}

/// Parse a single Claude JSONL file for token usage from assistant messages.
/// Claude's per-message `usage` is per-call (not cumulative), so summing across
/// messages is correct.
fn parse_claude_jsonl(path: &Path) -> FileTokenAgg {
    let mut agg = FileTokenAgg::default();
    let mut models: HashMap<String, (u64, u64, usize)> = HashMap::new();

    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return agg,
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

        agg.input += inp;
        agg.cache_read += cache_read;
        agg.cache_create += cache_create;
        agg.output += out;

        let entry = models.entry(model).or_insert((0, 0, 0));
        entry.0 += inp + cache_read + cache_create;
        entry.1 += out;
        entry.2 += 1;
    }

    agg.models = models
        .into_iter()
        .map(|(m, (i, o, c))| (m, i, o, c))
        .collect();
    agg
}

// ─── Rate limits (OAuth usage endpoint, subscriber accounts) ─────────────────

#[derive(Deserialize)]
struct ClaudeCreds {
    #[serde(rename = "claudeAiOauth")]
    claude_ai_oauth: Option<ClaudeOauth>,
}

#[derive(Deserialize)]
struct ClaudeOauth {
    #[serde(rename = "accessToken")]
    access_token: Option<String>,
}

/// Read the Claude OAuth access token from ~/.claude/.credentials.json, falling
/// back to the macOS Keychain (service "Claude Code-credentials"). Same sources
/// the Claude CLI itself uses.
fn claude_oauth_token() -> Option<String> {
    let parse = |raw: &str| -> Option<String> {
        serde_json::from_str::<ClaudeCreds>(raw)
            .ok()?
            .claude_ai_oauth?
            .access_token
            .filter(|t| !t.is_empty())
    };

    if let Ok(data) = fs::read_to_string(claude_dir().join(".credentials.json")) {
        if let Some(t) = parse(&data) {
            return Some(t);
        }
    }

    // Keychain fallback (best-effort; may be unavailable without prompt approval).
    if let Ok(out) = std::process::Command::new("security")
        .args([
            "find-generic-password",
            "-s",
            "Claude Code-credentials",
            "-w",
        ])
        .output()
    {
        if out.status.success() {
            if let Some(t) = parse(String::from_utf8_lossy(&out.stdout).trim()) {
                return Some(t);
            }
        }
    }
    None
}

#[derive(Deserialize)]
struct OAuthUsageResp {
    five_hour: Option<OAuthWindow>,
    seven_day: Option<OAuthWindow>,
}

#[derive(Deserialize)]
struct OAuthWindow {
    utilization: Option<f64>,
    resets_at: Option<String>,
}

// The OAuth usage endpoint is rate-limited by Anthropic, and get_rich_usage is
// called every few seconds while the Claude tab is open — cache for 60s.
type RateLimitCache = Option<(Instant, Option<RateWindow>, Option<RateWindow>)>;

static CLAUDE_RL_CACHE: Mutex<RateLimitCache> = Mutex::new(None);

/// (session_window, weekly_window) from the OAuth usage endpoint. Cached 60s.
fn fetch_claude_rate_limits() -> (Option<RateWindow>, Option<RateWindow>) {
    if let Ok(guard) = CLAUDE_RL_CACHE.lock() {
        if let Some((t, s, w)) = guard.as_ref() {
            if t.elapsed() < Duration::from_secs(60) {
                return (s.clone(), w.clone());
            }
        }
    }
    let result = fetch_claude_rate_limits_uncached();
    if let Ok(mut guard) = CLAUDE_RL_CACHE.lock() {
        *guard = Some((Instant::now(), result.0.clone(), result.1.clone()));
    }
    result
}

fn fetch_claude_rate_limits_uncached() -> (Option<RateWindow>, Option<RateWindow>) {
    let token = match claude_oauth_token() {
        Some(t) => t,
        None => return (None, None),
    };
    let rt = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(_) => return (None, None),
    };
    let resp: Option<OAuthUsageResp> = rt.block_on(async {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .ok()?;
        let r = client
            .get("https://api.anthropic.com/api/oauth/usage")
            .header("Authorization", format!("Bearer {}", token))
            .header("anthropic-beta", "oauth-2025-04-20")
            .send()
            .await
            .ok()?;
        if !r.status().is_success() {
            return None;
        }
        r.json::<OAuthUsageResp>().await.ok()
    });

    let resp = match resp {
        Some(r) => r,
        None => return (None, None),
    };

    // `utilization` is already a 0-100 percent. five_hour → session (fallback to
    // seven_day), seven_day → weekly.
    let mk = |w: &OAuthWindow, mins: u64| -> Option<RateWindow> {
        Some(RateWindow {
            used_percent: w.utilization?,
            window_minutes: mins,
            resets_at: w.resets_at.clone(),
            label: None,
        })
    };

    let session = resp
        .five_hour
        .as_ref()
        .and_then(|w| mk(w, 300))
        .or_else(|| resp.seven_day.as_ref().and_then(|w| mk(w, 10080)));
    let weekly = resp.seven_day.as_ref().and_then(|w| mk(w, 10080));

    (session, weekly)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claude_binary_exists() {
        let claude_bin = home().join(".local/bin/claude");
        assert!(
            claude_bin.exists(),
            "claude binary should exist at {:?}",
            claude_bin
        );
    }

    #[test]
    fn test_claude_cli_version_format() {
        let version = get_claude_cli_version();
        assert!(version.is_some(), "claude -v should return a value");
        let v = version.unwrap();
        // Should be like "2.1.168"
        let parts: Vec<&str> = v.split('.').collect();
        assert!(
            parts.len() >= 2,
            "version should have at least major.minor: {}",
            v
        );
        for part in &parts {
            assert!(
                part.chars().all(|c| c.is_ascii_digit()),
                "version part should be numeric: {} in {}",
                part,
                v
            );
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
        // Real Claude session files use camelCase keys (sessionId, startedAt).
        let json = r#"{"pid":12345,"sessionId":"abc","cwd":"/tmp","startedAt":1000,"version":"2.1.168","status":"busy","entrypoint":"cli"}"#;
        let sess: ClaudeSession = serde_json::from_str(json).unwrap();
        assert_eq!(sess.pid, 12345);
        assert_eq!(sess.session_id, "abc", "sessionId must map to session_id");
        assert_eq!(
            sess.started_at,
            Some(1000),
            "startedAt must map to started_at"
        );
        assert_eq!(sess.version, "2.1.168");
        assert_eq!(sess.entrypoint, "cli");
    }

    #[test]
    fn test_history_entry_camelcase() {
        // history.jsonl uses camelCase sessionId and millisecond timestamps.
        let json = r#"{"display":"/ide","timestamp":1772699516370,"sessionId":"8e4a29cf"}"#;
        let entry: HistoryEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.timestamp, 1772699516370);
        assert_eq!(
            entry.session_id, "8e4a29cf",
            "sessionId must map to session_id"
        );
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
