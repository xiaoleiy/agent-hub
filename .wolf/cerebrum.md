# Cerebrum

> OpenWolf's learning memory. Updated automatically as the AI learns from interactions.
> Do not edit manually unless correcting an error.
> Last updated: 2026-06-07

## User Preferences

- **Dual UI parity:** When adding features to the GUI, always add the same to the TUI. The user expects both interfaces to stay in sync.
- **UTF-8 safety:** All string slicing in Rust must use char-based operations (`chars().skip().collect()`), never byte indexing (`&s[byte_range]`). The codebase has paths with Chinese characters.
- **Defensive coding in TUI:** Always clamp `active_tab` after data refreshes that may change the number of tabs. Use `unwrap_or_default()` for optional session fields.
- **Tab-based agent view:** Both GUI and TUI use per-agent tabs (not flat sections). Each tab shows agent header, usage (5h + 1w), and active sessions.
- **Menu bar app:** The app should live in the macOS system tray. Clicking the tray icon toggles window visibility. Closing the window hides to tray, doesn't quit.

## Key Learnings

- **Project:** agent-hub — Manage AI coding agents on macOS. Tauri 2 + Svelte 5 + ratatui.
- **Agent detection:** Claude Code (~/.claude/sessions), Cursor (~/.cursor/ai-tracking SQLite), Codex (~/.codex/state_5.sqlite). Each has separate CLI/GUI version detection.
- **Cursor CLI version bug:** `cursor --version` returns the GUI version (e.g. "3.7.12"), not the CLI version. Must use `cursor-agent --version` only. Do NOT fall back to `cursor --version`.
- **Proxy/VPN detection on macOS:**
  - System proxy: `networksetup -getwebproxy/-getsecurewebproxy/-getsocksfirewallproxy "ServiceName"`
  - VPN: `scutil --nc list` (parse connected/disconnected status)
  - Proxy clients: Try Clash API (ports 9090, 9097), Surge API (port 6171), fallback to `pgrep`
  - Clash/Mihomo API: `GET /proxies` returns groups with `now` (selected node) and `history` (latency)
  - Reference projects: codexbar (Swift), clash-verge-rev (Tauri+Rust), sysproxy-rs crate
- **Tauri 2 tray icon:** Use `.setup()` with `TrayIconBuilder`, `.on_tray_icon_event()` for click handling, `WindowEvent::CloseRequested` with `api.prevent_close()` to hide-to-tray.
- **ratatui Tabs widget:** Passes `active_tab` index to `.select()`. Panics if index >= tab count. Always clamp after data changes.
- **UTF-8 panics in Rust:** `&str[byte_range]` panics if the range splits a multi-byte char. Use `chars().skip(n).collect::<String>()` instead. Common with Chinese/Japanese in file paths.
- **Run modes (lib.rs router):** no args / `gui` → Tauri GUI; any other arg → clap CLI; `tui` subcommand → `crate::tui::run_tui()` (ratatui interactive dashboard, aka the "CUI"). Run from source: `cargo run --manifest-path src-tauri/Cargo.toml -- tui` (or `-- status` for one-shot CLI).
- **TUI tab model:** `tab_names()` is the single source of truth — `[Dashboard, ...installed agents, Proxy/VPN, Keep-Alive]`. `tab_count()` MUST equal `tab_names().len()` (now does). Any separate arithmetic count drifts and makes a tab unreachable (was bug-008). Active-tab→data mapping lives in `active_tab_kind()` → `TabKind`.
- **TUI perf model:** Dashboard data loads at startup; Proxy (`proxy::get_proxy_info` is slow — subprocess spawns + 7 localhost HTTP probes each making its own tokio runtime + pgrep) and per-agent detail (sessions + rich usage) are LAZY — loaded only when their tab is first viewed (`ensure_active_tab_loaded`) and cached (`proxy: Option`, `agent_detail_loaded: Vec<bool>`). The 3s `tick_refresh()` refreshes only system stats + the ACTIVE tab — never all tabs. Don't reintroduce a monolithic `refresh()` that loads everything every tick; it blocks the single-threaded event loop and makes key handling laggy.
- **system::get_system_status:** uses `System::new()` + `refresh_cpu_usage()` (x2 with 200ms gap) + `refresh_memory()`, NOT `new_all()`/`refresh_all()` — output only needs CPU+RAM+net+uptime+user/host, so enumerating processes is wasted cost. sysinfo 0.32: `refresh_cpu_usage()` does enumerate CPUs (cpu_cores stays correct).
- **VERIFIED real agent data schemas (inspected on-machine 2026-06-08):**
  - **Claude sessions** `~/.claude/sessions/<pid>.json` — keys are **camelCase**: `pid, sessionId, cwd, startedAt(ms), version, status, entrypoint, kind`. Rust structs MUST use `#[serde(rename_all="camelCase")]` or session_id/started_at silently default to empty. `~/.claude/history.jsonl` lines: `{display, timestamp(ms epoch), project, sessionId}` — also camelCase. Per-message token usage in `~/.claude/projects/**/*.jsonl` (`type:"assistant"`, `/message/usage`) is PER-CALL (not cumulative) — summing is correct for Claude.
  - **Codex** `~/.codex/state_5.sqlite` table `threads`: `id, updated_at(INTEGER epoch SECONDS), created_at, source, model_provider, model, cwd, title, tokens_used, archived, updated_at_ms, ...`. NEVER compare updated_at against a date string (SQLite numeric affinity coerces it to `2026` → filter no-op); compare integer epoch. Token data in `~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl`: `token_count` event_msg, `payload/info/total_token_usage` is **CUMULATIVE per session** — take the LAST event per file, do NOT sum events (was 2-3x over-count). `payload/rate_limits/{primary,secondary}` has `used_percent, window_minutes, resets_at(epoch secs)`. Codex.app may live at `~/Apps/Codex.app`, not `/Applications` — check multiple candidates.
  - **Cursor** `~/.cursor/ai-tracking/ai-code-tracking.db`: `conversation_summaries(conversationId, title, model, mode, updatedAt(ms))` — NO `source`/`timestamp` columns (often empty table); `ai_code_hashes(hash, source, conversationId, timestamp(ms), model)` is the populated one. Cursor.app may be absent (CLI-only install via `~/.cursor` + cursor-agent).
- **VERIFIED version sources (2026-06-08) — DO NOT trust hardcoded paths or version.json:**
  - **Claude CLI version:** the running CLI session's `version` field (in `~/.claude/sessions/<pid>.json`, entrypoint cli/sdk-cli) is the authoritative live version (PATH-independent). `~/.local/bin/claude` is a symlink that can point at an OLDER installed version (was 2.1.139 while real `claude` on PATH + running session = 2.1.156). Prefer session version → PATH `claude -v` → ~/.local/bin.
  - **Claude GUI:** the desktop app is `/Applications/Claude.app` (`CFBundleShortVersionString` e.g. 1.11187.1) — totally different versioning from Claude Code CLI (2.1.x). Claude Desktop embeds claude-code under `~/Library/Application Support/Claude/claude-code/<ver>/`.
  - **Codex CLI version:** use `codex --version` ("codex-cli 0.130.0" → take the digit-leading token). `~/.codex/version.json` `latest_version` is the UPDATE-CHECKER's value (was 0.129.0 ≠ installed 0.130.0) — fallback only. `threads.cli_version` column is HISTORICAL (whatever version made each thread, e.g. 0.137.0-alpha.4) — NOT the installed version.
  - **GUI apps may NOT be in /Applications.** On this machine Cursor.app and Codex.app live in `~/Apps/`. Always check candidates: /Applications, ~/Applications, ~/Apps. (macOS GUI launch has a minimal PATH, so `codex`/`claude` may not resolve when launched from Finder — keep file/session-based fallbacks for GUI.)
- **Claude rate limits (IMPLEMENTED, CodexBar-style, zero-setup for subscribers):** Claude Code does NOT persist rate limits to disk (only sends them to the statusline command stdin). To get them without setup: read OAuth token from `~/.claude/.credentials.json` (`claudeAiOauth.accessToken`) or Keychain (`security find-generic-password -s "Claude Code-credentials" -w`), then `GET https://api.anthropic.com/api/oauth/usage` with headers `Authorization: Bearer <token>` + `anthropic-beta: oauth-2025-04-20`. Response: `five_hour`/`seven_day`/`seven_day_opus`/`seven_day_sonnet` windows, each `{utilization (already 0-100 percent), resets_at (ISO)}`. Map five_hour→session(300m), seven_day→weekly(10080m). Cached 60s in claude.rs (endpoint is Anthropic-rate-limited). Verified live: 31% session / 19% weekly. Reference: steipete/CodexBar `ClaudeOAuthUsageFetcher.swift`.
- **Cursor rate limits (IMPLEMENTED, browser-cookie import — UNVERIFIED end-to-end):** `cursor_cookies.rs` extracts `WorkosCursorSessionToken` from local browsers (Firefox + Safari first = no prompt; then Chromium family). Chromium decrypt: Keychain pw `security find-generic-password -s "<Browser> Safe Storage" -w` → PBKDF2-HMAC-SHA1("saltysalt", 1003) → 16-byte key → AES-128-CBC (IV=16×0x20, "v10" prefix, PKCS7), then strip a leading 32-byte SHA-256 domain prefix if the plaintext isn't valid UTF-8. Cookie DB is copied to a temp snapshot before opening (browser locks it); modern Chrome path is `<Profile>/Network/Cookies`. Then `GET https://cursor.com/api/usage-summary` with `Cookie: WorkosCursorSessionToken=…` → `individualUsage.plan.totalPercentUsed` → "Plan" window, `individualUsage.onDemand` → "On-Demand", `billingCycleEnd` → resets. Cached 60s. **CAVEAT:** reading Chrome's Keychain key triggers a one-time macOS "allow access" GUI prompt (click Always Allow) — this BLOCKS until answered, so it can't be verified headlessly. RateWindow gained a `label: Option<String>` field for provider-specific labels (Plan/On-Demand vs Session/Weekly). Needs interactive verification: `agent-hub usage --agent cursor`.
- **CodexBar is THE reference** for this whole app (steipete/CodexBar, Swift). When unsure how to source any provider's usage/limits, check its `Sources/CodexBarCore/Providers/<Provider>/` and `docs/<provider>.md`.
- **Cross-agent metric caution:** the dashboard usage chart plots `total_interactions` — keep it a COUNT for every agent (Claude=history lines, Cursor=ai_code_hashes rows, Codex=thread count). Never put a token SUM there or Codex's bar dwarfs everything.
- **JSONL token scans are cached by mtime** (static Mutex<HashMap<path,(mtime,agg)>> in claude.rs & codex.rs) — only the active/growing file re-parses each tick. Don't revert to re-reading every transcript.
- **Tests must use REAL schemas/fixtures, not invented ones.** The old Claude tests used snake_case JSON and passed while real camelCase data failed; `test_cursor_app_exists` hard-asserted /Applications. Capture fixtures from actual agent output.
- **Environment (2026-06-08):** Rust toolchain NOT installed on this machine — no `cargo`/`rustc`, no `~/.cargo`. Must `rustup` install before any Rust build/run. Node v24 present but TUI is pure Rust. Also `cargo` isn't on the non-interactive Bash PATH even after install — may need `source ~/.cargo/env`.

## Do-Not-Repeat

- [2026-06-07] **Cursor CLI version:** Removed fallback to `cursor --version` which returns GUI version. Only `cursor-agent --version` is the CLI version. If not available, return `None`.
- [2026-06-07] **TUI byte slicing crash:** `shorten_path()` and `truncate_id()` used `&s[byte_index..]` which panicked on Chinese characters (e.g. '包' = 3 bytes). Fixed to use `chars().skip().collect()`.
- [2026-06-07] **TUI tab index crash:** After `refresh()` updates agents list, `active_tab` could exceed new tab count. Added clamping in both `refresh()` and `draw()`.
- [2026-06-07] **Forgot TUI parity:** Added Proxy/VPN tab to GUI but forgot TUI. Always add features to both interfaces.

## Decision Log

- **2026-06-07: Proxy detection approach** — Used `networksetup` commands (not `sysproxy` crate) to avoid adding a dependency. Parse text output for HTTP/HTTPS/SOCKS/PAC settings. Query Clash/Surge REST APIs for node info.
- **2026-06-07: TUI tab structure** — Dashboard + per-installed-agent tabs + Proxy/VPN + Keep-Alive. Dynamic tab count based on installed agents. Key bindings: 1=Dash, 2-9=agents, 0=KA.
- **2026-06-07: GUI tab structure** — Same as TUI: system+network at top, then agent tabs with usage (5h/1w) and sessions, Proxy/VPN tab, Keep-Alive at bottom.
