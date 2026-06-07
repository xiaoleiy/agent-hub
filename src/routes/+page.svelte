<script>
  import { invoke } from "@tauri-apps/api/core";
  import { onMount, onDestroy } from "svelte";
  import SystemStatus from "$lib/components/SystemStatus.svelte";
  import NetworkInfo from "$lib/components/NetworkInfo.svelte";
  import AgentCard from "$lib/components/AgentCard.svelte";
  import SessionList from "$lib/components/SessionList.svelte";
  import UsageChart from "$lib/components/UsageChart.svelte";
  import KeepAlive from "$lib/components/KeepAlive.svelte";

  let systemStatus = $state(null);
  let networkInfo = $state(null);
  let agents = $state([]);
  let sessions = $state([]);
  let usageWindow = $state("5h");
  let usageStats = $state([]);
  let keepAliveStatus = $state(null);

  let pollInterval;
  let networkLoaded = false;

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

  async function fetchSessions() {
    try {
      // Fetch sessions for each agent individually
      const allSessions = [];
      const agentNames = ["claude", "cursor", "codex"];
      for (const name of agentNames) {
        try {
          const s = await invoke("get_agent_sessions", { agent: name });
          allSessions.push(...s);
        } catch (_) { /* agent may not be installed */ }
      }
      sessions = allSessions;
    } catch (e) {
      console.error("Failed to fetch sessions:", e);
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

  async function fetchUsage() {
    if (agents.length === 0) return;
    try {
      const allUsage = [];
      for (const agent of agents) {
        try {
          const u = await invoke("get_agent_usage", {
            agent: agent.name,
            window: usageWindow,
          });
          allUsage.push(u);
        } catch (_) { /* skip */ }
      }
      usageStats = allUsage;
    } catch (e) {
      console.error("Failed to fetch usage:", e);
    }
  }

  async function fetchAll() {
    await Promise.all([fetchSystem(), fetchAgents(), fetchSessions(), fetchKeepAlive()]);
  }

  // Fetch usage whenever agents or window changes
  $effect(() => {
    // Access reactive deps
    const _ = agents;
    const __ = usageWindow;
    fetchUsage();
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

  <section class="card agents-section">
    <h2>Agents</h2>
    <div class="agent-grid">
      {#each agents as agent (agent.name)}
        <AgentCard {agent} />
      {/each}
    </div>
  </section>

  <section class="card usage-section">
    <h2>Usage</h2>
    <div class="window-tabs">
      <button
        class:active={usageWindow === "5h"}
        onclick={() => (usageWindow = "5h")}
      >
        5 Hours
      </button>
      <button
        class:active={usageWindow === "1w"}
        onclick={() => (usageWindow = "1w")}
      >
        1 Week
      </button>
      <button
        class:active={usageWindow === "1m"}
        onclick={() => (usageWindow = "1m")}
      >
        1 Month
      </button>
    </div>
    <UsageChart stats={usageStats} window={usageWindow} />
  </section>

  <section class="card sessions-section">
    <h2>Active Sessions</h2>
    <SessionList {sessions} />
  </section>

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

  .card h2 {
    font-size: 1.1rem;
    font-weight: 600;
    color: #ffffff;
    margin-bottom: 12px;
  }

  .agent-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 12px;
  }

  .window-tabs {
    display: flex;
    gap: 8px;
    margin-bottom: 12px;
  }

  .window-tabs button {
    background: #2a2a2a;
    border: 1px solid #3a3a3a;
    color: #aaa;
    padding: 6px 16px;
    border-radius: 8px;
    cursor: pointer;
    font-size: 0.85rem;
    transition: all 0.2s;
  }

  .window-tabs button:hover {
    background: #333;
    color: #fff;
  }

  .window-tabs button.active {
    background: #2563eb;
    border-color: #2563eb;
    color: #fff;
  }
</style>
