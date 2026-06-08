<script>
  let { info } = $props();

  async function refresh() {
    // Will be refreshed on next parent poll
  }
</script>

<div class="network-info">
  <h2>Network</h2>
  {#if info}
    <div class="info-grid">
      <div class="info-row">
        <span class="label">Public IP</span>
        <span class="value ip">{info.ip}</span>
      </div>
      {#if info.country}
        <div class="info-row">
          <span class="label">Location</span>
          <span class="value">{info.city}, {info.region}, {info.country}</span>
        </div>
      {/if}
      {#if info.org}
        <div class="info-row">
          <span class="label">ISP</span>
          <span class="value">{info.org}</span>
        </div>
      {/if}
      {#if info.timezone}
        <div class="info-row">
          <span class="label">Timezone</span>
          <span class="value">{info.timezone}</span>
        </div>
      {/if}
    </div>
  {:else}
    <p class="loading">Loading network info...</p>
  {/if}
</div>

<style>
  .network-info h2 {
    font-size: 0.95rem;
    font-weight: 600;
    color: #fff;
    margin-bottom: 8px;
  }

  .info-grid {
    display: grid;
    gap: 6px;
  }

  .info-row {
    display: grid;
    grid-template-columns: 56px 1fr;
    gap: 8px;
    align-items: center;
  }

  .label {
    font-size: 0.8rem;
    color: #888;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .value {
    font-size: 0.85rem;
    color: #ccc;
  }

  .value.ip {
    font-size: 0.95rem;
    font-weight: 700;
    color: #22c55e;
    font-family: "SF Mono", "Fira Code", monospace;
  }

  .loading {
    color: #666;
    font-size: 0.9rem;
  }
</style>
