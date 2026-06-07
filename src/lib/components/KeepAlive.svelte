<script>
  import { invoke } from "@tauri-apps/api/core";

  let { status } = $props();

  const modes = [
    { label: "30 min", value: "30m" },
    { label: "1 hour", value: "1h" },
    { label: "3 hours", value: "3h" },
    { label: "Forever", value: "forever" },
  ];

  let loading = $state(false);

  async function setMode(mode) {
    loading = true;
    try {
      if (status?.active && status?.mode === mode) {
        // Toggle off
        await invoke("set_keepalive", { mode: "off" });
      } else {
        await invoke("set_keepalive", { mode });
      }
      // Refresh status
      status = await invoke("get_keepalive_status");
    } catch (e) {
      console.error("Failed to set keep-alive:", e);
    } finally {
      loading = false;
    }
  }

  async function turnOff() {
    loading = true;
    try {
      await invoke("set_keepalive", { mode: "off" });
      status = await invoke("get_keepalive_status");
    } catch (e) {
      console.error("Failed to disable keep-alive:", e);
    } finally {
      loading = false;
    }
  }
</script>

<div class="keepalive">
  <h2>Keep Alive</h2>
  <p class="description">Prevent your Mac from sleeping while agents are running</p>

  <div class="controls">
    {#each modes as mode}
      <button
        class:active={status?.active && status?.mode === mode.value}
        disabled={loading}
        onclick={() => setMode(mode.value)}
      >
        {mode.label}
      </button>
    {/each}
  </div>

  <div class="status-bar">
    {#if status?.active}
      <span class="status-active">
        <span class="pulse"></span>
        Active — {status.mode}
      </span>
      <button class="off-btn" onclick={turnOff} disabled={loading}>Turn Off</button>
    {:else}
      <span class="status-off">Sleep prevention is off</span>
    {/if}
  </div>
</div>

<style>
  .keepalive h2 {
    font-size: 1.1rem;
    font-weight: 600;
    color: #fff;
    margin-bottom: 4px;
  }

  .description {
    font-size: 0.85rem;
    color: #888;
    margin-bottom: 16px;
  }

  .controls {
    display: flex;
    gap: 8px;
    margin-bottom: 16px;
  }

  .controls button {
    flex: 1;
    padding: 10px 16px;
    background: #2a2a2a;
    border: 1px solid #3a3a3a;
    border-radius: 8px;
    color: #ccc;
    font-size: 0.85rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  .controls button:hover:not(:disabled) {
    background: #333;
    border-color: #444;
    color: #fff;
  }

  .controls button.active {
    background: #22c55e20;
    border-color: #22c55e;
    color: #22c55e;
  }

  .controls button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .status-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 14px;
    background: #1a1a1a;
    border-radius: 8px;
    font-size: 0.85rem;
  }

  .status-active {
    display: flex;
    align-items: center;
    gap: 8px;
    color: #22c55e;
    font-weight: 500;
  }

  .pulse {
    width: 8px;
    height: 8px;
    background: #22c55e;
    border-radius: 50%;
    animation: pulse 2s ease-in-out infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.3; }
  }

  .status-off {
    color: #666;
  }

  .off-btn {
    background: #ef444420;
    border: 1px solid #ef444440;
    color: #ef4444;
    padding: 4px 12px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.8rem;
    transition: all 0.2s;
  }

  .off-btn:hover:not(:disabled) {
    background: #ef444430;
  }

  .off-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
