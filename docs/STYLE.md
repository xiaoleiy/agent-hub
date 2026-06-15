# Agent Hub — Product Style Guide

Conventions for the **GUI** (Svelte/Tauri menu-bar app) and **TUI** (Ratatui). Follow this when adding or changing any user-facing surface.

**Source of truth for shared GUI classes:** `src/lib/styles/panels.css` (imported from `src/routes/+page.svelte`).  
**Theme tokens:** `src/routes/+page.svelte` (`:global(:root)`).

---

## Design goals

- **Compact** — tuned for a ~500px-wide menu-bar popover; avoid wide multi-column layouts that clip on narrow widths.
- **Consistent** — dashboard cards and tab panels should feel like one product (same radii, spacing, titles, labels).
- **Scannable** — uppercase field labels, tabular numbers, clear hierarchy (dashboard title → panel title → inner content).
- **Semantic color** — use theme tokens (`--ok`, `--danger`, …), not raw hex, except in SVG assets.

---

## Theme & color

Use CSS variables only. Never hardcode `#fff` / `#888` in components.

| Token | Use |
|-------|-----|
| `--bg` | Page background |
| `--surface` | Cards, section panels |
| `--surface-2` | Nested rows, inner blocks |
| `--surface-3` | Tracks, inactive fills |
| `--border` / `--border-strong` | Card and input borders |
| `--text` | Body copy |
| `--text-strong` | Titles, emphasis |
| `--text-muted` | Labels, secondary copy |
| `--text-dim` | Hints, empty/loading, reset times |
| `--accent` | Primary actions, links |
| `--ok` | Running, healthy, low stress |
| `--info` | CLI / network / mono highlights |
| `--warn` | Idle, medium stress |
| `--danger` | Errors, busy, high stress |
| `--purple` | GUI-specific badges |
| `--*-tint` | Badge/chip backgrounds (pair with matching accent text) |

**Light/dark:** dark is default; light follows `prefers-color-scheme` or `html[data-theme="light"|"dark"]`.

**Quota / usage bars:** color by *stress* (consumed quota), not raw percent — high remaining = green, high used = red. For `is_remaining` windows, stress = `100 - displayed_percent`.

---

## Typography

Base font size: **13px** on `html` (`+page.svelte`). System UI stack: `-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, …`.

| Role | Class / element | Size | Weight | Color |
|------|-----------------|------|--------|-------|
| Dashboard section title | `.dashboard-title` (`h2`) | 0.95rem | 600 | `--text-strong` |
| Tab panel section title | `.panel-title` (`h3`) | 0.85rem | 600 | `--text-strong` |
| Field label | `.field-label` | 0.8rem | 400 | `--text-muted`, uppercase, `letter-spacing: 0.05em` |
| Field value | `.field-value` | 0.85rem | 400 | `--text`, `tabular-nums` |
| Agent name | `.name` | 0.95rem | 700 | `--text-strong` |
| Badges / chips | `.badge`, `.ver-badge` | 0.68–0.72rem | 500 | tint + accent |
| Mono (paths, IDs) | — | 0.72–0.8rem | — | `"SF Mono", "Fira Code", monospace` |
| Helper / description | `.description` | 0.78rem | — | `--text-muted` |

**Do not** mix `h2` inside tabs for section headers — use `.panel-title` only. Reserve `.dashboard-title` for the home grid (System, Network, Keep Alive).

---

## Layout & spacing

| Pattern | Class | Spec |
|---------|-------|------|
| Tab content stack | `.tab-stack` | `flex-direction: column`, `gap: 12px` |
| Section card | `.section-card` | `padding: 10px 12px`, `border-radius: 9px`, `border: 1px solid var(--border)` |
| Home grid card | `.card` | `padding: 12px`, `border-radius: 9px` (match section-card) |
| Inner nested block | `.inner-surface` | `surface-2`, `border-radius: 8px`, `padding: 10px` |
| Title + trailing meta | `.panel-title-row` | flex baseline, `gap: 8px`, `margin-bottom: 10px` |
| Label + value row | `.info-row` / `.stat` | label column **56px** fixed, `gap: 8px` |
| Section vertical gap | — | **12px** between major blocks, **8–10px** inside lists |

**Agent tabs:** every major block (header, rate limits, tokens, sessions, …) is a `.section-card` inside `.tab-stack`. Agent header is `.section-card.agent-header` (layout only — no duplicate border styles).

**Proxy tab:** same as agent tabs — one `.section-card` per section (System Proxy, VPN, Client, Nodes), not flat unboxed sections.

**Flex shrink:** rate-limit and section cards use `flex-shrink: 0` so stacked rows (e.g. Codex 5h + weekly) are not clipped.

---

## Shared components (`panels.css`)

Import via `+page.svelte` — classes are global.

```html
<section class="section-card">
  <div class="panel-title-row">
    <h3 class="panel-title">Rate Limits</h3>
    <span class="sessions-count">…</span>
  </div>
  …
</section>
```

| Class | Purpose |
|-------|---------|
| `.dashboard-title` | Home grid headings |
| `.panel-title` | In-tab section headings |
| `.panel-title-row` | Title + badge/count on one row |
| `.section-card` | Bordered panel wrapper |
| `.field-label` / `.field-value` | Key/value rows |
| `.loading-state` | In-progress copy |
| `.empty-state` | No data copy |
| `.error-state` | Failure copy (`--danger`) |
| `.inner-surface` | Nested gray block |
| `.bar-track` / `.bar-fill` | 8px progress bars |
| `.tab-stack` | Vertical tab layout |

**Bars:** always `.bar-track` + `.bar-fill` (8px height). Do not invent 6px / 10px variants.

**Token grid:** `repeat(auto-fit, minmax(72px, 1fr))` — never a fixed 5-column grid on narrow widths.

---

## Copy & text

| Situation | Text |
|-----------|------|
| Loading | `Loading…` (ellipsis character `…`, not `...`) |
| Empty sessions | `No active sessions` |
| Empty proxy client | `No proxy client detected` |
| Rate limit quota | `{n}% remaining` or `{n}% used` — match `RateWindow.is_remaining` |
| Reset time | `Resets {locale datetime} ({relative})` — show **time**, not date-only |
| Agent idle | `Not Opened` |
| Agent missing | `Not Found` |
| Running, no sessions | `Running (idle)` in CLI/TUI; GUI uses badges |

**Title case:** section titles use Title Case (`Rate Limits`, `Token Usage`, `Active Sessions`). Field labels use uppercase via CSS (`CPU`, `Public IP`).

**Numbers:** use `toLocaleString()` / `tabular-nums` for counts and tokens; abbreviate large tokens (`1.2M`, `450k`) in agent tabs.

---

## Agent-specific UI

### Rate limits

- **Cursor:** hierarchical — Total overview + Breakdown (Auto+Composer, API). Labels `Total`, `Auto+Composer`, `API`; window is monthly.
- **Codex:** flat stack — two rows (`5h`, `Weekly`), one per `.inner-surface` card; vertical list, not side-by-side columns.
- **Claude / others:** flat windows when present; same card pattern as Codex.
- OAuth Codex API returns **consumed** percent — store `100 - used` with `is_remaining: true` for display.
- Per-window reset line under each bar; **no** reset text in the section title bar.

### Badges

| Badge | Meaning | Colors |
|-------|---------|--------|
| CLI | CLI version / sessions | `--info-tint` / `--info` |
| GUI | Desktop app | `--purple-tint` / `--purple` |
| active | Running sessions | `--ok-tint` / `--ok` |
| idle | Installed, not running | `--warn-tint` / `--warn` |

### Sessions list

- Paginate at **10** per page; show count pill in `panel-title-row`.
- Session ID flex-grows; time column does **not** use `margin-left: auto` (breaks flex growth).

---

## TUI (Ratatui)

Mirror GUI semantics where possible:

- Block titles: ` Rate Limits `, ` Tokens `, ` Active Sessions — {agent} ` (spaces pad the title).
- Flat rate limits: **vertical** rows (3 lines each: label+%, bar, reset). Panel height = `2 + n×3` lines for *n* windows.
- Dashboard quota chart: **remaining %** per agent (`RateWindow::remaining_percent()`), title ` Quota (remaining %) ` — not raw interaction counts.
- Status colors align with GUI: green / yellow / red by stress.

---

## Adding new UI — checklist

1. Reuse `panels.css` classes before adding local styles.
2. Wrap tab sections in `.section-card`; use `.tab-stack` for the tab root.
3. Use theme tokens for all colors.
4. Use `.loading-state` / `.empty-state` / `.error-state` for non-happy paths.
5. Keep label column 56px for aligned key/value rows.
6. Test at **500px** width (menu-bar popover).
7. Update this doc if you introduce a new shared pattern.

---

## Anti-patterns

- Hardcoded colors or one-off border radii (7px / 10px / 12px) on panels.
- `h2` section titles inside agent/proxy tabs.
- Side-by-side rate-limit columns on narrow layouts.
- Inconsistent loading strings (`Loading...`, `Loading proxy information...`).
- Putting reset times only in the section header instead of per window.
- `contain: content` on parents that must grow with stacked children (can clip).
- Comparing unlike metrics on the dashboard chart (use quota remaining %, not interaction counts).
