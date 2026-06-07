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

## Do-Not-Repeat

- [2026-06-07] **Cursor CLI version:** Removed fallback to `cursor --version` which returns GUI version. Only `cursor-agent --version` is the CLI version. If not available, return `None`.
- [2026-06-07] **TUI byte slicing crash:** `shorten_path()` and `truncate_id()` used `&s[byte_index..]` which panicked on Chinese characters (e.g. '包' = 3 bytes). Fixed to use `chars().skip().collect()`.
- [2026-06-07] **TUI tab index crash:** After `refresh()` updates agents list, `active_tab` could exceed new tab count. Added clamping in both `refresh()` and `draw()`.
- [2026-06-07] **Forgot TUI parity:** Added Proxy/VPN tab to GUI but forgot TUI. Always add features to both interfaces.

## Decision Log

- **2026-06-07: Proxy detection approach** — Used `networksetup` commands (not `sysproxy` crate) to avoid adding a dependency. Parse text output for HTTP/HTTPS/SOCKS/PAC settings. Query Clash/Surge REST APIs for node info.
- **2026-06-07: TUI tab structure** — Dashboard + per-installed-agent tabs + Proxy/VPN + Keep-Alive. Dynamic tab count based on installed agents. Key bindings: 1=Dash, 2-9=agents, 0=KA.
- **2026-06-07: GUI tab structure** — Same as TUI: system+network at top, then agent tabs with usage (5h/1w) and sessions, Proxy/VPN tab, Keep-Alive at bottom.
