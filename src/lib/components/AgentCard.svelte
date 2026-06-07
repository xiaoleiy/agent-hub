<script>
  let { agent } = $props();

  const statusColor = $derived(
    agent.running ? "#22c55e" : agent.installed ? "#eab308" : "#666"
  );
  const statusText = $derived(
    agent.running
      ? `${agent.active_sessions} active`
      : agent.installed
        ? "Not Opened"
        : "Not Found"
  );

  function versionDisplay(agent) {
    const cv = agent.cli_version;
    const gv = agent.gui_version;
    if (cv && gv && cv !== gv) return { cli: cv, gui: gv };
    if (cv && gv) return { single: cv };
    if (cv) return { cli: cv };
    if (gv) return { gui: gv };
    return {};
  }

  const v = $derived(versionDisplay(agent));
</script>

<div class="agent-card" class:running={agent.running}>
  <div class="header">
    <span class="dot" style="background: {statusColor}"></span>
    <span class="name">{agent.name}</span>
  </div>

  <div class="details">
    {#if v.single}
      <span class="version">v{v.single}</span>
    {/if}
    <span class="status">{statusText}</span>
  </div>

  {#if v.cli || v.gui}
    <div class="versions">
      {#if v.cli}
        <span class="ver-badge cli">CLI v{v.cli}</span>
      {/if}
      {#if v.gui}
        <span class="ver-badge gui">GUI v{v.gui}</span>
      {/if}
    </div>
  {/if}

  {#if agent.running}
    <div class="sessions">
      {#if agent.cli_sessions > 0}
        <span class="badge cli">{agent.cli_sessions} CLI</span>
      {/if}
      {#if agent.gui_sessions > 0}
        <span class="badge gui">{agent.gui_sessions} GUI</span>
      {/if}
    </div>
  {/if}

  {#if agent.install_path}
    <div class="path" title={agent.install_path}>
      {agent.install_path.split("/").pop()}
    </div>
  {/if}
</div>

<style>
  .agent-card {
    background: #222;
    border: 1px solid #333;
    border-radius: 10px;
    padding: 14px;
    transition: border-color 0.2s;
  }

  .agent-card.running {
    border-color: #22c55e40;
  }

  .header {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 8px;
  }

  .dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .name {
    font-weight: 600;
    font-size: 0.95rem;
    color: #fff;
  }

  .details {
    display: flex;
    gap: 8px;
    align-items: center;
    margin-bottom: 4px;
  }

  .version {
    font-size: 0.8rem;
    color: #888;
    font-family: "SF Mono", "Fira Code", monospace;
  }

  .status {
    font-size: 0.8rem;
    color: #aaa;
  }

  .versions {
    display: flex;
    gap: 6px;
    margin-top: 6px;
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

  .sessions {
    display: flex;
    gap: 6px;
    margin-top: 6px;
  }

  .badge {
    display: inline-block;
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 0.75rem;
    font-weight: 500;
  }

  .badge.cli {
    background: #0ea5e920;
    color: #0ea5e9;
  }

  .badge.gui {
    background: #a855f720;
    color: #a855f7;
  }

  .path {
    font-size: 0.75rem;
    color: #555;
    margin-top: 4px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
