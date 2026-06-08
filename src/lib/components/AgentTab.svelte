<script>
  import { invoke } from "@tauri-apps/api/core";

  let { agent } = $props();

  let sessions = $state(/** @type {any[]} */ ([]));
  let richUsage = $state(/** @type {any} */ (null));

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

  $effect(() => {
    const _ = agent;
    fetchData();
  });

  /** @param {string} id */
  function truncateId(id) {
    if (!id) return "";
    return id.length > 14 ? id.slice(0, 14) + "…" : id;
  }

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
    const diff = /** @type {any} */ (now) - /** @type {any} */ (date);
    const mins = Math.floor(diff / 60000);
    if (mins < 1) return "just now";
    if (mins < 60) return `${mins}m ago`;
    const hours = Math.floor(mins / 60);
    if (hours < 24) return `${hours}h ago`;
    const days = Math.floor(hours / 24);
    return `${days}d ago`;
  }

  /** @param {number} n */
  function fmtTokens(n) {
    if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
    if (n >= 1_000) return `${(n / 1_000).toFixed(1)}k`;
    return n.toLocaleString();
  }

  /** @param {number} percent */
  function usageBarColor(percent) {
    if (percent >= 90) return "#ef4444";
    if (percent >= 70) return "#eab308";
    return "#22c55e";
  }

  /** @param {number} mins */
  function formatWindow(mins) {
    if (mins >= 10080) return `${Math.round(mins / 10080)}w`;
    if (mins >= 1440) return `${Math.round(mins / 1440)}d`;
    if (mins >= 60) return `${Math.round(mins / 60)}h`;
    return `${mins}m`;
  }
</script>

<div class="agent-tab">
  <!-- Agent Header -->
  <div class="agent-header">
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
  {#if richUsage?.session_window || richUsage?.weekly_window}
    <div class="rate-limits">
      <h3>Rate Limits</h3>
      <div class="rate-grid">
        {#if richUsage.session_window}
          {@const w = richUsage.session_window}
          <div class="rate-card">
            <div class="rate-header">
              <span class="rate-label">Session ({formatWindow(w.window_minutes)})</span>
              <span class="rate-percent" style="color: {usageBarColor(w.used_percent)}">
                {w.used_percent.toFixed(1)}% used
              </span>
            </div>
            <div class="rate-bar-track">
              <div
                class="rate-bar-fill"
                style="width: {Math.min(w.used_percent, 100)}%; background: {usageBarColor(w.used_percent)}"
              ></div>
            </div>
            {#if w.resets_at}
              <div class="rate-reset">resets {relativeTime(w.resets_at)}</div>
            {/if}
          </div>
        {/if}
        {#if richUsage.weekly_window}
          {@const w = richUsage.weekly_window}
          <div class="rate-card">
            <div class="rate-header">
              <span class="rate-label">Weekly ({formatWindow(w.window_minutes)})</span>
              <span class="rate-percent" style="color: {usageBarColor(w.used_percent)}">
                {w.used_percent.toFixed(1)}% used
              </span>
            </div>
            <div class="rate-bar-track">
              <div
                class="rate-bar-fill"
                style="width: {Math.min(w.used_percent, 100)}%; background: {usageBarColor(w.used_percent)}"
              ></div>
            </div>
            {#if w.resets_at}
              <div class="rate-reset">resets {relativeTime(w.resets_at)}</div>
            {/if}
          </div>
        {/if}
      </div>
    </div>
  {/if}

  <!-- Token Usage -->
  {#if richUsage?.tokens}
    {@const t = richUsage.tokens}
    <div class="tokens-section">
      <h3>Token Usage</h3>
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
    </div>
  {/if}

  <!-- Model Breakdowns -->
  {#if richUsage?.model_breakdowns?.length > 0}
    <div class="models-section">
      <h3>Models</h3>
      <div class="model-list">
        {#each richUsage.model_breakdowns as m}
          <div class="model-row">
            <span class="model-name">{m.model}</span>
            <span class="model-tokens">{fmtTokens(m.total_tokens)} tokens</span>
            <span class="model-requests">{m.request_count} reqs</span>
          </div>
        {/each}
      </div>
    </div>
  {/if}

  <!-- Summary Stats -->
  {#if richUsage && !richUsage.tokens && !richUsage.session_window}
    <div class="summary-section">
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
    </div>
  {/if}

  <!-- Sessions Section -->
  <div class="sessions-section">
    <div class="sessions-head">
      <h3>Active Sessions</h3>
      <span class="sessions-count">{sessions.length}</span>
    </div>
    {#if sessions.length === 0}
      <p class="empty">No active sessions</p>
    {:else}
      <div class="session-list">
        {#each sessions as sess}
          <div class="session-item">
            <div class="session-top">
              <span
                class="s-dot"
                class:busy={sess.status === "busy"}
                title={sess.status}
              ></span>
              <span class="s-id">{truncateId(sess.id)}</span>
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
    {/if}
  </div>
</div>

<style>
  .agent-tab {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .agent-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 6px 8px;
    padding: 9px 12px;
    background: #1a1a1a;
    border: 1px solid #262626;
    border-radius: 9px;
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
    background: #666;
    flex-shrink: 0;
  }

  .dot.running { background: #22c55e; }

  .name {
    font-weight: 700;
    font-size: 0.95rem;
    color: #fff;
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

  .ver-badge.cli { background: #0ea5e920; color: #0ea5e9; }
  .ver-badge.gui { background: #a855f720; color: #a855f7; }

  .session-summary { display: flex; gap: 5px; flex-shrink: 0; }

  .badge {
    display: inline-block;
    padding: 2px 8px;
    border-radius: 5px;
    font-size: 0.72rem;
    font-weight: 500;
  }

  .badge.active { background: #22c55e20; color: #22c55e; }
  .badge.cli { background: #0ea5e920; color: #0ea5e9; }
  .badge.gui { background: #a855f720; color: #a855f7; }
  .badge.idle { background: #eab30820; color: #eab308; }
  .badge.missing { background: #333; color: #666; }

  h3 {
    font-size: 0.85rem;
    font-weight: 600;
    color: #fff;
    margin-bottom: 8px;
  }

  /* Rate Limits */
  .rate-limits {
    background: #1a1a1a;
    border: 1px solid #262626;
    border-radius: 10px;
    padding: 12px;
  }

  .rate-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }

  .rate-card {
    background: #222;
    border-radius: 8px;
    padding: 10px;
  }

  .rate-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 8px;
  }

  .rate-label {
    font-size: 0.8rem;
    color: #888;
  }

  .rate-percent {
    font-size: 0.85rem;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }

  .rate-bar-track {
    height: 8px;
    background: #333;
    border-radius: 4px;
    overflow: hidden;
  }

  .rate-bar-fill {
    height: 100%;
    border-radius: 4px;
    transition: width 0.5s ease;
  }

  .rate-reset {
    font-size: 0.7rem;
    color: #666;
    margin-top: 4px;
  }

  /* Tokens */
  .tokens-section {
    background: #1a1a1a;
    border: 1px solid #262626;
    border-radius: 10px;
    padding: 12px;
  }

  .token-grid {
    display: grid;
    grid-template-columns: repeat(5, 1fr);
    gap: 5px;
  }

  .token-card {
    background: #222;
    border-radius: 7px;
    padding: 7px 4px;
    text-align: center;
  }

  .token-card.total {
    background: #2563eb20;
    border: 1px solid #2563eb40;
  }

  .token-label {
    font-size: 0.58rem;
    color: #888;
    text-transform: uppercase;
    letter-spacing: 0.01em;
    margin-bottom: 3px;
    white-space: nowrap;
  }

  .token-value {
    font-size: 0.95rem;
    font-weight: 700;
    color: #fff;
    font-variant-numeric: tabular-nums;
  }

  .token-value.cache {
    color: #0ea5e9;
  }

  /* Models */
  .models-section {
    background: #1a1a1a;
    border: 1px solid #262626;
    border-radius: 10px;
    padding: 12px;
  }

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
    background: #222;
    border-radius: 6px;
    font-size: 0.8rem;
  }

  .model-name {
    flex: 1 1 auto;
    min-width: 0;
    font-family: "SF Mono", "Fira Code", monospace;
    font-size: 0.78rem;
    color: #fff;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .model-tokens {
    color: #0ea5e9;
    font-variant-numeric: tabular-nums;
    flex-shrink: 0;
  }

  .model-requests {
    color: #888;
    font-size: 0.72rem;
    flex-shrink: 0;
  }

  /* Summary */
  .summary-section {
    background: #1a1a1a;
    border: 1px solid #262626;
    border-radius: 10px;
    padding: 12px;
  }

  .summary-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }

  .summary-card {
    text-align: center;
  }

  .summary-label {
    font-size: 0.75rem;
    color: #888;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin-bottom: 4px;
  }

  .summary-value {
    font-size: 1.4rem;
    font-weight: 700;
    color: #fff;
  }

  /* Sessions */
  .sessions-head {
    display: flex;
    align-items: center;
    gap: 7px;
    margin-bottom: 6px;
  }

  .sessions-head h3 { margin-bottom: 0; }

  .sessions-count {
    font-size: 0.68rem;
    font-weight: 600;
    color: #999;
    background: #262626;
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
    border-bottom: 1px solid #1f1f1f;
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
    background: #22c55e;
    flex-shrink: 0;
  }
  .s-dot.busy { background: #ef4444; }

  .s-id {
    font-family: "SF Mono", "Fira Code", monospace;
    font-size: 0.78rem;
    color: #ccc;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
  }

  .s-mode {
    flex-shrink: 0;
    font-size: 0.64rem;
    font-weight: 500;
    padding: 1px 6px;
    border-radius: 4px;
    background: #0ea5e920;
    color: #0ea5e9;
    text-transform: uppercase;
    letter-spacing: 0.02em;
  }
  .s-mode.gui { background: #a855f720; color: #a855f7; }

  .s-time {
    margin-left: auto;
    flex-shrink: 0;
    font-size: 0.72rem;
    color: #777;
    font-variant-numeric: tabular-nums;
  }

  .session-dir {
    margin-top: 2px;
    margin-left: 15px;
    font-family: "SF Mono", "Fira Code", monospace;
    font-size: 0.72rem;
    color: #777;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .dim { color: #555; }
  .empty { color: #666; font-size: 0.85rem; text-align: center; padding: 14px; }
</style>
