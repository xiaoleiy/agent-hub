<script>
  let { agent } = $props();

  const statusColor = $derived(
    agent.running ? "#22c55e" : agent.installed ? "#eab308" : "#666"
  );
  const statusText = $derived(
    agent.running
      ? `${agent.active_sessions} active`
      : agent.installed
        ? "installed"
        : "not found"
  );
</script>

<div class="agent-card" class:running={agent.running}>
  <div class="header">
    <span class="dot" style="background: {statusColor}"></span>
    <span class="name">{agent.name}</span>
  </div>

  <div class="details">
    {#if agent.version}
      <span class="version">v{agent.version}</span>
    {/if}
    <span class="status">{statusText}</span>
  </div>

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

  .path {
    font-size: 0.75rem;
    color: #555;
    margin-top: 4px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
