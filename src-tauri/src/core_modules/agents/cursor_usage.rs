//! Cursor usage + rate limits via the same approach as the `cursor-usage` CLI:
//! read auth from Cursor's local `state.vscdb`, then call cursor.com APIs.
//! No browser cookie / Keychain access required.

use crate::models::types::{ModelUsage, RateWindow, TokenUsage};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OpenFlags};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

const USAGE_SUMMARY_URL: &str = "https://cursor.com/api/usage-summary";
const USAGE_EVENTS_URL: &str = "https://cursor.com/api/dashboard/get-filtered-usage-events";
const USER_AGENT: &str = "Mozilla/5.0 (compatible; agent-hub/1.0)";

const DB_KEY_STATSIG_BOOTSTRAP: &str = "workbench.experiments.statsigBootstrap";
const DB_KEY_ACCESS_TOKEN: &str = "cursorAuth/accessToken";

#[derive(Clone)]
struct CursorCredentials {
    user_id: String,
    access_token: String,
}

#[derive(Deserialize)]
struct UsageSummary {
    #[serde(rename = "billingCycleEnd")]
    billing_cycle_end: Option<serde_json::Value>,
    #[serde(rename = "individualUsage")]
    individual_usage: Option<IndividualUsage>,
    #[serde(rename = "autoModelSelectedDisplayMessage")]
    auto_model_message: Option<String>,
    #[serde(rename = "namedModelSelectedDisplayMessage")]
    named_model_message: Option<String>,
}

#[derive(Deserialize)]
struct IndividualUsage {
    plan: Option<UsageBlock>,
    #[serde(rename = "onDemand")]
    on_demand: Option<OnDemandBlock>,
}

#[derive(Deserialize)]
struct UsageBlock {
    used: Option<i64>,
    limit: Option<i64>,
    #[serde(rename = "totalPercentUsed")]
    total_percent_used: Option<f64>,
}

#[derive(Deserialize)]
struct OnDemandBlock {
    used: Option<i64>,
    limit: Option<i64>,
    #[serde(rename = "totalPercentUsed")]
    total_percent_used: Option<f64>,
    enabled: Option<bool>,
}

#[derive(Deserialize)]
struct UsageEventsResponse {
    #[serde(rename = "usageEventsDisplay", default)]
    usage_events_display: Vec<UsageEvent>,
}

#[derive(Deserialize)]
struct UsageEvent {
    model: Option<String>,
    #[serde(rename = "tokenUsage")]
    token_usage: Option<TokenUsageBlock>,
}

#[derive(Deserialize)]
struct TokenUsageBlock {
    #[serde(rename = "inputTokens", default)]
    input_tokens: u64,
    #[serde(rename = "outputTokens", default)]
    output_tokens: u64,
    #[serde(rename = "cacheWriteTokens", default)]
    cache_write_tokens: u64,
    #[serde(rename = "cacheReadTokens", default)]
    cache_read_tokens: u64,
}

#[derive(Default, Clone)]
pub struct CursorUsageSnapshot {
    pub session_window: Option<RateWindow>,
    pub weekly_window: Option<RateWindow>,
    pub tokens: Option<TokenUsage>,
    pub model_breakdowns: Vec<ModelUsage>,
}

type UsageCache = Option<(Instant, CursorUsageSnapshot)>;

static CURSOR_USAGE_CACHE: Mutex<UsageCache> = Mutex::new(None);

fn home() -> PathBuf {
    dirs_next::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

/// `~/Library/Application Support/Cursor/...` on macOS, `~/.config/Cursor/...` on Linux.
fn cursor_state_db_path() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        home().join("Library/Application Support/Cursor/User/globalStorage/state.vscdb")
    }
    #[cfg(target_os = "windows")]
    {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            PathBuf::from(appdata)
                .join("Cursor")
                .join("User")
                .join("globalStorage")
                .join("state.vscdb")
        } else {
            home()
                .join("Cursor")
                .join("User")
                .join("globalStorage")
                .join("state.vscdb")
        }
    }
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        home()
            .join(".config")
            .join("Cursor")
            .join("User")
            .join("globalStorage")
            .join("state.vscdb")
    }
}

fn open_sqlite_snapshot(db: &Path) -> Option<Connection> {
    let tmp = std::env::temp_dir().join("agent-hub-cursor-state.vscdb");
    fs::copy(db, &tmp).ok()?;
    Connection::open_with_flags(&tmp, OpenFlags::SQLITE_OPEN_READ_ONLY).ok()
}

fn query_db_value(conn: &Connection, key: &str) -> Option<String> {
    conn.query_row(
        "SELECT value FROM ItemTable WHERE key = ?1",
        [key],
        |row| row.get(0),
    )
    .ok()
}

fn load_credentials() -> Option<CursorCredentials> {
    let db_path = cursor_state_db_path();
    if !db_path.exists() {
        return None;
    }
    let conn = open_sqlite_snapshot(&db_path)?;

    let bootstrap = query_db_value(&conn, DB_KEY_STATSIG_BOOTSTRAP)?;
    let user_id = serde_json::from_str::<serde_json::Value>(&bootstrap)
        .ok()?
        .get("user")?
        .get("userID")?
        .as_str()?
        .to_string();

    let access_token = query_db_value(&conn, DB_KEY_ACCESS_TOKEN)?;
    if access_token.is_empty() {
        return None;
    }

    Some(CursorCredentials {
        user_id,
        access_token,
    })
}

fn session_cookie(creds: &CursorCredentials) -> String {
    let raw = format!("{}::{}", creds.user_id, creds.access_token);
    urlencoding::encode(&raw).into_owned()
}

fn http_client() -> Option<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .ok()
}

fn fetch_usage_summary(creds: &CursorCredentials) -> Option<UsageSummary> {
    let rt = tokio::runtime::Runtime::new().ok()?;
    let cookie = session_cookie(creds);
    rt.block_on(async {
        let client = http_client()?;
        let r = client
            .get(USAGE_SUMMARY_URL)
            .header(
                "Cookie",
                format!("WorkosCursorSessionToken={}", cookie),
            )
            .header("User-Agent", USER_AGENT)
            .send()
            .await
            .ok()?;
        if !r.status().is_success() {
            return None;
        }
        r.json::<UsageSummary>().await.ok()
    })
}

fn fetch_usage_events(creds: &CursorCredentials, start: DateTime<Utc>, end: DateTime<Utc>) -> Vec<UsageEvent> {
    let rt = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(_) => return vec![],
    };
    let cookie = session_cookie(creds);
    let body = serde_json::json!({
        "teamId": 0,
        "startDate": start.timestamp_millis().to_string(),
        "endDate": end.timestamp_millis().to_string(),
        "page": 1,
        "pageSize": 100,
    });

    rt.block_on(async {
        let client = match http_client() {
            Some(c) => c,
            None => return vec![],
        };
        let r = match client
            .post(USAGE_EVENTS_URL)
            .header(
                "Cookie",
                format!("WorkosCursorSessionToken={}", cookie),
            )
            .header("User-Agent", USER_AGENT)
            .header("Origin", "https://cursor.com")
            .json(&body)
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(_) => return vec![],
        };
        if !r.status().is_success() {
            return vec![];
        }
        r.json::<UsageEventsResponse>()
            .await
            .map(|d| d.usage_events_display)
            .unwrap_or_default()
    })
}

fn percent_from_message(msg: &str) -> Option<f64> {
    // "You've used 15% of your included total usage"
    let start = msg.find("used ")? + 5;
    let rest = &msg[start..];
    let end = rest.find('%')?;
    rest[..end].trim().parse().ok()
}

fn rate_windows_from_summary(summary: &UsageSummary) -> (Option<RateWindow>, Option<RateWindow>) {
    let resets = summary
        .billing_cycle_end
        .as_ref()
        .and_then(parse_reset);
    const MONTH_MINUTES: u64 = 30 * 24 * 60;

    let individual = match summary.individual_usage.as_ref() {
        Some(i) => i,
        None => return (None, None),
    };

    let mk_plan = |block: &UsageBlock, label: &str| -> Option<RateWindow> {
        let pct = block.total_percent_used.or_else(|| match (block.used, block.limit) {
            (Some(u), Some(l)) if l > 0 => Some(u as f64 / l as f64 * 100.0),
            _ => None,
        })?;
        Some(RateWindow {
            used_percent: pct,
            window_minutes: MONTH_MINUTES,
            resets_at: resets.clone(),
            label: Some(label.to_string()),
        })
    };

    let plan = individual
        .plan
        .as_ref()
        .and_then(|b| mk_plan(b, "Plan"));

    let on_demand = individual.on_demand.as_ref().and_then(|b| {
        if b.enabled == Some(false) {
            return None;
        }
        let pct = b.total_percent_used.or_else(|| match (b.used, b.limit) {
            (Some(u), Some(l)) if l > 0 => Some(u as f64 / l as f64 * 100.0),
            _ => None,
        })?;
        Some(RateWindow {
            used_percent: pct,
            window_minutes: MONTH_MINUTES,
            resets_at: resets.clone(),
            label: Some("On-Demand".to_string()),
        })
    });

    // Prefer the auto/named model messages when present — they track the
    // rolling session-style limits Cursor shows in the app.
    let auto_session = summary
        .auto_model_message
        .as_deref()
        .and_then(percent_from_message)
        .map(|pct| RateWindow {
            used_percent: pct,
            window_minutes: 5 * 60,
            resets_at: resets.clone(),
            label: Some("Auto".to_string()),
        });

    let named_weekly = summary
        .named_model_message
        .as_deref()
        .and_then(percent_from_message)
        .map(|pct| RateWindow {
            used_percent: pct,
            window_minutes: 7 * 24 * 60,
            resets_at: resets.clone(),
            label: Some("Named".to_string()),
        });

    (
        auto_session.or(plan),
        named_weekly.or(on_demand),
    )
}

fn aggregate_events(events: &[UsageEvent]) -> (Option<TokenUsage>, Vec<ModelUsage>) {
    if events.is_empty() {
        return (None, vec![]);
    }

    let mut tokens = TokenUsage {
        input_tokens: 0,
        cache_read_tokens: 0,
        cache_create_tokens: 0,
        output_tokens: 0,
        total_tokens: 0,
    };
    let mut models: std::collections::HashMap<String, (u64, u64, usize)> =
        std::collections::HashMap::new();

    for event in events {
        let usage = match event.token_usage.as_ref() {
            Some(u) => u,
            None => continue,
        };
        tokens.input_tokens += usage.input_tokens;
        tokens.output_tokens += usage.output_tokens;
        tokens.cache_read_tokens += usage.cache_read_tokens;
        tokens.cache_create_tokens += usage.cache_write_tokens;
        tokens.total_tokens += usage.input_tokens
            + usage.output_tokens
            + usage.cache_read_tokens
            + usage.cache_write_tokens;

        let model = event
            .model
            .clone()
            .filter(|m| !m.is_empty())
            .unwrap_or_else(|| "unknown".to_string());
        let entry = models.entry(model).or_insert((0, 0, 0));
        entry.0 += usage.input_tokens + usage.cache_read_tokens + usage.cache_write_tokens;
        entry.1 += usage.output_tokens;
        entry.2 += 1;
    }

    if tokens.total_tokens == 0 {
        return (None, vec![]);
    }

    let model_breakdowns = models
        .into_iter()
        .map(|(model, (inp, out, count))| ModelUsage {
            model,
            input_tokens: inp,
            output_tokens: out,
            total_tokens: inp + out,
            request_count: count,
        })
        .collect();

    (Some(tokens), model_breakdowns)
}

fn parse_reset(v: &serde_json::Value) -> Option<String> {
    if let Some(s) = v.as_str() {
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

fn fetch_snapshot_uncached() -> CursorUsageSnapshot {
    let creds = match load_credentials() {
        Some(c) => c,
        None => return CursorUsageSnapshot::default(),
    };

    let summary = fetch_usage_summary(&creds);
    let (session_window, weekly_window) = summary
        .as_ref()
        .map(rate_windows_from_summary)
        .unwrap_or((None, None));

    let end = Utc::now();
    let start = end - chrono::Duration::hours(5);
    let events = fetch_usage_events(&creds, start, end);
    let (tokens, model_breakdowns) = aggregate_events(&events);

    CursorUsageSnapshot {
        session_window,
        weekly_window,
        tokens,
        model_breakdowns,
    }
}

/// Cached Cursor usage snapshot (60s), using local state.vscdb auth.
pub fn fetch_snapshot() -> CursorUsageSnapshot {
    if let Ok(guard) = CURSOR_USAGE_CACHE.lock() {
        if let Some((t, snap)) = guard.as_ref() {
            if t.elapsed() < Duration::from_secs(60) {
                return snap.clone();
            }
        }
    }

    let snap = fetch_snapshot_uncached();
    if let Ok(mut guard) = CURSOR_USAGE_CACHE.lock() {
        *guard = Some((Instant::now(), snap.clone()));
    }
    snap
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_from_message_parses_cursor_copy() {
        let pct = percent_from_message("You've used 15% of your included total usage");
        assert_eq!(pct, Some(15.0));
    }

    #[test]
    fn cursor_state_db_path_is_non_empty() {
        let p = cursor_state_db_path();
        assert!(!p.as_os_str().is_empty());
    }
}
