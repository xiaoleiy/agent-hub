<script>
  import { invoke } from "@tauri-apps/api/core";

  let { agent } = $props();

  let sessions = $state(/** @type {any[]} */ ([]));
  let usage5h = $state(/** @type {any} */ (null));
  let usage1w = $state(/** @type {any} */ (null));

  async function fetchData() {
    try {
      const name = agent.name === "Claude Code" ? "claude" : agent.name.toLowerCase();
      sessions = await invoke("get_agent_sessions", { agent: name });
    } catch (_) {
      sessions = [];
    }
    try {
      const name = agent.name === "Claude Code" ? "claude" : agent.name.toLowerCase();
      usage5h = await invoke("get_agent_usage", { agent: name, window: "5h" });
      usage1w = await invoke("get_agent_usage", { agent: name, window: "1w" });
    } catch (_) {
      usage5h = null;
      usage1w = null;
    }
  }

  $effect(() => {
    // Re-fetch when agent changes
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

  <!-- Usage Section -->
  <div class="usage-section">
    <h3>Usage</h3>
    <div class="usage-grid">
      <div class="usage-card">
        <div class="usage-label">Per 5 Hours</div>
        {#if usage5h}
          <div class="usage-value">{usage5h.total_interactions.toLocaleString()}</div>
          <div class="usage-detail">{usage5h.total_sessions} sessions</div>
        {:else}
          <div class="usage-value dim">—</div>
        {/if}
      </div>
      <div class="usage-card">
        <div class="usage-label">Per 1 Week</div>
        {#if usage1w}
          <div class="usage-value">{usage1w.total_interactions.toLocaleString()}</div>
          <div class="usage-detail">{usage1w.total_sessions} sessions</div>
        {:else}
          <div class="usage-value dim">—</div>
        {/if}
      </div>
    </div>
  </div>

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

  .dot.running {
    background: #22c55e;
  }

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

  .ver-badge.cli {
    background: #0ea5e920;
    color: #0ea5e9;
  }

  .ver-badge.gui {
    background: #a855f720;
    color: #a855f7;
  }

  .session-summary {
    display: flex;
    gap: 6px;
  }

  .badge {
    display: inline-block;
    padding: 3px 10px;
    border-radius: 6px;
    font-size: 0.75rem;
    font-weight: 500;
  }

  .badge.active {
    background: #22c55e20;
    color: #22c55e;
  }

  .badge.cli {
    background: #0ea5e920;
    color: #0ea5e9;
  }

  .badge.gui {
    background: #a855f720;
    color: #a855f7;
  }

  .badge.idle {
    background: #eab30820;
    color: #eab308;
  }

  .badge.missing {
    background: #333;
    color: #666;
  }

  .usage-section h3,
  .sessions-section h3 {
    font-size: 0.95rem;
    font-weight: 600;
    color: #fff;
    margin-bottom: 10px;
  }

  .usage-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }

  .usage-card {
    background: #1a1a1a;
    border: 1px solid #2a2a2a;
    border-radius: 10px;
    padding: 16px;
    text-align: center;
  }

  .usage-label {
    font-size: 0.75rem;
    color: #888;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin-bottom: 8px;
  }

  .usage-value {
    font-size: 1.6rem;
    font-weight: 700;
    color: #fff;
    font-variant-numeric: tabular-nums;
  }

  .usage-value.dim {
    color: #444;
    font-size: 1.2rem;
  }

  .usage-detail {
    font-size: 0.75rem;
    color: #888;
    margin-top: 4px;
  }

  .sessions-section {
    overflow-x: auto;
  }

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

  .status-badge.busy {
    background: #ef444420;
    color: #ef4444;
  }

  .status-badge.idle {
    background: #22c55e20;
    color: #22c55e;
  }

  .mode-badge {
    display: inline-block;
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 0.75rem;
    font-weight: 500;
    background: #333;
    color: #888;
  }

  .mode-badge.cli {
    background: #0ea5e920;
    color: #0ea5e9;
  }

  .mode-badge.gui {
    background: #a855f720;
    color: #a855f7;
  }

  .workdir {
    max-width: 150px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .time {
    color: #888;
    font-size: 0.8rem;
  }

  .dim {
    color: #555;
  }

  .empty {
    color: #666;
    font-size: 0.9rem;
    text-align: center;
    padding: 20px;
  }
</style>
