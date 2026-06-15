<script>
  let { info } = $props();
</script>

<div class="network-info">
  <h2 class="dashboard-title">Network</h2>
  {#if info}
    <div class="info-grid">
      <div class="info-row">
        <span class="field-label">Public IP</span>
        <span class="field-value ip">{info.ip}</span>
      </div>
      {#if info.country}
        <div class="info-row">
          <span class="field-label">Location</span>
          <span class="field-value">{info.city}, {info.region}, {info.country}</span>
        </div>
      {/if}
      {#if info.org}
        <div class="info-row">
          <span class="field-label">ISP</span>
          <span class="field-value">{info.org}</span>
        </div>
      {/if}
      {#if info.timezone}
        <div class="info-row">
          <span class="field-label">Timezone</span>
          <span class="field-value">{info.timezone}</span>
        </div>
      {/if}
    </div>
  {:else}
    <p class="loading-state">Loading…</p>
  {/if}
</div>

<style>
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

  .field-value.ip {
    font-size: 0.95rem;
    font-weight: 700;
    color: var(--ok);
    font-family: "SF Mono", "Fira Code", monospace;
  }
</style>
