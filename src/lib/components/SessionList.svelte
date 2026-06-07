<script>
  let { sessions } = $props();

  function truncateId(id) {
    if (!id) return "";
    return id.length > 16 ? id.slice(0, 16) + "…" : id;
  }

  function relativeTime(isoString) {
    if (!isoString) return "";
    const date = new Date(isoString);
    const now = new Date();
    const diff = now - date;
    const mins = Math.floor(diff / 60000);
    if (mins < 1) return "just now";
    if (mins < 60) return `${mins}m ago`;
    const hours = Math.floor(mins / 60);
    if (hours < 24) return `${hours}h ago`;
    const days = Math.floor(hours / 24);
    return `${days}d ago`;
  }
</script>

<div class="session-list">
  {#if sessions.length === 0}
    <p class="empty">No active sessions</p>
  {:else}
    <table>
      <thead>
        <tr>
          <th>Agent</th>
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
            <td class="agent-name">{sess.agent}</td>
            <td class="session-id">{truncateId(sess.id)}</td>
            <td>
              <span class="mode-badge" class:cli={sess.entrypoint === "cli" || sess.entrypoint === "sdk-cli"} class:gui={sess.entrypoint !== "cli" && sess.entrypoint !== "sdk-cli"}>
                {sess.entrypoint || "—"}
              </span>
            </td>
            <td>
              <span class="status-badge" class:busy={sess.status === "busy"} class:idle={sess.status === "idle" || sess.status === "completed"}>
                {sess.status}
              </span>
            </td>
            <td class="workdir">
              {#if sess.working_dir}
                <span title={sess.working_dir}>
                  {sess.working_dir.split("/").pop()}/
                </span>
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

<style>
  .session-list {
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

  .agent-name {
    font-weight: 600;
    color: #fff;
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
