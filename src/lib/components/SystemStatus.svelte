<script>
  let { status } = $props();

  function formatBytes(bytes) {
    if (bytes >= 1073741824) return (bytes / 1073741824).toFixed(2) + " GB";
    if (bytes >= 1048576) return (bytes / 1048576).toFixed(2) + " MB";
    if (bytes >= 1024) return (bytes / 1024).toFixed(2) + " KB";
    return bytes + " B";
  }

  function formatRate(bytesPerSec) {
    if (bytesPerSec >= 1048576) return (bytesPerSec / 1048576).toFixed(2) + " MB/s";
    if (bytesPerSec >= 1024) return (bytesPerSec / 1024).toFixed(2) + " KB/s";
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
        <span class="label">Upload</span>
        <span class="value standalone">{formatRate(status.network_upload_rate)}</span>
      </div>

      <div class="stat">
        <span class="label">Download</span>
        <span class="value standalone">{formatRate(status.network_download_rate)}</span>
      </div>
    </div>
  {:else}
    <p class="loading">Loading...</p>
  {/if}
</div>

<style>
  .system-status h2 {
    font-size: 1.1rem;
    font-weight: 600;
    color: #fff;
    margin-bottom: 12px;
  }

  .stat-grid {
    display: grid;
    gap: 10px;
  }

  .stat {
    display: grid;
    grid-template-columns: 90px 1fr auto;
    align-items: center;
    gap: 8px;
  }

  .label {
    font-size: 0.8rem;
    color: #888;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .bar-container {
    height: 6px;
    background: #2a2a2a;
    border-radius: 3px;
    overflow: hidden;
  }

  .bar {
    height: 100%;
    background: #22c55e;
    border-radius: 3px;
    transition: width 0.5s ease;
  }

  .bar.medium {
    background: #eab308;
  }

  .bar.high {
    background: #ef4444;
  }

  .value {
    font-size: 0.85rem;
    color: #ccc;
    font-variant-numeric: tabular-nums;
    text-align: right;
  }

  .value.standalone {
    grid-column: 2 / -1;
    text-align: left;
  }

  .loading {
    color: #666;
    font-size: 0.9rem;
  }
</style>
