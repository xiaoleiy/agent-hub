# Agent Hub

Manage AI coding agents on your macOS machine. Monitor system status, track agent usage, and keep your Mac awake while agents are running.

## Supported Agents

| Agent | Detection | Sessions | Usage Tracking |
|-------|-----------|----------|----------------|
| Claude Code | `~/.claude/sessions/*.json` | Live PID verification | `history.jsonl` |
| Cursor | `/Applications/Cursor.app` | `ai-tracking.db` SQLite | AI code metrics |
| Codex | `~/.codex/state_5.sqlite` | Thread activity | Token counts |

## Install

### Homebrew (recommended)

```bash
brew tap xiaoleiy/tap
brew install --cask xiaoleiy/tap/agent-hub
```

This installs both the GUI app and the `agent-hub` CLI command.

### Build from source

```bash
git clone https://github.com/xiaoleiy/agent-hub.git
cd agent-hub
npm install
npm run tauri build
```

## Usage

### GUI

Launch from Applications or run:

```bash
agent-hub
# or
agent-hub gui
```

### TUI (Terminal UI)

Interactive terminal dashboard (like `htop`):

```bash
agent-hub tui
```

Keybindings:
- `←`/`→` or `Tab` — switch tabs
- `1`/`2`/`3`/`4` — jump to tab
- `r` — refresh
- `w` — cycle usage window (5h → 1w → 1m)
- `Space` — toggle keep-alive (1h default)
- `a`/`s`/`d`/`f` — set keep-alive (30m/1h/3h/forever)
- `q`/`Esc` — quit

### CLI

```bash
# System status (CPU, RAM, uptime, network)
agent-hub status
agent-hub status --json

# Network info (public IP from ip.net.coffee)
agent-hub network

# List detected agents
agent-hub agents

# Active sessions
agent-hub sessions
agent-hub sessions --agent claude

# Usage statistics
agent-hub usage                  # default: 5 hour window
agent-hub usage --window 1w      # 1 week
agent-hub usage --window 1m      # 1 month

# Keep-alive (prevent sleep)
agent-hub keepalive 30m          # 30 minutes
agent-hub keepalive 1h           # 1 hour
agent-hub keepalive 3h           # 3 hours
agent-hub keepalive forever      # until manually disabled
agent-hub keepalive off          # disable
agent-hub keepalive --status     # show current status
```

All CLI commands support `--json` for machine-readable output.

## Architecture

```
agent-hub
├── src/                    # Svelte frontend
│   ├── routes/+page.svelte # Main dashboard
│   └── lib/components/     # UI components
├── src-tauri/              # Rust backend
│   └── src/
│       ├── lib.rs          # CLI/GUI router
│       ├── cli/            # CLI argument parsing (clap)
│       ├── tui/            # Interactive terminal UI (ratatui)
│       ├── core_modules/   # Business logic
│       │   ├── system.rs   # System stats (sysinfo)
│       │   ├── network.rs  # IP lookup (reqwest)
│       │   ├── keepalive.rs # caffeinate wrapper
│       │   └── agents/     # Agent detection
│       ├── commands/       # Tauri IPC handlers
│       └── models/         # Shared data types
```

## Development

```bash
# Install dependencies
npm install

# Run in development mode (hot-reload)
npm run tauri dev

# Build for production
npm run tauri build

# Run CLI directly
cargo run --manifest-path src-tauri/Cargo.toml -- status

# Run TUI directly
cargo run --manifest-path src-tauri/Cargo.toml -- tui
```

## License

MIT
