use crate::models::types::{
    AccountInfo, AgentInfo, AgentType, AgentUsage, ModelUsage, RateWindow, Session, TokenUsage,
    UsageStats,
};
use base64::Engine;
use chrono::{DateTime, Utc};
use rusqlite::Connection;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

/// Candidate locations for the Codex desktop app (varies by install method).
fn codex_app_candidates() -> Vec<PathBuf> {
    vec![
        PathBuf::from("/Applications/Codex.app"),
        home().join("Applications/Codex.app"),
        home().join("Apps/Codex.app"),
    ]
}

fn codex_app_path() -> Option<PathBuf> {
    codex_app_candidates().into_iter().find(|p| p.exists())
}

fn home() -> PathBuf {
    dirs_next::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

fn codex_dir() -> PathBuf {
    home().join(".codex")
}

fn state_db_path() -> PathBuf {
    codex_dir().join("state_5.sqlite")
}

/// Detect Codex CLI installation and running state
pub fn detect() -> AgentInfo {
    let codex_bin = PathBuf::from("/opt/homebrew/bin/codex");
    let codex_app = codex_app_path();
    let installed = codex_bin.exists() || codex_dir().exists() || codex_app.is_some();

    // Active sessions = conversations active in the recent window (not lifetime
    // threads). running = a codex process is alive, or there's recent activity.
    let (cli_sessions, gui_sessions) = count_active_sessions();
    let running = cli_sessions + gui_sessions > 0 || is_codex_running();

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
        } else {
            codex_app
                .as_ref()
                .map(|app| app.to_string_lossy().to_string())
        },
        account: get_account(),
    }
}

/// Logged-in account from ~/.codex/auth.json (decode the id_token JWT payload).
fn get_account() -> Option<AccountInfo> {
    let data = std::fs::read_to_string(codex_dir().join("auth.json")).ok()?;
    let v: serde_json::Value = serde_json::from_str(&data).ok()?;
    let id_token = v
        .pointer("/tokens/id_token")
        .or_else(|| v.get("id_token"))
        .and_then(|x| x.as_str())?;
    let payload = id_token.split('.').nth(1)?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .ok()?;
    let claims: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    let s = |k: &str| claims.get(k).and_then(|x| x.as_str()).map(str::to_string);
    let info = AccountInfo {
        email: s("email"),
        display_name: s("name"),
        organization: None,
    };
    if info.email.is_none() && info.display_name.is_none() {
        None
    } else {
        Some(info)
    }
}

fn is_codex_running() -> bool {
    // -x matches the exact process name "codex". `-f codex` matched any process
    // whose full command line / env contained "codex" (e.g. unrelated MCP servers
    // with .codex in their PATH), producing false "running" states.
    std::process::Command::new("pgrep")
        .arg("-x")
        .arg("codex")
        .output()
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false)
}

/// Count active sessions = threads updated within the recent window, split by
/// source (cli/sdk-cli vs everything else). Previously this counted ALL
/// non-archived threads (lifetime history), which massively overcounted —
/// e.g. "111 active (13 CLI, 98 GUI)" for a user with no active session.
fn count_active_sessions() -> (usize, usize) {
    const WINDOW_SECS: i64 = 15 * 60;
    let db_path = state_db_path();
    if !db_path.exists() {
        return (0, 0);
    }
    let conn =
        match Connection::open_with_flags(&db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY) {
            Ok(c) => c,
            Err(_) => return (0, 0),
        };
    // updated_at is INTEGER epoch seconds — compare against an integer cutoff.
    let cutoff = Utc::now().timestamp() - WINDOW_SECS;
    let mut stmt =
        match conn.prepare("SELECT source FROM threads WHERE archived = 0 AND updated_at >= ?1") {
            Ok(s) => s,
            Err(_) => return (0, 0),
        };
    let mut cli = 0usize;
    let mut gui = 0usize;
    if let Ok(rows) = stmt.query_map([cutoff], |row| row.get::<_, String>(0)) {
        for source in rows.flatten() {
            match source.as_str() {
                "cli" | "sdk-cli" => cli += 1,
                _ => gui += 1,
            }
        }
    }
    (cli, gui)
}

fn get_codex_cli_version() -> Option<String> {
    // Prefer the actual installed binary. `codex --version` prints e.g.
    // "codex-cli 0.130.0" — take the first version-looking token.
    if let Ok(output) = std::process::Command::new("codex")
        .arg("--version")
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Some(tok) = stdout.split_whitespace().find(|t| {
                t.chars()
                    .next()
                    .map(|c| c.is_ascii_digit())
                    .unwrap_or(false)
            }) {
                return Some(tok.to_string());
            }
        }
    }
    // Fallback (e.g. GUI launch with no PATH): version.json holds the update
    // checker's "latest_version" — an approximation, NOT necessarily installed.
    let version_path = codex_dir().join("version.json");
    if let Ok(data) = std::fs::read_to_string(&version_path) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&data) {
            if let Some(ver) = v.get("latest_version").and_then(|v| v.as_str()) {
                return Some(ver.to_string());
            }
        }
    }
    None
}

fn get_codex_desktop_version() -> Option<String> {
    let app = codex_app_path()?;
    let plist = app.join("Contents/Info.plist");
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

    let conn =
        match Connection::open_with_flags(&db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY) {
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
            // updated_at is an INTEGER (epoch seconds), not text — read it as i64
            // and format it, otherwise the Time column is blank.
            let updated_at: i64 = row.get(6).unwrap_or(0);
            let started = DateTime::from_timestamp(updated_at, 0).map(|d| d.to_rfc3339());

            Ok(Session {
                id: if title.is_empty() { id } else { title },
                agent: "Codex".to_string(),
                status: source.clone(),
                started_at: started,
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
    // threads.updated_at is an INTEGER epoch-seconds column. The previous code
    // compared it against a formatted date STRING, which SQLite coerced to the
    // number 2026 (numeric affinity) — making the window filter a no-op. Compare
    // against an integer epoch instead.
    let cutoff = Utc::now().timestamp() - window_secs as i64;

    let mut session_count = 0usize;

    if db_path.exists() {
        if let Ok(conn) =
            Connection::open_with_flags(&db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        {
            if let Ok(mut stmt) =
                conn.prepare("SELECT id FROM threads WHERE updated_at >= ?1 AND archived = 0")
            {
                if let Ok(rows) = stmt.query_map([cutoff], |_row| Ok(())) {
                    for _ in rows.flatten() {
                        session_count += 1;
                    }
                }
            }
        }
    }

    UsageStats {
        agent: "Codex".to_string(),
        window: window.to_string(),
        total_sessions: session_count,
        // Codex has no per-message count in this table; report active threads as
        // the activity count. (Previously this summed tokens_used, which made the
        // cross-agent dashboard chart compare token totals against message counts.)
        total_interactions: session_count,
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
        if let Ok(conn) =
            Connection::open_with_flags(&db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        {
            if let Ok(count) = conn.query_row(
                "SELECT COUNT(*) FROM threads WHERE archived = 0",
                [],
                |row| row.get::<_, usize>(0),
            ) {
                total_sessions = count;
            }
        }
    }

    // Aggregate token usage + most-recent rate limits across all session files.
    let agg = aggregate_codex_sessions(&sessions_dir);

    let tokens = TokenUsage {
        input_tokens: agg.input,
        cache_read_tokens: agg.cached,
        cache_create_tokens: 0, // Codex doesn't report cache-creation separately
        output_tokens: agg.output,
        total_tokens: agg.total,
    };

    let model_breakdowns = if agg.total > 0 {
        vec![ModelUsage {
            model: "codex".to_string(),
            input_tokens: agg.input + agg.cached,
            output_tokens: agg.output,
            total_tokens: agg.total,
            request_count: agg.session_files,
        }]
    } else {
        vec![]
    };

    let has_tokens = agg.total > 0;

    AgentUsage {
        agent: "Codex".to_string(),
        session_window: agg.primary.map(RateSnapshot::into_window),
        weekly_window: agg.secondary.map(RateSnapshot::into_window),
        tokens: if has_tokens { Some(tokens) } else { None },
        model_breakdowns,
        total_interactions: 0, // Not meaningful for Codex
        total_sessions,
    }
}

/// A rate-limit snapshot from a Codex `token_count` event.
#[derive(Clone)]
struct RateSnapshot {
    used_percent: f64,
    window_minutes: u64,
    resets_at: Option<i64>, // epoch seconds
}

impl RateSnapshot {
    fn into_window(self) -> RateWindow {
        RateWindow {
            used_percent: self.used_percent,
            window_minutes: self.window_minutes,
            resets_at: self
                .resets_at
                .and_then(|s| DateTime::from_timestamp(s, 0))
                .map(|d| d.to_rfc3339()),
            label: None,
        }
    }
}

/// Per-session-file aggregate. Codex's `total_token_usage` is a CUMULATIVE
/// running total, so the final event in a file is the session's grand total —
/// we keep the last value rather than summing events (which over-counted).
#[derive(Clone, Default)]
struct CodexFileAgg {
    input: u64,
    cached: u64,
    output: u64,
    total: u64,
    primary: Option<RateSnapshotData>,
    secondary: Option<RateSnapshotData>,
}

#[derive(Clone, Default)]
struct RateSnapshotData {
    used_percent: f64,
    window_minutes: u64,
    resets_at: Option<i64>,
}

/// Aggregate across all session files, with a top-level (cross-file) result.
#[derive(Default)]
struct CodexAggregate {
    input: u64,
    cached: u64,
    output: u64,
    total: u64,
    session_files: usize,
    primary: Option<RateSnapshot>,
    secondary: Option<RateSnapshot>,
}

// Re-reading every rollout file each refresh is expensive; cache each file's
// aggregate keyed by mtime so only the active (growing) file is re-parsed.
static CODEX_TOKEN_CACHE: Mutex<Option<HashMap<PathBuf, (SystemTime, CodexFileAgg)>>> =
    Mutex::new(None);

/// Walk the sessions dir, summing each session's FINAL cumulative totals and
/// taking rate limits from the most-recently-modified file.
fn aggregate_codex_sessions(dir: &Path) -> CodexAggregate {
    let mut out = CodexAggregate::default();
    let mut newest: Option<SystemTime> = None;
    walk_codex_files(dir, &mut |path, mtime, agg| {
        if agg.total > 0 {
            out.input += agg.input;
            out.cached += agg.cached;
            out.output += agg.output;
            out.total += agg.total;
            out.session_files += 1;
        }
        // Rate limits: keep the snapshot from the newest file that has one.
        let is_newer = newest.map(|n| mtime > n).unwrap_or(true);
        if is_newer && (agg.primary.is_some() || agg.secondary.is_some()) {
            newest = Some(mtime);
            out.primary = agg.primary.clone().map(snapshot_from_data);
            out.secondary = agg.secondary.clone().map(snapshot_from_data);
        }
        let _ = path;
    });
    out
}

fn snapshot_from_data(d: RateSnapshotData) -> RateSnapshot {
    RateSnapshot {
        used_percent: d.used_percent,
        window_minutes: d.window_minutes,
        resets_at: d.resets_at,
    }
}

/// Recurse the sessions dir, yielding each .jsonl file's cached aggregate.
fn walk_codex_files(dir: &Path, f: &mut impl FnMut(&Path, SystemTime, &CodexFileAgg)) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_dir() {
            walk_codex_files(&path, f);
        } else if path.extension().map(|e| e == "jsonl").unwrap_or(false) {
            if let Some((mtime, agg)) = codex_file_agg(&path) {
                f(&path, mtime, &agg);
            }
        }
    }
}

/// Return a file's aggregate + mtime, using the cache when unchanged.
fn codex_file_agg(path: &Path) -> Option<(SystemTime, CodexFileAgg)> {
    let mtime = fs::metadata(path).ok()?.modified().ok()?;
    if let Ok(guard) = CODEX_TOKEN_CACHE.lock() {
        if let Some(map) = guard.as_ref() {
            if let Some((cached_mtime, agg)) = map.get(path) {
                if *cached_mtime == mtime {
                    return Some((mtime, agg.clone()));
                }
            }
        }
    }
    let agg = parse_codex_jsonl(path);
    if let Ok(mut guard) = CODEX_TOKEN_CACHE.lock() {
        guard
            .get_or_insert_with(HashMap::new)
            .insert(path.to_path_buf(), (mtime, agg.clone()));
    }
    Some((mtime, agg))
}

/// Parse one Codex rollout file: keep the LAST token_count totals (cumulative)
/// and the last rate-limit snapshot.
fn parse_codex_jsonl(path: &Path) -> CodexFileAgg {
    let mut agg = CodexFileAgg::default();
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return agg,
    };

    for line in content.lines() {
        let v: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };

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

        if let Some(total_usage) = payload.pointer("/info/total_token_usage") {
            let inp = total_usage
                .get("input_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let cached = total_usage
                .get("cached_input_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let out = total_usage
                .get("output_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let reasoning = total_usage
                .get("reasoning_output_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let reported_total = total_usage.get("total_tokens").and_then(|v| v.as_u64());

            // Cumulative — overwrite with the latest event's running totals.
            agg.input = inp;
            agg.cached = cached;
            agg.output = out + reasoning;
            agg.total = reported_total.unwrap_or(inp + out + reasoning);
        }

        if let Some(rate_limits) = payload.get("rate_limits") {
            if let Some(primary) = rate_limits.get("primary") {
                agg.primary = Some(parse_rate(primary, 300));
            }
            if let Some(secondary) = rate_limits.get("secondary") {
                agg.secondary = Some(parse_rate(secondary, 10080));
            }
        }
    }

    agg
}

fn parse_rate(v: &serde_json::Value, default_minutes: u64) -> RateSnapshotData {
    RateSnapshotData {
        used_percent: v
            .get("used_percent")
            .and_then(|x| x.as_f64())
            .unwrap_or(0.0),
        window_minutes: v
            .get("window_minutes")
            .and_then(|x| x.as_u64())
            .unwrap_or(default_minutes),
        resets_at: v.get("resets_at").and_then(|x| x.as_i64()),
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
        assert!(
            parts.len() >= 2,
            "version should have at least major.minor: {}",
            v
        );
    }

    #[test]
    fn test_codex_desktop_version_format() {
        let version = get_codex_desktop_version();
        if let Some(v) = version {
            // Desktop version is like "26.602.40724"
            assert!(
                v.chars()
                    .next()
                    .map(|c| c.is_ascii_digit())
                    .unwrap_or(false),
                "desktop version should start with a digit: {}",
                v
            );
        }
    }

    #[test]
    fn test_codex_cli_and_desktop_versions_differ() {
        let cli = get_codex_cli_version();
        let desktop = get_codex_desktop_version();
        if let (Some(cv), Some(dv)) = (&cli, &desktop) {
            assert_ne!(
                cv, dv,
                "CLI version ({}) should differ from Desktop version ({})",
                cv, dv
            );
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
        assert!(
            db_path.exists(),
            "state_5.sqlite should exist at {:?}",
            db_path
        );
    }

    #[test]
    fn test_state_db_has_threads_table() {
        let db_path = state_db_path();
        if !db_path.exists() {
            return;
        }
        let conn =
            Connection::open_with_flags(&db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                .unwrap();
        let result = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='threads'",
            [],
            |row| row.get::<_, i32>(0),
        );
        assert!(
            result.unwrap_or(0) > 0,
            "threads table should exist in state_5.sqlite"
        );
    }

    #[test]
    fn test_threads_table_has_source_column() {
        let db_path = state_db_path();
        if !db_path.exists() {
            return;
        }
        let conn =
            Connection::open_with_flags(&db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                .unwrap();
        // Query to check if source column exists
        let result = conn.prepare("SELECT source FROM threads LIMIT 1");
        assert!(result.is_ok(), "threads table should have 'source' column");
    }

    #[test]
    fn test_active_sessions_bounded_by_total() {
        // Active (recent-window) sessions must never exceed all non-archived
        // threads — guards against the old lifetime-count overcount bug.
        let (cli, gui) = count_active_sessions();
        let db = state_db_path();
        if db.exists() {
            if let Ok(conn) =
                Connection::open_with_flags(&db, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            {
                if let Ok(total) =
                    conn.query_row("SELECT COUNT(*) FROM threads WHERE archived = 0", [], |r| {
                        r.get::<_, usize>(0)
                    })
                {
                    assert!(
                        cli + gui <= total,
                        "active {} must be <= total {}",
                        cli + gui,
                        total
                    );
                }
            }
        }
    }
}
