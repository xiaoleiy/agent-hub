# Agent Hub — Handoff Document

**Last updated:** 2026-06-07
**Repo:** https://github.com/xiaoleiy/agent-hub
**Status:** Functional with one known TUI bug

---

## What This Is

A macOS desktop + CLI + TUI app for managing AI coding agents (Claude Code, Cursor, Codex) on a local machine. Built with Tauri 2 (Rust) + Svelte 5 + ratatui.

## Quick Start

```bash
cd /Users/xiaoleiyu/Repos/agent-hub

# GUI mode (Tauri window)
npm run tauri dev

# TUI mode (interactive terminal dashboard)
cargo run --manifest-path src-tauri/Cargo.toml -- tui

# CLI mode (one-shot commands)
cargo run --manifest-path src-tauri/Cargo.toml -- status
cargo run --manifest-path src-tauri/Cargo.toml -- agents
cargo run --manifest-path src-tauri/Cargo.toml -- sessions
cargo run --manifest-path src-tauri/Cargo.toml -- usage
cargo run --manifest-path src-tauri/Cargo.toml -- network
cargo run --manifest-path src-tauri/Cargo.toml -- keepalive --status

# Run all tests
cargo test --manifest-path src-tauri/Cargo.toml
```

---

## Architecture

Single binary, three modes:
- **No args / `gui`** → Tauri GUI window
- **`tui`** → Interactive ratatui dashboard (like htop)
- **Other args** → CLI one-shot commands (clap)

All three share `core_modules/` for business logic.

```
src-tauri/src/
├── lib.rs              # Router: CLI vs GUI vs TUI
├── cli/
│   ├── mod.rs          # Clap argument definitions
│   └── handlers.rs     # CLI command handlers
├── tui/
│   └── mod.rs          # ratatui interactive dashboard
├── core_modules/
│   ├── system.rs       # CPU, RAM, uptime, network speed (sysinfo)
│   ├── network.rs      # IP lookup via ipinfo.io/json
│   ├── keepalive.rs    # caffeinate wrapper for sleep prevention
│   └── agents/
│       ├── claude.rs   # Claude Code detection
│       ├── cursor.rs   # Cursor detection (CLI + GUI)
│       └── codex.rs    # Codex detection (CLI + Desktop)
├── commands/           # Tauri IPC handlers (for GUI)
│   ├── system.rs
│   ├── network.rs
│   ├── agents.rs
│   └── keepalive.rs
└── models/
    └── types.rs        # Shared structs (SystemStatus, AgentInfo, Session, etc.)

src/                    # Svelte frontend
├── lib/components/
│   ├── SystemStatus.svelte
│   ├── NetworkInfo.svelte
│   ├── AgentCard.svelte
│   ├── SessionList.svelte
│   ├── UsageChart.svelte
│   └── KeepAlive.svelte
└── routes/+page.svelte # Main dashboard
```

---

## Features

| Feature | CLI | TUI | GUI |
|---------|-----|-----|-----|
| System status (CPU, RAM, uptime, hostname) | ✅ | ✅ | ✅ |
| Network speed (upload/download rate) | ✅ | ✅ | ✅ |
| IP info (ipinfo.io) | ✅ | ✅ | ✅ |
| Agent detection (Claude, Cursor, Codex) | ✅ | ✅ | ✅ |
| CLI vs GUI session counts per agent | ✅ | ✅ | ✅ |
| Dual version display (CLI ver / GUI ver) | ✅ | ⚠️ | ✅ |
| Active sessions with entrypoint | ✅ | ✅ | ✅ |
| Usage stats (5h/1w/1m) | ✅ | ✅ | ✅ |
| Keep-alive toggle (30m/1h/3h/forever) | ✅ | ✅ | ✅ |

---

## Known Bug: TUI Cursor CLI Version

**Symptom:** TUI Agents table shows Cursor CLI Ver as "3.7.12" (GUI version) instead of "2026.06.04-5fd875e" (CLI version).

**What works:**
- CLI `agent-hub agents` shows correct "CLI v2026.06.04-5fd875e / GUI v3.7.12"
- GUI AgentCard shows correct versions
- Unit tests pass (including `test_cursor_cli_version_is_date_based`)

**What was tried:**
1. Wider column widths → did not fix
2. Paragraph with explicit labels → user reverted, says not display issue
3. Debug file output → confirmed correct values reach rendering code
4. Retry logic in `get_cursor_cli_version()` → fixed flaky test but TUI still wrong
5. Clean rebuilds → user says issue persists

**Investigation needed:** The Rust code produces the correct value. The issue is somewhere in the TUI rendering pipeline. Possible causes:
- ratatui caching/stale state
- Column order mismatch between header and body
- Terminal-specific rendering issue
- Binary not being updated despite rebuild

---

## Version Detection Sources

| Agent | CLI Version Source | GUI Version Source |
|-------|-------------------|-------------------|
| Claude Code | `claude -v` → parse "2.1.168 (Claude Code)" | Session JSON (entrypoint != "cli") |
| Cursor | `~/.local/bin/cursor-agent --version` → "2026.06.04-5fd875e" | `Info.plist CFBundleShortVersionString` → "3.7.12" |
| Codex | `~/.codex/version.json` → "0.137.0" | `/Applications/Codex.app/Contents/Info.plist` → "26.602.40724" |

---

## Data Sources for Agent Detection

**Claude Code** (`~/.claude/`):
- `sessions/*.json` — live sessions with PID, entrypoint (cli/sdk-cli), version, status
- `history.jsonl` — interaction history for usage stats

**Cursor** (`~/.cursor/`):
- `ai-tracking/ai-code-tracking.db` — SQLite with conversation_summaries, ai_code_hashes
- `/Applications/Cursor.app` — GUI app

**Codex** (`~/.codex/`):
- `state_5.sqlite` — threads table with source (cli/vscode/exec), tokens_used
- `process_manager/chat_processes.json` — live PIDs
- `version.json` — CLI version
- `/Applications/Codex.app` — Desktop app

---

## Dependencies

**Rust (Cargo.toml):**
- tauri 2 (with tray-icon feature)
- sysinfo 0.32 (system stats)
- reqwest 0.12 (HTTP client)
- rusqlite 0.32 (SQLite, bundled)
- clap 4 (CLI parsing)
- ratatui 0.29 + crossterm 0.28 (TUI)
- chrono, whoami, colored, anyhow, dirs-next, tokio

**Node.js (package.json):**
- @tauri-apps/api 2
- svelte 5, @sveltejs/kit, @sveltejs/adapter-static
- vite 6

---

## Distribution

**Homebrew:** Plan is a Cask with `binary` stanza to expose CLI in PATH.
- Tap repo: `xiaoleiy/homebrew-tap` (not yet created)
- CI/CD: `.github/workflows/release.yml` (tag → build DMGs → GitHub release)
- Needs `gh auth refresh -h github.com -s workflow` to push workflow files

---

## Testing

26 unit tests across 4 modules:
- `cursor::tests` (7) — binary exists, version format, CLI ≠ GUI
- `claude::tests` (7) — binary exists, version format, session parsing
- `codex::tests` (9) — DB schema, version format, session counts
- `system::tests` (4) — format_uptime, system status, rates, serialization

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

---

## Git Status

- Main branch: `main`
- All changes committed and pushed
- GitHub Actions workflows need `gh auth refresh -h github.com -s workflow` to push
