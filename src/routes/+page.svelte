<script>
  import { invoke } from "@tauri-apps/api/core";
  import { onMount, onDestroy } from "svelte";
  import SystemStatus from "$lib/components/SystemStatus.svelte";
  import NetworkInfo from "$lib/components/NetworkInfo.svelte";
  import AgentTab from "$lib/components/AgentTab.svelte";
  import ProxyInfo from "$lib/components/ProxyInfo.svelte";
  import KeepAlive from "$lib/components/KeepAlive.svelte";
  import "$lib/styles/panels.css";

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
      const next = await invoke("get_agents");
      // Only reassign when the payload actually changed. The poll fires every
      // 5s; reassigning to a brand-new array/objects each time forces every
      // AgentTab prop to a new reference and re-renders the whole tab — a
      // poll mid-scroll caused jank. Payload is small, so a stringify compare
      // is cheap and avoids needless re-renders when nothing changed.
      if (JSON.stringify(next) !== JSON.stringify(agents)) {
        agents = /** @type {any[]} */ (next);
      }
    } catch (e) {
      console.error("Failed to fetch agents:", e);
    }
  }

  async function fetchKeepAlive() {
    try {
      const next = await invoke("get_keepalive_status");
      if (JSON.stringify(next) !== JSON.stringify(keepAliveStatus)) {
        keepAliveStatus = next;
      }
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
    <img class="app-icon" src="/app-icon.png" alt="Agent Hub" />
    <div class="header-text">
      <h1>Agent Hub</h1>
      <p class="subtitle">Manage AI coding agents on your machine</p>
    </div>
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

  /* ===== Semantic theme tokens =====
     Dark is the default (matches the original look). Light is applied
     automatically via prefers-color-scheme, and a manual override hook is
     exposed through html[data-theme="light"|"dark"]. System/auto is default. */
  :global(:root) {
    /* Surfaces */
    --bg: #0f0f0f;
    --surface: #1a1a1a;
    --surface-2: #222222;
    --surface-3: #2a2a2a;

    /* Borders */
    --border: #262626;
    --border-strong: #333333;

    /* Text */
    --text: #e0e0e0;
    --text-strong: #ffffff;
    --text-muted: #888888;
    --text-dim: #666666;

    /* Accents (hue preserved across themes) */
    --accent: #2563eb;
    --ok: #22c55e;
    --info: #0ea5e9;
    --warn: #eab308;
    --danger: #ef4444;
    --purple: #a855f7;

    /* Translucent accent-tint chip backgrounds */
    --accent-tint: #2563eb20;
    --accent-tint-strong: #2563eb40;
    --ok-tint: #22c55e20;
    --ok-tint-strong: #22c55e40;
    --info-tint: #0ea5e920;
    --info-tint-weak: #0ea5e910;
    --warn-tint: #eab30820;
    --danger-tint: #ef444420;
    --danger-tint-strong: #ef444440;
    --danger-tint-hover: #ef444430;
    --purple-tint: #a855f720;
  }

  /* Light palette — applied for system light or explicit data-theme="light". */
  :global(:root[data-theme="light"]) {
    --bg: #f4f4f6;
    --surface: #ffffff;
    --surface-2: #f1f1f4;
    --surface-3: #e7e7ec;

    --border: #e3e3e8;
    --border-strong: #d6d6dc;

    --text: #1d1d1f;
    --text-strong: #000000;
    --text-muted: #6b6b73;
    --text-dim: #97979f;

    /* Slightly darkened accent text for contrast on light surfaces. */
    --accent: #1d4ed8;
    --ok: #15803d;
    --info: #0369a1;
    --warn: #a16207;
    --danger: #dc2626;
    --purple: #9333ea;

    /* Tints bumped to ~18% so chips read on white surfaces. */
    --accent-tint: color-mix(in srgb, #2563eb 16%, var(--surface));
    --accent-tint-strong: color-mix(in srgb, #2563eb 32%, var(--surface));
    --ok-tint: color-mix(in srgb, #22c55e 18%, var(--surface));
    --ok-tint-strong: color-mix(in srgb, #22c55e 34%, var(--surface));
    --info-tint: color-mix(in srgb, #0ea5e9 18%, var(--surface));
    --info-tint-weak: color-mix(in srgb, #0ea5e9 10%, var(--surface));
    --warn-tint: color-mix(in srgb, #eab308 20%, var(--surface));
    --danger-tint: color-mix(in srgb, #ef4444 16%, var(--surface));
    --danger-tint-strong: color-mix(in srgb, #ef4444 32%, var(--surface));
    --danger-tint-hover: color-mix(in srgb, #ef4444 24%, var(--surface));
    --purple-tint: color-mix(in srgb, #a855f7 18%, var(--surface));
  }

  @media (prefers-color-scheme: light) {
    /* System light: only when not explicitly forced to dark. */
    :global(:root:not([data-theme="dark"])) {
      --bg: #f4f4f6;
      --surface: #ffffff;
      --surface-2: #f1f1f4;
      --surface-3: #e7e7ec;

      --border: #e3e3e8;
      --border-strong: #d6d6dc;

      --text: #1d1d1f;
      --text-strong: #000000;
      --text-muted: #6b6b73;
      --text-dim: #97979f;

      --accent: #1d4ed8;
      --ok: #15803d;
      --info: #0369a1;
      --warn: #a16207;
      --danger: #dc2626;
      --purple: #9333ea;

      --accent-tint: color-mix(in srgb, #2563eb 16%, var(--surface));
      --accent-tint-strong: color-mix(in srgb, #2563eb 32%, var(--surface));
      --ok-tint: color-mix(in srgb, #22c55e 18%, var(--surface));
      --ok-tint-strong: color-mix(in srgb, #22c55e 34%, var(--surface));
      --info-tint: color-mix(in srgb, #0ea5e9 18%, var(--surface));
      --info-tint-weak: color-mix(in srgb, #0ea5e9 10%, var(--surface));
      --warn-tint: color-mix(in srgb, #eab308 20%, var(--surface));
      --danger-tint: color-mix(in srgb, #ef4444 16%, var(--surface));
      --danger-tint-strong: color-mix(in srgb, #ef4444 32%, var(--surface));
      --danger-tint-hover: color-mix(in srgb, #ef4444 24%, var(--surface));
      --purple-tint: color-mix(in srgb, #a855f7 18%, var(--surface));
    }
  }

  /* Compact base scale tuned for the ~500px menu-bar popover. */
  :global(html) {
    font-size: 13px;
  }

  :global(body) {
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto,
      "Helvetica Neue", Arial, sans-serif;
    background: var(--bg);
    color: var(--text);
    line-height: 1.45;
  }

  .app {
    max-width: 960px;
    margin: 0 auto;
    padding: 12px 12px 16px;
  }

  header {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 12px;
  }

  .app-icon {
    width: 36px;
    height: 36px;
    border-radius: 8px;
    flex-shrink: 0;
    /* Crisp rendering of the 128px source shown small. */
    image-rendering: -webkit-optimize-contrast;
  }

  .header-text {
    min-width: 0;
  }

  header h1 {
    font-size: 1.4rem;
    font-weight: 700;
    letter-spacing: -0.01em;
    color: var(--text-strong);
  }

  .subtitle {
    color: var(--text-muted);
    font-size: 0.82rem;
    margin-top: 1px;
  }

  .grid {
    display: grid;
    /* Collapses to one column at popover width, two when wider. */
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: 8px;
    margin-bottom: 8px;
    align-items: stretch;
  }

  .grid .card {
    margin-bottom: 0;
    height: 100%;
  }

  .card {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 9px;
    padding: 12px;
    margin-bottom: 8px;
    /* Isolate each card's layout/paint so a subtree update (e.g. a poll
       refreshing one section) doesn't reflow the whole scrolling page. */
    contain: content;
  }

  .tab-bar {
    display: flex;
    gap: 2px;
    margin-bottom: 10px;
    border-bottom: 1px solid var(--surface-3);
    padding-bottom: 0;
    overflow-x: auto;
  }

  .tab-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 11px;
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    color: var(--text-muted);
    font-size: 0.85rem;
    font-weight: 500;
    cursor: pointer;
    transition: color 0.15s, border-bottom-color 0.15s;
    margin-bottom: -1px;
    white-space: nowrap;
  }

  .tab-btn:hover {
    color: var(--text);
  }

  .tab-btn.active {
    color: var(--text-strong);
    border-bottom-color: var(--accent);
  }

  .tab-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--text-dim);
    flex-shrink: 0;
  }

  .tab-dot.running {
    background: var(--ok);
  }

  .tab-icon {
    font-size: 0.75rem;
    flex-shrink: 0;
  }

  .tab-content {
    min-height: 80px;
    contain: layout style;
  }
</style>
