use crate::models::types::{
    AccountInfo, AgentInfo, AgentType, AgentUsage, RateWindow, Session, UsageStats,
};
use chrono::{DateTime, Utc};
use rusqlite::Connection;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

fn home() -> PathBuf {
    dirs_next::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

fn cursor_dir() -> PathBuf {
    home().join(".cursor")
}

fn tracking_db_path() -> PathBuf {
    cursor_dir().join("ai-tracking/ai-code-tracking.db")
}

/// Candidate locations for the Cursor app (varies by install method).
fn cursor_app_candidates() -> Vec<PathBuf> {
    vec![
        PathBuf::from("/Applications/Cursor.app"),
        home().join("Applications/Cursor.app"),
        home().join("Apps/Cursor.app"),
    ]
}

fn cursor_app_path() -> Option<PathBuf> {
    cursor_app_candidates().into_iter().find(|p| p.exists())
}

/// Detect Cursor installation and running state
pub fn detect() -> AgentInfo {
    let app_path = cursor_app_path();
    let installed = app_path.is_some() || cursor_dir().exists();
    let running = is_cursor_running();

    // Count recent sessions from DB as GUI sessions
    let gui_sessions = if running {
        count_active_conversations()
    } else {
        0
    };

    let cli_version = get_cursor_cli_version();
    let gui_version = get_cursor_gui_version();
    let version = match (&cli_version, &gui_version) {
        (Some(cv), Some(gv)) => Some(format!("CLI {} / GUI {}", cv, gv)),
        (Some(cv), None) => Some(format!("CLI v{}", cv)),
        (None, Some(gv)) => Some(format!("GUI v{}", gv)),
        _ => None,
    };

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
        install_path: app_path.as_ref().map(|p| p.to_string_lossy().to_string()),
        account: get_account(),
    }
}

/// Logged-in account from ~/.cursor/cli-config.json `authInfo`.
fn get_account() -> Option<AccountInfo> {
    let data = fs::read_to_string(cursor_dir().join("cli-config.json")).ok()?;
    let v: serde_json::Value = serde_json::from_str(&data).ok()?;
    let info = v.get("authInfo")?;
    let s = |k: &str| info.get(k).and_then(|x| x.as_str()).map(str::to_string);
    let account = AccountInfo {
        email: s("email"),
        display_name: s("displayName"),
        organization: None,
    };
    if account.email.is_none() && account.display_name.is_none() {
        None
    } else {
        Some(account)
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
        return 0;
    }
    let conn =
        match Connection::open_with_flags(&db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY) {
            Ok(c) => c,
            Err(_) => return 0,
        };
    // conversation_summaries has `updatedAt` (epoch millis), not `timestamp`.
    // Count conversations active in the last hour. Don't fabricate a count on
    // failure — return 0 rather than a fake "1".
    let cutoff = (Utc::now() - chrono::Duration::hours(1)).timestamp_millis();
    conn.query_row(
        "SELECT COUNT(*) FROM conversation_summaries WHERE updatedAt >= ?1",
        [cutoff],
        |row| row.get(0),
    )
    .unwrap_or(0)
}

fn get_cursor_gui_version() -> Option<String> {
    let app = cursor_app_path()?;
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

fn get_cursor_cli_version() -> Option<String> {
    // Try cursor-agent first (the CLI/TUI variant)
    // Try full path, then PATH lookup, retry up to 2 times for flaky symlink
    let candidate = home().join(".local/bin/cursor-agent");
    let attempts: Vec<String> = if candidate.exists() {
        vec![
            candidate.to_string_lossy().to_string(),
            "cursor-agent".to_string(),
            "cursor-agent".to_string(),
        ]
    } else {
        vec!["cursor-agent".to_string(), "cursor-agent".to_string()]
    };

    for cmd in &attempts {
        if let Ok(output) = std::process::Command::new(cmd).arg("--version").output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let first_line = stdout.lines().next().unwrap_or("").trim();
                if !first_line.is_empty() {
                    return Some(first_line.to_string());
                }
            }
        }
    }

    // Do NOT fall back to `cursor --version` — that returns the GUI version (e.g. "3.7.12"),
    // not the CLI version (e.g. "2026.06.04-5fd875e"). Return None instead.
    None
}

/// Get active Cursor sessions (from conversation_summaries if DB accessible)
pub fn get_sessions() -> Vec<Session> {
    let db_path = tracking_db_path();
    if !db_path.exists() {
        return vec![];
    }

    let conn =
        match Connection::open_with_flags(&db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY) {
            Ok(c) => c,
            Err(_) => return vec![],
        };

    // Real columns are conversationId, model, mode, updatedAt (epoch millis).
    // There is no `source`/`timestamp` column — the old query errored and the
    // sessions list was always empty.
    let mut stmt = match conn.prepare(
        "SELECT conversationId, model, mode, updatedAt FROM conversation_summaries ORDER BY updatedAt DESC LIMIT 30",
    ) {
        Ok(s) => s,
        Err(_) => return vec![],
    };

    let sessions: Vec<Session> = stmt
        .query_map([], |row| {
            let id: String = row.get(0).unwrap_or_default();
            let model: String = row.get(1).unwrap_or_default();
            let mode: String = row.get(2).unwrap_or_default();
            let ts: i64 = row.get(3).unwrap_or(0);

            let started = DateTime::from_timestamp_millis(ts)
                .map(|d| d.to_rfc3339())
                .unwrap_or_default();

            Ok(Session {
                id,
                agent: "Cursor".to_string(),
                status: "completed".to_string(),
                started_at: Some(started),
                working_dir: None,
                model: if model.is_empty() { None } else { Some(model) },
                pid: None,
                entrypoint: mode,
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
    let cutoff_ts =
        (Utc::now().timestamp_millis() as u64).saturating_sub(window_secs * 1000) as i64;

    let mut total = 0usize;
    let mut conversations = std::collections::HashSet::new();

    if db_path.exists() {
        if let Ok(conn) =
            Connection::open_with_flags(&db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        {
            if let Ok(mut stmt) =
                conn.prepare("SELECT conversationId FROM ai_code_hashes WHERE timestamp >= ?1")
            {
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

/// Get rich usage data for Cursor via cursor.com APIs (state.vscdb auth).
pub fn get_rich_usage() -> AgentUsage {
    let db_path = tracking_db_path();
    let mut total_sessions = 0usize;
    let mut total_interactions = 0usize;

    if db_path.exists() {
        if let Ok(conn) =
            Connection::open_with_flags(&db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        {
            if let Ok(count) = conn.query_row(
                "SELECT COUNT(DISTINCT conversationId) FROM conversation_summaries",
                [],
                |row| row.get::<_, usize>(0),
            ) {
                total_sessions = count;
            }
            if let Ok(count) = conn.query_row("SELECT COUNT(*) FROM ai_code_hashes", [], |row| {
                row.get::<_, usize>(0)
            }) {
                total_interactions = count;
            }
        }
    }

    // Primary path: local state.vscdb credentials (same as `cursor-usage` CLI).
    let api = super::cursor_usage::fetch_snapshot();
    let (session_window, weekly_window, extra_rate_windows) =
        if api.session_window.is_some()
            || api.weekly_window.is_some()
            || !api.extra_rate_windows.is_empty()
        {
            (
                api.session_window,
                api.weekly_window,
                api.extra_rate_windows,
            )
        } else {
            let (s, w) = fetch_cursor_rate_limits();
            (s, w, vec![])
        };

    AgentUsage {
        agent: "Cursor".to_string(),
        session_window,
        weekly_window,
        extra_rate_windows,
        tokens: api.tokens,
        model_breakdowns: api.model_breakdowns,
        total_interactions,
        total_sessions,
    }
}

// ─── Rate limits (cursor.com usage API via browser session cookie) ───────────

#[derive(Deserialize)]
struct UsageSummary {
    #[serde(rename = "billingCycleEnd")]
    billing_cycle_end: Option<serde_json::Value>,
    #[serde(rename = "individualUsage")]
    individual_usage: Option<IndividualUsage>,
}

#[derive(Deserialize)]
struct IndividualUsage {
    plan: Option<UsageBlock>,
}

#[derive(Deserialize)]
struct UsageBlock {
    used: Option<i64>,
    limit: Option<i64>,
    #[serde(rename = "totalPercentUsed")]
    total_percent_used: Option<f64>,
    #[serde(rename = "apiPercentUsed")]
    api_percent_used: Option<f64>,
}

// Network call — cache 60s (get_rich_usage runs every few seconds on the tab).
type RateLimitCache = Option<(Instant, Option<RateWindow>, Option<RateWindow>)>;

static CURSOR_RL_CACHE: Mutex<RateLimitCache> = Mutex::new(None);

fn fetch_cursor_rate_limits() -> (Option<RateWindow>, Option<RateWindow>) {
    // Off by default: the only way to read Cursor's server-side limits is its
    // browser session cookie, and decrypting Chrome's cookie store triggers a
    // macOS Keychain prompt. Don't scan browsers / prompt unless explicitly
    // opted in. Enable with AGENT_HUB_CURSOR_USAGE=1.
    match std::env::var("AGENT_HUB_CURSOR_USAGE").as_deref() {
        Ok("1") | Ok("true") => {}
        _ => return (None, None),
    }

    if let Ok(guard) = CURSOR_RL_CACHE.lock() {
        if let Some((t, s, w)) = guard.as_ref() {
            if t.elapsed() < Duration::from_secs(60) {
                return (s.clone(), w.clone());
            }
        }
    }
    let result = fetch_cursor_rate_limits_uncached();
    if let Ok(mut guard) = CURSOR_RL_CACHE.lock() {
        *guard = Some((Instant::now(), result.0.clone(), result.1.clone()));
    }
    result
}

fn fetch_cursor_rate_limits_uncached() -> (Option<RateWindow>, Option<RateWindow>) {
    let cookie = match super::cursor_cookies::find_cursor_session_cookie() {
        Some(c) => c,
        None => return (None, None),
    };
    let rt = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(_) => return (None, None),
    };
    let summary: Option<UsageSummary> = rt.block_on(async {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .ok()?;
        let r = client
            .get("https://cursor.com/api/usage-summary")
            .header("Cookie", format!("WorkosCursorSessionToken={}", cookie))
            .header("User-Agent", "agent-hub")
            .send()
            .await
            .ok()?;
        if !r.status().is_success() {
            return None;
        }
        r.json::<UsageSummary>().await.ok()
    });

    let summary = match summary {
        Some(s) => s,
        None => return (None, None),
    };
    let resets = summary
        .billing_cycle_end
        .as_ref()
        .and_then(parse_cursor_reset);
    let individual = match summary.individual_usage {
        Some(i) => i,
        None => return (None, None),
    };

    // Cursor bills monthly; show Total (Auto+Composer+API) and API (named models).
    const MONTH_MINUTES: u64 = 30 * 24 * 60;
    let mk = |pct: f64, label: &str| RateWindow {
        used_percent: pct,
        window_minutes: MONTH_MINUTES,
        resets_at: resets.clone(),
        label: Some(label.to_string()),
        is_remaining: false,
    };

    let plan = match individual.plan {
        Some(p) => p,
        None => return (None, None),
    };
    let pct_from = |field: Option<f64>| {
        field.or_else(|| match (plan.used, plan.limit) {
            (Some(u), Some(l)) if l > 0 => Some(u as f64 / l as f64 * 100.0),
            _ => None,
        })
    };

    let total = pct_from(plan.total_percent_used).map(|pct| mk(pct, "Total"));
    let api = plan.api_percent_used.map(|pct| mk(pct, "API"));
    (total, api)
}

/// billingCycleEnd may be an ISO string or epoch millis (number or string).
fn parse_cursor_reset(v: &serde_json::Value) -> Option<String> {
    if let Some(s) = v.as_str() {
        // epoch-millis-as-string?
        if let Ok(ms) = s.parse::<i64>() {
            return DateTime::from_timestamp_millis(ms).map(|d| d.to_rfc3339());
        }
        return Some(s.to_string());
    }
    if let Some(ms) = v.as_i64() {
        return DateTime::from_timestamp_millis(ms).map(|d| d.to_rfc3339());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cursor_agent_version_is_not_gui_version() {
        // Whatever the CLI reports, it must never be the GUI semver (the bug we fixed).
        if let Some(v) = get_cursor_cli_version() {
            assert_ne!(v, "3.7.12", "CLI version must not be the GUI version");
        }
    }

    #[test]
    fn test_cursor_agent_version_format() {
        let version = get_cursor_cli_version();
        if let Some(v) = version {
            // Should be a date-based version like "2026.06.04-5fd875e"
            assert!(v.contains('.'), "version should contain dots: {}", v);
            assert!(
                !v.contains('\n'),
                "version should not contain newlines: {}",
                v
            );
            // Should NOT be the GUI version "3.7.12"
            assert_ne!(v, "3.7.12", "CLI version should not be the GUI version");
        }
        // If cursor-agent is not installed, None is acceptable
    }

    #[test]
    fn test_cursor_gui_version_format() {
        let version = get_cursor_gui_version();
        if let Some(v) = version {
            // GUI version should be like "3.7.12"
            assert!(
                v.chars()
                    .next()
                    .map(|c| c.is_ascii_digit())
                    .unwrap_or(false),
                "GUI version should start with a digit: {}",
                v
            );
        }
    }

    #[test]
    fn test_cursor_cli_version_is_date_based() {
        let cli = get_cursor_cli_version();
        if let Some(v) = cli {
            // CLI version should be date-based like "2026.06.04-5fd875e", not semver like "3.7.12"
            assert!(
                v.contains('-') && v.contains('.'),
                "CLI version should be date-based (YYYY.MM.DD-hash), got: {}",
                v
            );
            // Should NOT be the GUI semver version
            assert_ne!(
                v, "3.7.12",
                "CLI version should not be the GUI version 3.7.12, got: {}",
                v
            );
        }
        // If cursor-agent is not installed, None is acceptable
    }

    #[test]
    fn test_detect_returns_correct_fields() {
        let info = detect();
        assert_eq!(info.name, "Cursor");
        assert_eq!(info.agent_type, AgentType::Cursor);
        // CLI version should not be the GUI version
        if let Some(cv) = &info.cli_version {
            assert_ne!(cv, "3.7.12", "cli_version should not be GUI version 3.7.12");
        }
        // GUI version should be available wherever Cursor.app is installed
        if cursor_app_path().is_some() {
            assert!(
                info.gui_version.is_some(),
                "GUI version should be set when Cursor.app exists"
            );
        }
    }

    #[test]
    fn test_detect_installed_consistent_with_paths() {
        // detect() should report installed iff Cursor.app OR ~/.cursor exists —
        // not assert a specific machine has Cursor at a hardcoded path.
        let app = PathBuf::from("/Applications/Cursor.app");
        let expected = app.exists() || cursor_dir().exists();
        assert_eq!(detect().installed, expected);
    }
}
