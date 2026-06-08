<script>
  let { status } = $props();

  function formatBytes(/** @type {number} */ bytes) {
    if (bytes >= 1073741824) return (bytes / 1073741824).toFixed(2) + " GB";
    if (bytes >= 1048576) return (bytes / 1048576).toFixed(2) + " MB";
    if (bytes >= 1024) return (bytes / 1024).toFixed(2) + " KB";
    return bytes + " B";
  }

  function formatRate(/** @type {number} */ bytesPerSec) {
    if (bytesPerSec >= 1048576) return (bytesPerSec / 1048576).toFixed(1) + " MB/s";
    if (bytesPerSec >= 1024) return Math.round(bytesPerSec / 1024) + " KB/s";
    return Math.round(bytesPerSec) + " B/s";
  }
</script>

<div class="system-status">
  <h2>System Status</h2>
  {#if status}
    <div class="stat-grid">
      <div class="stat">
        <span class="label">CPU</span>
        <div class="bar-container">
          <div
            class="bar"
            style="width: {Math.min(status.cpu_usage, 100)}%"
            class:high={status.cpu_usage > 80}
            class:medium={status.cpu_usage > 50 && status.cpu_usage <= 80}
          ></div>
        </div>
        <span class="value">{status.cpu_usage.toFixed(1)}% ({status.cpu_cores} cores)</span>
      </div>

      <div class="stat">
        <span class="label">RAM</span>
        <div class="bar-container">
          <div
            class="bar"
            style="width: {Math.min(status.ram_usage_percent, 100)}%"
            class:high={status.ram_usage_percent > 80}
            class:medium={status.ram_usage_percent > 50 && status.ram_usage_percent <= 80}
          ></div>
        </div>
        <span class="value">{status.ram_used_gb} / {status.ram_total_gb} GB ({status.ram_usage_percent.toFixed(1)}%)</span>
      </div>

      <div class="stat">
        <span class="label">Uptime</span>
        <span class="value standalone">{status.uptime_formatted}</span>
      </div>

      <div class="stat">
        <span class="label">User</span>
        <span class="value standalone">{status.username}</span>
      </div>

      <div class="stat">
        <span class="label">Host</span>
        <span class="value standalone">{status.hostname}</span>
      </div>

      <div class="stat">
        <span class="label">Traffic</span>
        <span class="value standalone">↑ {formatRate(status.network_upload_rate)} &nbsp; ↓ {formatRate(status.network_download_rate)}</span>
      </div>
    </div>
  {:else}
    <p class="loading">Loading...</p>
  {/if}
</div>

<style>
  .system-status h2 {
    font-size: 0.95rem;
    font-weight: 600;
    color: var(--text-strong);
    margin-bottom: 8px;
  }

  .stat-grid {
    display: grid;
    gap: 7px;
  }

  .stat {
    display: grid;
    grid-template-columns: 56px 1fr auto;
    align-items: center;
    gap: 8px;
  }

  .label {
    font-size: 0.8rem;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .bar-container {
    height: 6px;
    background: var(--surface-3);
    border-radius: 3px;
    overflow: hidden;
  }

  .bar {
    height: 100%;
    background: var(--ok);
    border-radius: 3px;
    transition: width 0.5s ease;
  }

  .bar.medium {
    background: var(--warn);
  }

  .bar.high {
    background: var(--danger);
  }

  .value {
    font-size: 0.85rem;
    color: var(--text);
    font-variant-numeric: tabular-nums;
    text-align: right;
  }

  .value.standalone {
    grid-column: 2 / -1;
    text-align: left;
  }

  .loading {
    color: var(--text-dim);
    font-size: 0.9rem;
  }
</style>
