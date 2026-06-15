# Agent Hub — Agent Instructions

OpenWolf context: see `.wolf/OPENWOLF.md`, `.wolf/cerebrum.md`, `.wolf/anatomy.md`.

## UI & styling

Follow **[docs/STYLE.md](docs/STYLE.md)** for product-wide conventions (layout, typography, colors, copy, shared CSS classes). Shared GUI primitives live in `src/lib/styles/panels.css`.

## Stack

- **GUI:** SvelteKit + Tauri (`src/`, `src-tauri/`)
- **TUI:** Ratatui (`src-tauri/src/tui/mod.rs`)
- **Agents:** `src-tauri/src/core_modules/agents/`
