<script>
  import { invoke } from "@tauri-apps/api/core";
  import { onMount, onDestroy } from "svelte";
  import SystemStatus from "$lib/components/SystemStatus.svelte";
  import NetworkInfo from "$lib/components/NetworkInfo.svelte";
  import AgentTab from "$lib/components/AgentTab.svelte";
  import ProxyInfo from "$lib/components/ProxyInfo.svelte";
  import KeepAlive from "$lib/components/KeepAlive.svelte";

  let systemStatus = $state(null);
  let networkInfo = $state(null);
  let agents = $state(/** @type {any[]} */ ([]));
  let keepAliveStatus = $state(null);
  let activeTab = $state(0);

  /** @type {ReturnType<typeof setInterval> | undefined} */
  let pollInterval;
  let networkLoaded = false;

  // Only show tabs for installed agents
  const availableAgents = $derived(agents.filter((a) => a.installed));

  // Build tab list: agent tabs + proxy tab
  const tabs = $derived(() => {
    const list = availableAgents.map((a, i) => ({
      kind: "agent",
      label: a.name,
      running: a.running,
      index: i,
    }));
    list.push({ kind: "proxy", label: "Proxy / VPN", running: false, index: list.length });
    return list;
  });

  async function fetchSystem() {
    try {
      systemStatus = await invoke("get_system_status");
    } catch (e) {
      console.error("Failed to fetch system status:", e);
    }
  }

  async function fetchAgents() {
    try {
      agents = await invoke("get_agents");
    } catch (e) {
      console.error("Failed to fetch agents:", e);
    }
  }

  async function fetchKeepAlive() {
    try {
      keepAliveStatus = await invoke("get_keepalive_status");
    } catch (e) {
      console.error("Failed to fetch keepalive:", e);
    }
  }

  async function fetchNetwork() {
    if (networkLoaded) return;
    try {
      networkInfo = await invoke("get_network_info");
      networkLoaded = true;
    } catch (e) {
      console.error("Failed to fetch network info:", e);
    }
  }

  async function fetchAll() {
    await Promise.all([fetchSystem(), fetchAgents(), fetchKeepAlive()]);
  }

  // Clamp active tab when tabs change
  $effect(() => {
    const count = tabs().length;
    if (count > 0 && activeTab >= count) {
      activeTab = count - 1;
    }
  });

  onMount(() => {
    fetchAll();
    fetchNetwork();
    pollInterval = setInterval(() => {
      fetchAll();
    }, 5000);
  });

  onDestroy(() => {
    if (pollInterval) clearInterval(pollInterval);
  });
</script>

<main class="app">
  <header>
    <h1>Agent Hub</h1>
    <p class="subtitle">Manage AI coding agents on your machine</p>
  </header>

  <div class="grid">
    <section class="card system-card">
      <SystemStatus status={systemStatus} />
    </section>

    <section class="card network-card">
      <NetworkInfo info={networkInfo} />
    </section>
  </div>

  <!-- Tabs -->
  {#if tabs().length > 0}
    <section class="card tabs-section">
      <div class="tab-bar">
        {#each tabs() as tab, i}
          <button
            class="tab-btn"
            class:active={activeTab === i}
            onclick={() => (activeTab = i)}
          >
            {#if tab.kind === "agent"}
              <span class="tab-dot" class:running={tab.running}></span>
            {:else}
              <span class="tab-icon">🔒</span>
            {/if}
            {tab.label}
          </button>
        {/each}
      </div>

      <div class="tab-content">
        {#if tabs()[activeTab]?.kind === "agent"}
          {@const agentIdx = tabs()[activeTab].index}
          {#if availableAgents[agentIdx]}
            <AgentTab agent={availableAgents[agentIdx]} />
          {/if}
        {:else if tabs()[activeTab]?.kind === "proxy"}
          <ProxyInfo />
        {/if}
      </div>
    </section>
  {/if}

  <section class="card keepalive-section">
    <KeepAlive status={keepAliveStatus} />
  </section>
</main>

<style>
  :global(*) {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
  }

  :global(body) {
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto,
      "Helvetica Neue", Arial, sans-serif;
    background: #0f0f0f;
    color: #e0e0e0;
    line-height: 1.5;
  }

  .app {
    max-width: 960px;
    margin: 0 auto;
    padding: 20px;
  }

  header {
    margin-bottom: 24px;
  }

  header h1 {
    font-size: 1.8rem;
    font-weight: 700;
    color: #ffffff;
  }

  .subtitle {
    color: #888;
    font-size: 0.9rem;
    margin-top: 2px;
  }

  .grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px;
    margin-bottom: 16px;
  }

  .card {
    background: #1a1a1a;
    border: 1px solid #2a2a2a;
    border-radius: 12px;
    padding: 16px;
    margin-bottom: 16px;
  }

  .tab-bar {
    display: flex;
    gap: 4px;
    margin-bottom: 16px;
    border-bottom: 1px solid #2a2a2a;
    padding-bottom: 0;
    overflow-x: auto;
  }

  .tab-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 16px;
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    color: #888;
    font-size: 0.9rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
    margin-bottom: -1px;
    white-space: nowrap;
  }

  .tab-btn:hover {
    color: #ccc;
  }

  .tab-btn.active {
    color: #fff;
    border-bottom-color: #2563eb;
  }

  .tab-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #666;
    flex-shrink: 0;
  }

  .tab-dot.running {
    background: #22c55e;
  }

  .tab-icon {
    font-size: 0.75rem;
    flex-shrink: 0;
  }

  .tab-content {
    min-height: 200px;
  }
</style>
