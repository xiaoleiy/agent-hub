use crate::models::types::{
    AccountInfo, AgentInfo, AgentType, AgentUsage, ModelUsage, Session, TokenUsage, UsageStats,
};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OpenFlags};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::SystemTime;

fn home() -> PathBuf {
    dirs_next::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

/// OpenCode keeps everything in one SQLite DB under the XDG data dir.
fn data_dir() -> PathBuf {
    home().join(".local/share/opencode")
}

fn db_path() -> PathBuf {
    data_dir().join("opencode.db")
}

/// Candidate locations for the `opencode` CLI binary (varies by install method).
fn opencode_bin() -> Option<PathBuf> {
    [
        PathBuf::from("/opt/homebrew/bin/opencode"),
        PathBuf::from("/usr/local/bin/opencode"),
        home().join(".opencode/bin/opencode"),
        home().join(".local/bin/opencode"),
    ]
    .into_iter()
    .find(|p| p.exists())
}

fn open_db() -> Option<Connection> {
    let path = db_path();
    if !path.exists() {
        return None;
    }
    Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).ok()
}

/// Detect OpenCode installation and running state.
pub fn detect() -> AgentInfo {
    let bin = opencode_bin();
    // OpenCode is CLI/TUI-only (no desktop app). It's "installed" if either the
    // binary or its data dir/DB exists — the data dir survives even when the
    // binary isn't on a non-interactive PATH.
    let installed = bin.is_some() || db_path().exists() || data_dir().exists();

    // OpenCode has no GUI app, so every session is a CLI/TUI session.
    let cli_sessions = count_active_sessions();
    let running = cli_sessions > 0 || is_opencode_running();

    let version = get_cli_version();

    AgentInfo {
        name: "OpenCode".to_string(),
        agent_type: AgentType::OpenCode,
        installed,
        running,
        active_sessions: cli_sessions,
        cli_sessions,
        gui_sessions: 0,
        version: version.clone(),
        cli_version: version,
        gui_version: None,
        install_path: bin
            .map(|p| p.to_string_lossy().to_string())
            .or_else(|| Some(data_dir().to_string_lossy().to_string())),
        account: get_account(),
    }
}

/// Logged-in account from the `account` table (the opencode cloud account).
/// Most installs auth via per-provider API keys instead, so this is best-effort
/// and commonly returns None.
fn get_account() -> Option<AccountInfo> {
    let conn = open_db()?;
    // Prefer the active account; fall back to any account row.
    let email: Option<String> = conn
        .query_row(
            "SELECT a.email FROM account a
             JOIN account_state s ON s.active_account_id = a.id
             LIMIT 1",
            [],
            |row| row.get(0),
        )
        .ok()
        .or_else(|| {
            conn.query_row("SELECT email FROM account LIMIT 1", [], |row| row.get(0))
                .ok()
        });
    let email = email?;
    if email.is_empty() {
        return None;
    }
    Some(AccountInfo {
        email: Some(email),
        display_name: None,
        organization: None,
    })
}

fn is_opencode_running() -> bool {
    // -x matches the exact process name "opencode" (the TUI/server process),
    // avoiding false positives from unrelated processes that merely mention it.
    std::process::Command::new("pgrep")
        .arg("-x")
        .arg("opencode")
        .output()
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false)
}

const ACTIVE_WINDOW_MS: i64 = 15 * 60 * 1000;

/// Count sessions touched within the recent window. `time_updated` is epoch
/// MILLISECONDS, so compare against a millisecond cutoff.
fn count_active_sessions() -> usize {
    let Some(conn) = open_db() else {
        return 0;
    };
    let cutoff = Utc::now().timestamp_millis() - ACTIVE_WINDOW_MS;
    conn.query_row(
        "SELECT COUNT(*) FROM session
         WHERE time_archived IS NULL AND time_updated >= ?1",
        [cutoff],
        |row| row.get::<_, usize>(0),
    )
    .unwrap_or(0)
}

/// Installed CLI version. Prefer `opencode --version` (prints e.g. "1.16.2");
/// fall back to the newest session's `version` column (historical — the version
/// that created the session, e.g. "1.4.10").
fn get_cli_version() -> Option<String> {
    if let Ok(output) = std::process::Command::new("opencode")
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
    let conn = open_db()?;
    conn.query_row(
        "SELECT version FROM session WHERE version != '' ORDER BY time_updated DESC LIMIT 1",
        [],
        |row| row.get::<_, String>(0),
    )
    .ok()
    .filter(|v| !v.is_empty())
}

/// Most-recent model used in a session (from its newest assistant message).
fn session_model(conn: &Connection, session_id: &str) -> Option<String> {
    let data: String = conn
        .query_row(
            "SELECT data FROM message
             WHERE session_id = ?1 AND data LIKE '%\"role\":\"assistant\"%'
             ORDER BY time_created DESC LIMIT 1",
            [session_id],
            |row| row.get(0),
        )
        .ok()?;
    let v: serde_json::Value = serde_json::from_str(&data).ok()?;
    let model = v.get("modelID").and_then(|x| x.as_str())?;
    match v.get("providerID").and_then(|x| x.as_str()) {
        Some(p) if !p.is_empty() => Some(format!("{}/{}", p, model)),
        _ => Some(model.to_string()),
    }
}

/// Active (non-archived) OpenCode sessions, newest first.
pub fn get_sessions() -> Vec<Session> {
    let Some(conn) = open_db() else {
        return vec![];
    };
    let now = Utc::now().timestamp_millis();
    let mut stmt = match conn.prepare(
        "SELECT id, title, directory, time_updated
         FROM session WHERE time_archived IS NULL
         ORDER BY time_updated DESC LIMIT 20",
    ) {
        Ok(s) => s,
        Err(_) => return vec![],
    };

    let rows: Vec<(String, String, String, i64)> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0).unwrap_or_default(),
                row.get::<_, String>(1).unwrap_or_default(),
                row.get::<_, String>(2).unwrap_or_default(),
                row.get::<_, i64>(3).unwrap_or(0),
            ))
        })
        .map(|r| r.filter_map(|x| x.ok()).collect())
        .unwrap_or_default();

    rows.into_iter()
        .map(|(id, title, dir, updated_ms)| {
            let model = session_model(&conn, &id);
            let started = DateTime::from_timestamp_millis(updated_ms).map(|d| d.to_rfc3339());
            let status = if now - updated_ms <= ACTIVE_WINDOW_MS {
                "active"
            } else {
                "idle"
            };
            Session {
                id: if title.is_empty() { id } else { title },
                agent: "OpenCode".to_string(),
                status: status.to_string(),
                started_at: started,
                working_dir: if dir.is_empty() { None } else { Some(dir) },
                model,
                pid: None,
                entrypoint: "cli".to_string(),
            }
        })
        .collect()
}

fn window_ms(window: &str) -> i64 {
    let secs: i64 = match window {
        "5h" => 5 * 3600,
        "1w" => 7 * 86400,
        "1m" => 30 * 86400,
        _ => 5 * 3600,
    };
    secs * 1000
}

/// Simple windowed usage. `total_interactions` is a COUNT (assistant messages in
/// the window) so the cross-agent dashboard chart stays comparable — never a
/// token sum.
pub fn get_usage(window: &str) -> UsageStats {
    let mut total_sessions = 0usize;
    let mut total_interactions = 0usize;

    if let Some(conn) = open_db() {
        let cutoff = Utc::now().timestamp_millis() - window_ms(window);
        total_sessions = conn
            .query_row(
                "SELECT COUNT(*) FROM session
                 WHERE time_archived IS NULL AND time_updated >= ?1",
                [cutoff],
                |row| row.get::<_, usize>(0),
            )
            .unwrap_or(0);
        total_interactions = conn
            .query_row(
                "SELECT COUNT(*) FROM message
                 WHERE time_created >= ?1 AND data LIKE '%\"role\":\"assistant\"%'",
                [cutoff],
                |row| row.get::<_, usize>(0),
            )
            .unwrap_or(0);
    }

    UsageStats {
        agent: "OpenCode".to_string(),
        window: window.to_string(),
        total_sessions,
        total_interactions,
        first_activity: None,
        last_activity: None,
    }
}

/// Rich usage: sum per-message token usage (OpenCode reports per-call totals, so
/// summing is correct — unlike Codex's cumulative running total). Cached by DB
/// mtime so we only re-scan when the database changes.
pub fn get_rich_usage() -> AgentUsage {
    let agg = aggregate_tokens();

    let total_sessions = open_db()
        .and_then(|conn| {
            conn.query_row(
                "SELECT COUNT(*) FROM session WHERE time_archived IS NULL",
                [],
                |row| row.get::<_, usize>(0),
            )
            .ok()
        })
        .unwrap_or(0);

    let has_tokens = agg.total > 0;
    let tokens = TokenUsage {
        input_tokens: agg.input,
        cache_read_tokens: agg.cache_read,
        cache_create_tokens: agg.cache_write,
        output_tokens: agg.output,
        total_tokens: agg.total,
    };

    let mut model_breakdowns: Vec<ModelUsage> = agg
        .models
        .into_iter()
        .map(|(model, m)| ModelUsage {
            model,
            input_tokens: m.input + m.cache_read + m.cache_write,
            output_tokens: m.output,
            total_tokens: m.total,
            request_count: m.requests,
        })
        .collect();
    model_breakdowns.sort_by_key(|m| std::cmp::Reverse(m.total_tokens));

    AgentUsage {
        agent: "OpenCode".to_string(),
        // OpenCode is bring-your-own-provider with no unified rate-limit API.
        session_window: None,
        weekly_window: None,
        tokens: if has_tokens { Some(tokens) } else { None },
        model_breakdowns,
        total_interactions: agg.assistant_msgs,
        total_sessions,
    }
}

#[derive(Clone, Default)]
struct ModelAgg {
    input: u64,
    output: u64,
    cache_read: u64,
    cache_write: u64,
    total: u64,
    requests: usize,
}

#[derive(Clone, Default)]
struct OpencodeAgg {
    input: u64,
    output: u64,
    cache_read: u64,
    cache_write: u64,
    total: u64,
    assistant_msgs: usize,
    models: HashMap<String, ModelAgg>,
}

// Re-scanning every assistant message each refresh is wasteful; cache the whole
// aggregate keyed by the DB file mtime and only re-scan when it changes.
static OPENCODE_TOKEN_CACHE: Mutex<Option<(SystemTime, OpencodeAgg)>> = Mutex::new(None);

fn db_mtime() -> Option<SystemTime> {
    std::fs::metadata(db_path()).ok()?.modified().ok()
}

fn aggregate_tokens() -> OpencodeAgg {
    let mtime = db_mtime();
    if let (Some(mtime), Ok(guard)) = (mtime, OPENCODE_TOKEN_CACHE.lock()) {
        if let Some((cached_mtime, agg)) = guard.as_ref() {
            if *cached_mtime == mtime {
                return agg.clone();
            }
        }
    }

    let agg = scan_tokens();
    if let (Some(mtime), Ok(mut guard)) = (mtime, OPENCODE_TOKEN_CACHE.lock()) {
        *guard = Some((mtime, agg.clone()));
    }
    agg
}

fn scan_tokens() -> OpencodeAgg {
    let mut agg = OpencodeAgg::default();
    let Some(conn) = open_db() else {
        return agg;
    };
    let mut stmt =
        match conn.prepare("SELECT data FROM message WHERE data LIKE '%\"role\":\"assistant\"%'") {
            Ok(s) => s,
            Err(_) => return agg,
        };
    let rows = match stmt.query_map([], |row| row.get::<_, String>(0)) {
        Ok(r) => r,
        Err(_) => return agg,
    };

    for data in rows.flatten() {
        let v: serde_json::Value = match serde_json::from_str(&data) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if v.get("role").and_then(|r| r.as_str()) != Some("assistant") {
            continue;
        }
        let u = |ptr: &str| v.pointer(ptr).and_then(|x| x.as_u64()).unwrap_or(0);
        let input = u("/tokens/input");
        let output = u("/tokens/output") + u("/tokens/reasoning");
        let cache_read = u("/tokens/cache/read");
        let cache_write = u("/tokens/cache/write");
        let total = v
            .pointer("/tokens/total")
            .and_then(|x| x.as_u64())
            .unwrap_or(input + output + cache_read + cache_write);

        if total == 0 {
            continue;
        }

        agg.input += input;
        agg.output += output;
        agg.cache_read += cache_read;
        agg.cache_write += cache_write;
        agg.total += total;
        agg.assistant_msgs += 1;

        let model = {
            let m = v
                .get("modelID")
                .and_then(|x| x.as_str())
                .unwrap_or("unknown");
            match v.get("providerID").and_then(|x| x.as_str()) {
                Some(p) if !p.is_empty() => format!("{}/{}", p, m),
                _ => m.to_string(),
            }
        };
        let e = agg.models.entry(model).or_default();
        e.input += input;
        e.output += output;
        e.cache_read += cache_read;
        e.cache_write += cache_write;
        e.total += total;
        e.requests += 1;
    }

    agg
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_dir_shape() {
        assert!(data_dir().ends_with("opencode"));
        assert!(db_path().ends_with("opencode.db"));
    }

    #[test]
    fn test_detect_returns_correct_fields() {
        let info = detect();
        assert_eq!(info.name, "OpenCode");
        assert_eq!(info.agent_type, AgentType::OpenCode);
        // OpenCode has no GUI app.
        assert_eq!(info.gui_sessions, 0);
        assert!(info.gui_version.is_none());
    }

    #[test]
    fn test_readers_do_not_panic() {
        // DB may be absent on a clean machine (e.g. CI) — readers must be safe.
        let _ = get_sessions();
        let _ = get_usage("5h");
        let _ = get_rich_usage();
        let _ = count_active_sessions();
    }

    #[test]
    fn test_cli_version_format() {
        let Some(v) = get_cli_version() else {
            return; // not installed
        };
        assert!(
            v.chars()
                .next()
                .map(|c| c.is_ascii_digit())
                .unwrap_or(false),
            "version should start with a digit: {}",
            v
        );
    }

    #[test]
    fn test_active_sessions_bounded_by_total() {
        let active = count_active_sessions();
        if let Some(conn) = open_db() {
            if let Ok(total) = conn.query_row(
                "SELECT COUNT(*) FROM session WHERE time_archived IS NULL",
                [],
                |r| r.get::<_, usize>(0),
            ) {
                assert!(
                    active <= total,
                    "active {} must be <= total {}",
                    active,
                    total
                );
            }
        }
    }

    #[test]
    fn test_schema_has_expected_tables() {
        let Some(conn) = open_db() else {
            return;
        };
        for table in ["session", "message", "part"] {
            let exists: i32 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                    [table],
                    |r| r.get(0),
                )
                .unwrap_or(0);
            assert!(exists > 0, "table {} should exist", table);
        }
    }
}
