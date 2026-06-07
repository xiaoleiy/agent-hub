# Memory

> Chronological action log. Hooks and AI append to this file automatically.
> Old sessions are consolidated by the daemon weekly.

## Session: 2026-06-07 (main implementation)

| Time | Action | File(s) | Outcome | ~Tokens |
|------|--------|---------|---------|--------|
| — | Fix Cursor CLI version bug | `cursor.rs` | Removed `cursor --version` fallback that returned GUI version | ~2k |
| — | Add AgentTab component | `AgentTab.svelte` | Per-agent tab with usage (5h/1w) + sessions table | ~3k |
| — | Rewrite +page.svelte with tabs | `+page.svelte` | Agent tabs replacing flat sections | ~3k |
| — | Add Tauri tray icon click handler | `lib.rs` | Toggle window visibility on tray click, hide-to-tray on close | ~2k |
| — | Add proxy/VPN detection module | `proxy.rs` | System proxy via networksetup, VPN via scutil, client detection via API + pgrep | ~8k |
| — | Add ProxyInfo types | `types.rs` | ProxyInfo, SystemProxy, ProxyEntry, VpnConnection, ProxyClient, ProxyNode | ~2k |
| — | Add ProxyInfo GUI component | `ProxyInfo.svelte` | System proxy cards, VPN list, client info, expandable node groups | ~4k |
| — | Register proxy Tauri command | `commands/proxy.rs`, `lib.rs` | `get_proxy_info` IPC command | ~1k |
| — | Align TUI with GUI tabs | `tui/mod.rs` | Dynamic agent tabs, removed fixed Sessions/Usage tabs | ~5k |
| — | Improve TUI sessions table | `tui/mod.rs` | Removed Agent column, wider Session ID (24), wider Working Dir (Min 16) | ~2k |
| — | Fix TUI tab index crash | `tui/mod.rs` | Added active_tab clamping in refresh() and draw() | ~1k |
| — | Fix UTF-8 panic in shorten_path | `tui/mod.rs` | Changed byte slicing to char-based operations | ~1k |
| — | Add Proxy/VPN tab to TUI | `tui/mod.rs` | System proxy, VPN, client, nodes sections | ~5k |
| — | Update OpenWolf memory | `.wolf/cerebrum.md`, `.wolf/memory.md` | Synced learnings and session log | ~2k |

## Session: 2026-06-07 (memory sync)

| Time | Action | File(s) | Outcome | ~Tokens |
|------|--------|---------|---------|--------|
| — | Sync OpenWolf cerebrum | `.wolf/cerebrum.md` | Added user prefs, key learnings, do-not-repeat, decision log | ~2k |
| — | Sync OpenWolf memory | `.wolf/memory.md` | Added session action log | ~1k |
| — | Log bugs to buglog | `.wolf/buglog.json` | Logged Cursor CLI version bug, UTF-8 panic, tab index crash | ~1k |
| 23:41 | Edited src-tauri/src/models/types.rs | expanded (+49 lines) | ~476 |
| 23:41 | Edited src-tauri/src/core_modules/agents/claude.rs | added 1 import(s) | ~61 |
| 23:41 | Edited src-tauri/src/core_modules/agents/claude.rs | modified parse_window() | ~1276 |
| 23:42 | Edited src-tauri/src/core_modules/agents/codex.rs | added 2 import(s) | ~62 |
| 23:42 | Edited src-tauri/src/core_modules/agents/codex.rs | modified window_seconds() | ~1612 |
| 23:42 | Edited src-tauri/src/core_modules/agents/mod.rs | inline fix | ~22 |
| 23:42 | Edited src-tauri/src/core_modules/agents/mod.rs | modified get_usage() | ~174 |
| 23:42 | Edited src-tauri/src/core_modules/agents/cursor.rs | 4→4 lines | ~43 |
| 23:42 | Edited src-tauri/src/core_modules/agents/cursor.rs | modified get_rich_usage() | ~379 |
| 23:43 | Edited src-tauri/src/commands/agents.rs | 2→2 lines | ~31 |
| 23:43 | Edited src-tauri/src/commands/agents.rs | modified get_agent_rich_usage() | ~193 |
| 23:43 | Edited src-tauri/src/lib.rs | 2→3 lines | ~39 |
| 23:44 | Created src/lib/components/AgentTab.svelte | — | ~3836 |
| 23:44 | Edited src-tauri/src/tui/mod.rs | inline fix | ~34 |
| 23:44 | Edited src-tauri/src/tui/mod.rs | 3→4 lines | ~38 |
| 23:44 | Edited src-tauri/src/tui/mod.rs | 2→3 lines | ~33 |
| 23:44 | Edited src-tauri/src/tui/mod.rs | expanded (+7 lines) | ~155 |
| 23:45 | Edited src-tauri/src/tui/mod.rs | modified len() | ~139 |
| 23:45 | Edited src-tauri/src/tui/mod.rs | modified draw_agent_tab() | ~401 |
| 23:45 | Edited src-tauri/src/tui/mod.rs | modified draw_rate_limits() | ~1075 |
| 23:45 | Edited src-tauri/src/tui/mod.rs | removed 15 lines | ~12 |
