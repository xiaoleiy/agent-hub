<script>
  let { stats, window } = $props();

  const maxInteractions = $derived(
    Math.max(...stats.map((s) => s.total_interactions), 1)
  );
</script>

<div class="usage-chart">
  {#if stats.length === 0}
    <p class="empty">No usage data available</p>
  {:else}
    <div class="bars">
      {#each stats as stat}
        <div class="bar-group">
          <div class="bar-label">{stat.agent}</div>
          <div class="bar-track">
            <div
              class="bar-fill"
              style="width: {(stat.total_interactions / maxInteractions) * 100}%"
            ></div>
          </div>
          <div class="bar-value">
            <span class="interactions">{stat.total_interactions.toLocaleString()}</span>
            <span class="sessions">{stat.total_sessions} sessions</span>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .usage-chart {
    min-height: 80px;
  }

  .bars {
    display: grid;
    gap: 12px;
  }

  .bar-group {
    display: grid;
    grid-template-columns: 100px 1fr 160px;
    align-items: center;
    gap: 12px;
  }

  .bar-label {
    font-weight: 600;
    font-size: 0.85rem;
    color: #fff;
  }

  .bar-track {
    height: 20px;
    background: #2a2a2a;
    border-radius: 6px;
    overflow: hidden;
  }

  .bar-fill {
    height: 100%;
    background: linear-gradient(90deg, #2563eb, #3b82f6);
    border-radius: 6px;
    transition: width 0.5s ease;
    min-width: 4px;
  }

  .bar-value {
    display: flex;
    flex-direction: column;
    text-align: right;
  }

  .interactions {
    font-size: 0.9rem;
    font-weight: 600;
    color: #fff;
    font-variant-numeric: tabular-nums;
  }

  .sessions {
    font-size: 0.75rem;
    color: #888;
  }

  .empty {
    color: #666;
    font-size: 0.9rem;
    text-align: center;
    padding: 20px;
  }
</style>
