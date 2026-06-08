<script>
  import { invoke } from "@tauri-apps/api/core";

  /** @type {{ status: any }} */
  let { status } = $props();

  const modes = [
    { label: "30 min", value: "30m" },
    { label: "1 hour", value: "1h" },
    { label: "3 hours", value: "3h" },
    { label: "Forever", value: "forever" },
  ];

  let loading = $state(false);

  async function setMode(/** @type {string} */ mode) {
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
    font-size: 0.95rem;
    font-weight: 600;
    color: var(--text-strong);
    margin-bottom: 3px;
  }

  .description {
    font-size: 0.78rem;
    color: var(--text-muted);
    margin-bottom: 10px;
  }

  .controls {
    display: flex;
    gap: 6px;
    margin-bottom: 10px;
  }

  .controls button {
    flex: 1;
    padding: 7px 10px;
    background: var(--surface-3);
    border: 1px solid var(--border-strong);
    border-radius: 7px;
    color: var(--text);
    font-size: 0.82rem;
    cursor: pointer;
    transition: color 0.15s, background 0.15s, border-color 0.15s;
  }

  .controls button:hover:not(:disabled) {
    background: var(--border-strong);
    border-color: var(--text-dim);
    color: var(--text-strong);
  }

  .controls button.active {
    background: var(--ok-tint);
    border-color: var(--ok);
    color: var(--ok);
  }

  .controls button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .status-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px;
    background: var(--surface-2);
    border: 1px solid var(--surface-3);
    border-radius: 8px;
    font-size: 0.82rem;
  }

  .status-active {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--ok);
    font-weight: 500;
  }

  .pulse {
    width: 8px;
    height: 8px;
    background: var(--ok);
    border-radius: 50%;
    animation: pulse 2s ease-in-out infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.3; }
  }

  .status-off {
    color: var(--text-dim);
  }

  .off-btn {
    background: var(--danger-tint);
    border: 1px solid var(--danger-tint-strong);
    color: var(--danger);
    padding: 4px 12px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.8rem;
    transition: background 0.15s, border-color 0.15s;
  }

  .off-btn:hover:not(:disabled) {
    background: var(--danger-tint-hover);
  }

  .off-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
