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
    return id.length > 16 ? id.slice(0, 16) + "…" : id;
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
    <h3>Active Sessions</h3>
    {#if sessions.length === 0}
      <p class="empty">No active sessions</p>
    {:else}
      <table>
        <thead>
          <tr>
            <th>Session</th>
            <th>Mode</th>
            <th>Status</th>
            <th>Working Dir</th>
            <th>Time</th>
          </tr>
        </thead>
        <tbody>
          {#each sessions as sess}
            <tr>
              <td class="session-id">{truncateId(sess.id)}</td>
              <td>
                <span
                  class="mode-badge"
                  class:cli={sess.entrypoint === "cli" || sess.entrypoint === "sdk-cli"}
                  class:gui={sess.entrypoint !== "cli" && sess.entrypoint !== "sdk-cli"}
                >
                  {sess.entrypoint || "—"}
                </span>
              </td>
              <td>
                <span
                  class="status-badge"
                  class:busy={sess.status === "busy"}
                  class:idle={sess.status === "idle" || sess.status === "completed"}
                >
                  {sess.status}
                </span>
              </td>
              <td class="workdir">
                {#if sess.working_dir}
                  <span title={sess.working_dir}>{sess.working_dir.split("/").pop()}/</span>
                {:else}
                  <span class="dim">—</span>
                {/if}
              </td>
              <td class="time">{relativeTime(sess.started_at)}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </div>
</div>

<style>
  .agent-tab {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .agent-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px;
    background: #1a1a1a;
    border: 1px solid #2a2a2a;
    border-radius: 10px;
  }

  .agent-info {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: #666;
    flex-shrink: 0;
  }

  .dot.running { background: #22c55e; }

  .name {
    font-weight: 700;
    font-size: 1.05rem;
    color: #fff;
  }

  .ver-badge {
    display: inline-block;
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 0.7rem;
    font-weight: 500;
    font-family: "SF Mono", "Fira Code", monospace;
  }

  .ver-badge.cli { background: #0ea5e920; color: #0ea5e9; }
  .ver-badge.gui { background: #a855f720; color: #a855f7; }

  .session-summary { display: flex; gap: 6px; }

  .badge {
    display: inline-block;
    padding: 3px 10px;
    border-radius: 6px;
    font-size: 0.75rem;
    font-weight: 500;
  }

  .badge.active { background: #22c55e20; color: #22c55e; }
  .badge.cli { background: #0ea5e920; color: #0ea5e9; }
  .badge.gui { background: #a855f720; color: #a855f7; }
  .badge.idle { background: #eab30820; color: #eab308; }
  .badge.missing { background: #333; color: #666; }

  h3 {
    font-size: 0.95rem;
    font-weight: 600;
    color: #fff;
    margin-bottom: 10px;
  }

  /* Rate Limits */
  .rate-limits {
    background: #1a1a1a;
    border: 1px solid #2a2a2a;
    border-radius: 10px;
    padding: 16px;
  }

  .rate-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }

  .rate-card {
    background: #222;
    border-radius: 8px;
    padding: 12px;
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
    border: 1px solid #2a2a2a;
    border-radius: 10px;
    padding: 16px;
  }

  .token-grid {
    display: grid;
    grid-template-columns: repeat(5, 1fr);
    gap: 8px;
  }

  .token-card {
    background: #222;
    border-radius: 8px;
    padding: 10px;
    text-align: center;
  }

  .token-card.total {
    background: #2563eb20;
    border: 1px solid #2563eb40;
  }

  .token-label {
    font-size: 0.65rem;
    color: #888;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin-bottom: 4px;
  }

  .token-value {
    font-size: 1.1rem;
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
    border: 1px solid #2a2a2a;
    border-radius: 10px;
    padding: 16px;
  }

  .model-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .model-row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 6px 10px;
    background: #222;
    border-radius: 6px;
    font-size: 0.85rem;
  }

  .model-name {
    flex: 1;
    font-family: "SF Mono", "Fira Code", monospace;
    font-size: 0.8rem;
    color: #fff;
  }

  .model-tokens {
    color: #0ea5e9;
    font-variant-numeric: tabular-nums;
  }

  .model-requests {
    color: #888;
    font-size: 0.75rem;
  }

  /* Summary */
  .summary-section {
    background: #1a1a1a;
    border: 1px solid #2a2a2a;
    border-radius: 10px;
    padding: 16px;
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
  .sessions-section { overflow-x: auto; }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.85rem;
  }

  th {
    text-align: left;
    padding: 8px 12px;
    color: #888;
    font-weight: 500;
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    border-bottom: 1px solid #2a2a2a;
  }

  td {
    padding: 8px 12px;
    border-bottom: 1px solid #1f1f1f;
    color: #ccc;
  }

  .session-id {
    font-family: "SF Mono", "Fira Code", monospace;
    font-size: 0.8rem;
    color: #888;
  }

  .status-badge {
    display: inline-block;
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 0.75rem;
    font-weight: 500;
  }

  .status-badge.busy { background: #ef444420; color: #ef4444; }
  .status-badge.idle { background: #22c55e20; color: #22c55e; }

  .mode-badge {
    display: inline-block;
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 0.75rem;
    font-weight: 500;
    background: #333;
    color: #888;
  }

  .mode-badge.cli { background: #0ea5e920; color: #0ea5e9; }
  .mode-badge.gui { background: #a855f720; color: #a855f7; }

  .workdir {
    max-width: 150px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .time { color: #888; font-size: 0.8rem; }
  .dim { color: #555; }
  .empty { color: #666; font-size: 0.9rem; text-align: center; padding: 20px; }
</style>
