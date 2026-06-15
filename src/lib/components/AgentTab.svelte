<script>
  import { invoke } from "@tauri-apps/api/core";

  let { agent } = $props();

  let sessions = $state(/** @type {any[]} */ ([]));
  let richUsage = $state(/** @type {any} */ (null));

  // Client-side pagination for the session list (10 per page).
  const PAGE_SIZE = 10;
  let page = $state(0);
  const pageCount = $derived(Math.max(1, Math.ceil(sessions.length / PAGE_SIZE)));
  const pagedSessions = $derived(
    sessions.slice(page * PAGE_SIZE, page * PAGE_SIZE + PAGE_SIZE),
  );
  // Keep the page in range when the list shrinks on refresh.
  $effect(() => {
    if (page > pageCount - 1) page = pageCount - 1;
  });

  async function fetchData() {
    const name = agent.name === "Claude Code" ? "claude" : agent.name.toLowerCase();
    try {
      sessions = await invoke("get_agent_sessions", { agent: name });
    } catch (_) {
      sessions = [];
    }
    try {
      richUsage = await invoke("get_agent_rich_usage", { agent: name });
    } catch (_) {
      richUsage = null;
    }
  }

  // Refetch only when the *selected agent identity* changes (e.g. the user
  // switches tabs), NOT on every new object reference from the parent's 5s
  // poll. The parent reassigns `agents` to brand-new objects each tick, so
  // depending on `agent` directly re-ran this effect (and re-rendered the
  // whole tab) every 5s — a poll firing mid-scroll caused a hitch.
  $effect(() => {
    const _ = agent.name; // stable identity dependency only
    page = 0; // reset pagination when switching agents
    fetchData();
  });

  // Keep this tab's own sessions/usage live with a modest, decoupled interval
  // so data still refreshes without coupling to the parent's object churn.
  $effect(() => {
    const id = setInterval(fetchData, 5000);
    return () => clearInterval(id);
  });

  /** Show the tail of a path (most informative part) for narrow rows. */
  function shortDir(/** @type {string} */ p) {
    if (!p) return "";
    const max = 38;
    return p.length > max ? "…" + p.slice(p.length - max) : p;
  }

  /** @param {string} isoString */
  function relativeTime(isoString) {
    if (!isoString) return "";
    const date = new Date(isoString);
    const now = new Date();
    const diff = /** @type {any} */ (date) - /** @type {any} */ (now);
    const absMins = Math.abs(Math.floor(diff / 60000));
    if (absMins < 1) return diff >= 0 ? "soon" : "just now";
    if (absMins < 60) return diff >= 0 ? `in ${absMins}m` : `${absMins}m ago`;
    const hours = Math.floor(absMins / 60);
    if (hours < 24) return diff >= 0 ? `in ${hours}h` : `${hours}h ago`;
    const days = Math.floor(hours / 24);
    return diff >= 0 ? `in ${days}d` : `${days}d ago`;
  }

  /** @param {string} isoString */
  function formatResetAt(isoString) {
    if (!isoString) return "";
    const date = new Date(isoString);
    const when = date.toLocaleString(undefined, {
      month: "short",
      day: "numeric",
      hour: "numeric",
      minute: "2-digit",
    });
    return `Resets ${when} (${relativeTime(isoString)})`;
  }

  /** @param {any} usage */
  function collectRateWindows(usage) {
    if (!usage) return [];
    /** @type {any[]} */
    const windows = [];
    if (usage.session_window) windows.push(usage.session_window);
    if (usage.weekly_window) windows.push(usage.weekly_window);
    if (usage.extra_rate_windows?.length) windows.push(...usage.extra_rate_windows);
    return windows;
  }

  /** @param {any} usage */
  function splitRateLimits(usage) {
    if (!usage) return { total: null, breakdown: [], flat: [] };
    const total =
      usage.session_window?.label === "Total" ? usage.session_window : null;
    if (total) {
      /** @type {any[]} */
      const breakdown = [];
      for (const w of usage.extra_rate_windows ?? []) breakdown.push(w);
      if (usage.weekly_window) breakdown.push(usage.weekly_window);
      const order = ["Auto+Composer", "API"];
      breakdown.sort(
        (a, b) =>
          (order.indexOf(a.label) === -1 ? 99 : order.indexOf(a.label)) -
          (order.indexOf(b.label) === -1 ? 99 : order.indexOf(b.label)),
      );
      return { total, breakdown, flat: [] };
    }
    return { total: null, breakdown: [], flat: collectRateWindows(usage) };
  }

  const rateLayout = $derived(splitRateLimits(richUsage));

  /** @param {number} n */
  function fmtTokens(n) {
    if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
    if (n >= 1_000) return `${(n / 1_000).toFixed(1)}k`;
    return n.toLocaleString();
  }

  /** @param {number} percent */
  /** @param {any} w */
  function quotaStress(w) {
    return w.is_remaining ? 100 - w.used_percent : w.used_percent;
  }

  /** @param {any} w */
  function quotaBarColor(w) {
    const stress = quotaStress(w);
    if (stress >= 90) return "var(--danger)";
    if (stress >= 70) return "var(--warn)";
    return "var(--ok)";
  }

  /** @param {any} w */
  function quotaBarWidth(w) {
    return Math.min(w.used_percent, 100);
  }

  /** @param {any} w */
  function quotaLabel(w) {
    return w.is_remaining ? "remaining" : "used";
  }

  /** @param {number} mins */
  function formatWindow(mins) {
    if (mins >= 43200) return "monthly";
    if (mins >= 10080) return `${Math.round(mins / 10080)}w`;
    if (mins >= 1440) return `${Math.round(mins / 1440)}d`;
    if (mins >= 60) return `${Math.round(mins / 60)}h`;
    return `${mins}m`;
  }
</script>

<div class="agent-tab tab-stack">
  <!-- Agent Header -->
  <div class="section-card agent-header">
    <div class="agent-info">
      <span class="dot" class:running={agent.running}></span>
      <span class="name">{agent.name}</span>
      {#if agent.cli_version}
        <span class="ver-badge cli">CLI {agent.cli_version}</span>
      {/if}
      {#if agent.gui_version}
        <span class="ver-badge gui">GUI {agent.gui_version}</span>
      {/if}
    </div>
    <div class="session-summary">
      {#if agent.running}
        <span class="badge active">{agent.active_sessions} active</span>
        {#if agent.cli_sessions > 0}
          <span class="badge cli">{agent.cli_sessions} CLI</span>
        {/if}
        {#if agent.gui_sessions > 0}
          <span class="badge gui">{agent.gui_sessions} GUI</span>
        {/if}
      {:else if agent.installed}
        <span class="badge idle">Not Opened</span>
      {:else}
        <span class="badge missing">Not Found</span>
      {/if}
    </div>
  </div>

  <!-- Rate Limit Windows -->
  {#if rateLayout.total || rateLayout.breakdown.length > 0 || rateLayout.flat.length > 0}
    <section class="section-card rate-limits">
      <div class="panel-title-row">
        <h3 class="panel-title">Rate Limits</h3>
      </div>

      {#if rateLayout.total}
        {@const w = rateLayout.total}
        <div class="rate-hierarchy">
          <div class="rate-overview inner-surface">
            <div class="rate-header">
              <span class="rate-label rate-label-overview">
                {w.label ?? "Total"}
                <span class="rate-sublabel">monthly overview</span>
              </span>
              <span class="rate-percent" style="color: {quotaBarColor(w)}">
                {w.used_percent.toFixed(1)}% {quotaLabel(w)}
              </span>
            </div>
            <div class="rate-bar-track bar-track rate-bar-track-overview">
              <div
                class="rate-bar-fill bar-fill"
                style="width: {quotaBarWidth(w)}%; background: {quotaBarColor(w)}"
              ></div>
            </div>
            {#if w.resets_at}
              <span class="rate-reset">{formatResetAt(w.resets_at)}</span>
            {/if}
          </div>

          {#if rateLayout.breakdown.length > 0}
            <div class="rate-breakdown-panel">
              <span class="rate-breakdown-title">Breakdown</span>
              <div class="rate-breakdown-list">
                {#each rateLayout.breakdown as w, i}
                  <div class="rate-row">
                    <span class="rate-tree" aria-hidden="true"
                      >{i + 1 === rateLayout.breakdown.length ? "└" : "├"}</span
                    >
                    <div class="rate-row-content">
                      <div class="rate-header">
                        <span class="rate-label">{w.label ?? "—"} (monthly)</span>
                        <span class="rate-percent" style="color: {quotaBarColor(w)}">
                          {w.used_percent.toFixed(1)}% {quotaLabel(w)}
                        </span>
                      </div>
                      <div class="rate-bar-track bar-track">
                        <div
                          class="rate-bar-fill bar-fill"
                          style="width: {quotaBarWidth(w)}%; background: {quotaBarColor(w)}"
                        ></div>
                      </div>
                      {#if w.resets_at}
                        <span class="rate-reset">{formatResetAt(w.resets_at)}</span>
                      {/if}
                    </div>
                  </div>
                {/each}
              </div>
            </div>
          {/if}
        </div>
      {:else}
        <!-- Generic agents: flat grid -->
        <div class="rate-grid">
          {#each rateLayout.flat as w}
            <div class="rate-card inner-surface">
              <div class="rate-header">
                <span class="rate-label"
                  >{w.label ?? "Limit"}{#if w.window_minutes >= 43200}
                    (monthly){:else}
                    ({formatWindow(w.window_minutes)}){/if}</span
                >
                <span class="rate-percent" style="color: {quotaBarColor(w)}">
                  {w.used_percent.toFixed(1)}% {quotaLabel(w)}
                </span>
              </div>
              <div class="rate-bar-track bar-track">
                <div
                  class="rate-bar-fill bar-fill"
                  style="width: {quotaBarWidth(w)}%; background: {quotaBarColor(w)}"
                ></div>
              </div>
              {#if w.resets_at}
                <span class="rate-reset">{formatResetAt(w.resets_at)}</span>
              {/if}
            </div>
          {/each}
        </div>
      {/if}
    </section>
  {/if}

  <!-- Token Usage -->
  {#if richUsage?.tokens}
    {@const t = richUsage.tokens}
    <section class="section-card tokens-section">
      <h3 class="panel-title">Token Usage</h3>
      <div class="token-grid">
        <div class="token-card">
          <div class="token-label">Input</div>
          <div class="token-value">{fmtTokens(t.input_tokens)}</div>
        </div>
        <div class="token-card">
          <div class="token-label">Cache Read</div>
          <div class="token-value cache">{fmtTokens(t.cache_read_tokens)}</div>
        </div>
        <div class="token-card">
          <div class="token-label">Cache Create</div>
          <div class="token-value cache">{fmtTokens(t.cache_create_tokens)}</div>
        </div>
        <div class="token-card">
          <div class="token-label">Output</div>
          <div class="token-value">{fmtTokens(t.output_tokens)}</div>
        </div>
        <div class="token-card total">
          <div class="token-label">Total</div>
          <div class="token-value">{fmtTokens(t.total_tokens)}</div>
        </div>
      </div>
    </section>
  {/if}

  <!-- Model Breakdowns -->
  {#if richUsage?.model_breakdowns?.length > 0}
    <section class="section-card models-section">
      <h3 class="panel-title">Models</h3>
      <div class="model-list">
        {#each richUsage.model_breakdowns as m}
          <div class="model-row">
            <span class="model-name">{m.model}</span>
            <span class="model-tokens">{fmtTokens(m.total_tokens)} tokens</span>
            <span class="model-requests">{m.request_count} reqs</span>
          </div>
        {/each}
      </div>
    </section>
  {/if}

  <!-- Summary Stats -->
  {#if richUsage && !richUsage.tokens && !rateLayout.total && rateLayout.flat.length === 0}
    <section class="section-card summary-section">
      <div class="summary-grid">
        <div class="summary-card">
          <div class="summary-label">Total Interactions</div>
          <div class="summary-value">{richUsage.total_interactions.toLocaleString()}</div>
        </div>
        <div class="summary-card">
          <div class="summary-label">Total Sessions</div>
          <div class="summary-value">{richUsage.total_sessions}</div>
        </div>
      </div>
    </section>
  {/if}

  <!-- Sessions Section -->
  <section class="section-card sessions-section">
    <div class="panel-title-row">
      <h3 class="panel-title">Active Sessions</h3>
      <span class="sessions-count">{sessions.length}</span>
    </div>
    {#if sessions.length === 0}
      <p class="empty-state">No active sessions</p>
    {:else}
      <div class="session-list">
        {#each pagedSessions as sess}
          <div class="session-item">
            <div class="session-top">
              <span
                class="s-dot"
                class:busy={sess.status === "busy"}
                title={sess.status}
              ></span>
              <span class="s-id" title={sess.id}>{sess.id}</span>
              <span
                class="s-mode"
                class:gui={sess.entrypoint !== "cli" && sess.entrypoint !== "sdk-cli"}
              >
                {sess.entrypoint || "—"}
              </span>
              <span class="s-time">{relativeTime(sess.started_at)}</span>
            </div>
            <div class="session-dir" title={sess.working_dir || ""}>
              {#if sess.working_dir}{shortDir(sess.working_dir)}{:else}<span class="dim">no working dir</span>{/if}
            </div>
          </div>
        {/each}
      </div>
      {#if pageCount > 1}
        <div class="pager">
          <button
            class="pg-btn"
            disabled={page === 0}
            onclick={() => (page = Math.max(0, page - 1))}
            aria-label="Previous page">‹</button
          >
          <span class="pg-info">{page + 1} / {pageCount}</span>
          <button
            class="pg-btn"
            disabled={page >= pageCount - 1}
            onclick={() => (page = Math.min(pageCount - 1, page + 1))}
            aria-label="Next page">›</button
          >
        </div>
      {/if}
    {/if}
  </section>
</div>

<style>
  .agent-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 6px 8px;
  }

  .rate-limits {
    flex-shrink: 0;
  }

  .agent-info {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px;
    min-width: 0;
  }

  .dot {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: var(--text-dim);
    flex-shrink: 0;
  }

  .dot.running { background: var(--ok); }

  .name {
    font-weight: 700;
    font-size: 0.95rem;
    color: var(--text-strong);
    white-space: nowrap;
    margin-right: 2px;
  }

  .ver-badge {
    display: inline-block;
    padding: 1px 6px;
    border-radius: 4px;
    font-size: 0.68rem;
    font-weight: 500;
    font-family: "SF Mono", "Fira Code", monospace;
  }

  .ver-badge.cli { background: var(--info-tint); color: var(--info); }
  .ver-badge.gui { background: var(--purple-tint); color: var(--purple); }

  .session-summary { display: flex; gap: 5px; flex-shrink: 0; }

  .badge {
    display: inline-block;
    padding: 2px 8px;
    border-radius: 5px;
    font-size: 0.72rem;
    font-weight: 500;
  }

  .badge.active { background: var(--ok-tint); color: var(--ok); }
  .badge.cli { background: var(--info-tint); color: var(--info); }
  .badge.gui { background: var(--purple-tint); color: var(--purple); }
  .badge.idle { background: var(--warn-tint); color: var(--warn); }
  .badge.missing { background: var(--border-strong); color: var(--text-dim); }

  .rate-grid {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .rate-hierarchy {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .rate-label-overview {
    display: flex;
    flex-direction: column;
    gap: 1px;
    font-weight: 600;
    color: var(--text-strong);
  }

  .rate-sublabel {
    font-size: 0.65rem;
    font-weight: 500;
    color: var(--text-dim);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .rate-breakdown-panel {
    padding: 10px 10px 10px 12px;
    background: var(--surface-2);
    border-radius: 8px;
    border-left: 3px solid var(--border-strong);
  }

  .rate-breakdown-title {
    display: block;
    font-size: 0.68rem;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin-bottom: 8px;
  }

  .rate-breakdown-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .rate-row {
    display: grid;
    grid-template-columns: 18px minmax(0, 1fr);
    column-gap: 8px;
    align-items: start;
  }

  .rate-tree {
    font-family: "SF Mono", "Fira Code", monospace;
    font-size: 0.85rem;
    color: var(--text-dim);
    line-height: 1.35;
    padding-top: 1px;
    text-align: center;
  }

  .rate-row-content {
    min-width: 0;
  }

  .rate-row-content .rate-header {
    margin-bottom: 6px;
  }

  .rate-row-content .rate-label {
    font-size: 0.8rem;
  }

  .rate-card {
    flex-shrink: 0;
  }

  .rate-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 8px;
    margin-bottom: 8px;
  }

  .rate-label {
    font-size: 0.8rem;
    color: var(--text-muted);
    min-width: 0;
  }

  .rate-percent {
    font-size: 0.85rem;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    flex-shrink: 0;
    white-space: nowrap;
  }

  .rate-reset {
    font-size: 0.72rem;
    color: var(--text-dim);
    margin-top: 6px;
    display: block;
  }

  /* Tokens */
  .token-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(72px, 1fr));
    gap: 6px;
  }

  .token-card {
    background: var(--surface-2);
    border-radius: 8px;
    padding: 8px 4px;
    text-align: center;
  }

  .token-card.total {
    background: var(--accent-tint);
    border: 1px solid var(--accent-tint-strong);
  }

  .token-label {
    font-size: 0.58rem;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.01em;
    margin-bottom: 3px;
    white-space: nowrap;
  }

  .token-value {
    font-size: 0.95rem;
    font-weight: 700;
    color: var(--text-strong);
    font-variant-numeric: tabular-nums;
  }

  .token-value.cache {
    color: var(--info);
  }

  /* Models */
  .model-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .model-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 9px;
    background: var(--surface-2);
    border-radius: 6px;
    font-size: 0.8rem;
    contain: content;
  }

  .model-name {
    flex: 1 1 auto;
    min-width: 0;
    font-family: "SF Mono", "Fira Code", monospace;
    font-size: 0.78rem;
    color: var(--text-strong);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .model-tokens {
    color: var(--info);
    font-variant-numeric: tabular-nums;
    flex-shrink: 0;
  }

  .model-requests {
    color: var(--text-muted);
    font-size: 0.72rem;
    flex-shrink: 0;
  }

  /* Summary */
  .summary-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }

  .summary-card {
    text-align: center;
  }

  .summary-label {
    font-size: 0.8rem;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin-bottom: 4px;
  }

  .summary-value {
    font-size: 1.4rem;
    font-weight: 700;
    color: var(--text-strong);
  }

  /* Sessions */
  .sessions-count {
    font-size: 0.68rem;
    font-weight: 600;
    color: var(--text-muted);
    background: var(--border);
    border-radius: 999px;
    padding: 1px 7px;
    font-variant-numeric: tabular-nums;
  }

  .session-list {
    display: flex;
    flex-direction: column;
  }

  .session-item {
    padding: 7px 0;
    border-bottom: 1px solid var(--border);
    contain: content;
  }
  .session-item:first-child { padding-top: 1px; }
  .session-item:last-child { border-bottom: none; padding-bottom: 1px; }

  .session-top {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .s-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--ok);
    flex-shrink: 0;
  }
  .s-dot.busy { background: var(--danger); }

  .s-id {
    flex: 1 1 auto;
    min-width: 0;
    font-family: "SF Mono", "Fira Code", monospace;
    font-size: 0.78rem;
    color: var(--text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .s-mode {
    flex-shrink: 0;
    font-size: 0.64rem;
    font-weight: 500;
    padding: 1px 6px;
    border-radius: 4px;
    background: var(--info-tint);
    color: var(--info);
    text-transform: uppercase;
    letter-spacing: 0.02em;
  }
  .s-mode.gui { background: var(--purple-tint); color: var(--purple); }

  .s-time {
    flex-shrink: 0;
    font-size: 0.72rem;
    color: var(--text-dim);
    font-variant-numeric: tabular-nums;
  }

  .session-dir {
    margin-top: 2px;
    margin-left: 15px;
    font-family: "SF Mono", "Fira Code", monospace;
    font-size: 0.72rem;
    color: var(--text-dim);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .dim { color: var(--text-dim); }

  /* Pagination */
  .pager {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 12px;
    margin-top: 8px;
  }

  .pg-btn {
    min-width: 26px;
    height: 24px;
    padding: 0 8px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text);
    font-size: 0.95rem;
    line-height: 1;
    cursor: pointer;
    transition: background 0.15s, border-color 0.15s, color 0.15s;
  }

  .pg-btn:hover:not(:disabled) {
    background: var(--surface-3);
    color: var(--text-strong);
  }

  .pg-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .pg-info {
    font-size: 0.72rem;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }
</style>
