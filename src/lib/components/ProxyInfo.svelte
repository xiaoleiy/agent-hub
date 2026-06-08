<script>
  import { invoke } from "@tauri-apps/api/core";
  import { onMount, onDestroy } from "svelte";

  let info = $state(/** @type {any} */ (null));
  let error = $state(/** @type {string|null} */ (null));
  let expandedNode = $state(/** @type {string|null} */ (null));

  let pollInterval = /** @type {any} */ (undefined);

  async function fetchProxy() {
    try {
      info = await invoke("get_proxy_info");
      error = null;
    } catch (e) {
      error = String(e);
    }
  }

  onMount(() => {
    fetchProxy();
    pollInterval = setInterval(fetchProxy, 10000);
  });

  onDestroy(() => {
    if (pollInterval) clearInterval(pollInterval);
  });

  /** @param {string} name */
  function toggleNode(name) {
    expandedNode = expandedNode === name ? null : name;
  }

  /** @param {number|undefined} ms */
  function formatDelay(ms) {
    if (ms === undefined || ms === null) return "—";
    if (ms >= 1000) return `${(ms / 1000).toFixed(1)}s`;
    return `${ms}ms`;
  }

  /** @param {number|undefined} ms */
  function delayColor(ms) {
    if (!ms) return "var(--text-dim)";
    if (ms <= 100) return "var(--ok)";
    if (ms <= 300) return "var(--warn)";
    return "var(--danger)";
  }
</script>

<div class="proxy-info">
  {#if error}
    <p class="error">Failed to load proxy info: {error}</p>
  {:else if !info}
    <p class="loading">Loading proxy information...</p>
  {:else}
    <!-- System Proxy -->
    <div class="section">
      <h3>System Proxy</h3>
      <div class="service-name">
        <span class="label">Active Service</span>
        <span class="value">{info.system_proxy.active_service}</span>
      </div>

      <div class="proxy-grid">
        <!-- HTTP -->
        <div class="proxy-card" class:enabled={info.system_proxy.http.enabled}>
          <div class="proxy-type">HTTP</div>
          {#if info.system_proxy.http.enabled}
            <div class="proxy-addr">{info.system_proxy.http.server}:{info.system_proxy.http.port}</div>
          {:else}
            <div class="proxy-off">Disabled</div>
          {/if}
        </div>

        <!-- HTTPS -->
        <div class="proxy-card" class:enabled={info.system_proxy.https.enabled}>
          <div class="proxy-type">HTTPS</div>
          {#if info.system_proxy.https.enabled}
            <div class="proxy-addr">{info.system_proxy.https.server}:{info.system_proxy.https.port}</div>
          {:else}
            <div class="proxy-off">Disabled</div>
          {/if}
        </div>

        <!-- SOCKS -->
        <div class="proxy-card" class:enabled={info.system_proxy.socks.enabled}>
          <div class="proxy-type">SOCKS</div>
          {#if info.system_proxy.socks.enabled}
            <div class="proxy-addr">{info.system_proxy.socks.server}:{info.system_proxy.socks.port}</div>
          {:else}
            <div class="proxy-off">Disabled</div>
          {/if}
        </div>

        <!-- PAC -->
        <div class="proxy-card" class:enabled={info.system_proxy.pac}>
          <div class="proxy-type">PAC</div>
          {#if info.system_proxy.pac}
            <div class="proxy-addr pac-url" title={info.system_proxy.pac}>{info.system_proxy.pac}</div>
          {:else}
            <div class="proxy-off">Disabled</div>
          {/if}
        </div>
      </div>

      {#if info.system_proxy.bypass.length > 0}
        <div class="bypass">
          <span class="label">Bypass</span>
          <span class="bypass-list">{info.system_proxy.bypass.join(", ")}</span>
        </div>
      {/if}
    </div>

    <!-- VPN Connections -->
    {#if info.vpn_connections.length > 0}
      <div class="section">
        <h3>VPN Connections</h3>
        <div class="vpn-list">
          {#each info.vpn_connections as vpn}
            <div class="vpn-item" class:connected={vpn.connected}>
              <span class="vpn-status" class:connected={vpn.connected}>
                {vpn.connected ? "●" : "○"}
              </span>
              <span class="vpn-name">{vpn.name}</span>
              <span class="vpn-type">{vpn.vpn_type}</span>
              <span class="vpn-state">{vpn.connected ? "Connected" : "Disconnected"}</span>
            </div>
          {/each}
        </div>
      </div>
    {/if}

    <!-- Active Proxy Client -->
    {#if info.active_client}
      <div class="section">
        <h3>Proxy Client</h3>
        <div class="client-card">
          <div class="client-header">
            <span class="client-name">{info.active_client.name}</span>
            {#if info.active_client.version}
              <span class="client-ver">v{info.active_client.version.replace(/^v/, "")}</span>
            {/if}
            <span class="client-type">{info.active_client.client_type}</span>
          </div>
          <div class="client-details">
            <span class="detail">
              {#if info.active_client.api_socket}
                <span class="detail-label">API</span>
                <span class="detail-value">unix socket</span>
              {:else}
                <span class="detail-label">API Port</span>
                <span class="detail-value">{info.active_client.api_port}</span>
              {/if}
            </span>
            {#if info.active_client.mode}
              <span class="detail">
                <span class="detail-label">Mode</span>
                <span class="detail-value">{info.active_client.mode}</span>
              </span>
            {/if}
          </div>
        </div>
      </div>
    {:else}
      <div class="section">
        <h3>Proxy Client</h3>
        <p class="no-client">No proxy client detected</p>
      </div>
    {/if}

    <!-- Proxy Nodes (from client API) -->
    {#if info.proxy_nodes.length > 0}
      <div class="section">
        <h3>Proxy Nodes</h3>
        <div class="node-list">
          {#each info.proxy_nodes as node}
            <div class="node-group">
              <button class="node-header" onclick={() => toggleNode(node.name)}>
                <span class="node-name">{node.name}</span>
                <span class="node-type">{node.node_type}</span>
                <span class="node-selected">{node.selected}</span>
                {#if node.delay !== undefined && node.delay !== null}
                  <span class="node-delay" style="color: {delayColor(node.delay)}">
                    {formatDelay(node.delay)}
                  </span>
                {/if}
                <span class="expand-icon">{expandedNode === node.name ? "▾" : "▸"}</span>
              </button>

              {#if expandedNode === node.name && node.available_nodes.length > 0}
                <div class="node-children">
                  {#each node.available_nodes as child}
                    <div class="node-child" class:selected={child === node.selected}>
                      {#if child === node.selected}
                        <span class="child-check">✓</span>
                      {:else}
                        <span class="child-spacer"></span>
                      {/if}
                      <span class="child-name">{child}</span>
                    </div>
                  {/each}
                </div>
              {/if}
            </div>
          {/each}
        </div>
      </div>
    {/if}
  {/if}
</div>

<style>
  .proxy-info {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  /* Flat sections (no outer card) — consistent with the agent tabs; the inner
     cards (proxy-card, vpn-item, client-card, node-group) carry the surfaces. */
  .section h3 {
    font-size: 0.85rem;
    font-weight: 600;
    color: var(--text-strong);
    margin-bottom: 8px;
  }

  .service-name {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 8px;
    font-size: 0.82rem;
  }

  .label {
    color: var(--text-muted);
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .value {
    color: var(--text-strong);
    font-weight: 500;
  }

  .proxy-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }

  .proxy-card {
    background: var(--surface-2);
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    padding: 8px 10px;
    transition: border-color 0.2s;
  }

  .proxy-card.enabled {
    border-color: var(--accent-tint-strong);
  }

  .proxy-type {
    font-size: 0.7rem;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin-bottom: 4px;
  }

  .proxy-addr {
    font-family: "SF Mono", "Fira Code", monospace;
    font-size: 0.8rem;
    color: var(--info);
    word-break: break-all;
  }

  .proxy-addr.pac-url {
    font-size: 0.7rem;
    color: var(--text-muted);
    max-height: 2.4em;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .proxy-off {
    font-size: 0.8rem;
    color: var(--text-dim);
  }

  .bypass {
    margin-top: 10px;
    font-size: 0.8rem;
    display: flex;
    gap: 8px;
    align-items: baseline;
  }

  .bypass-list {
    color: var(--text-muted);
    font-size: 0.75rem;
    word-break: break-all;
  }

  .vpn-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .vpn-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 12px;
    background: var(--surface-2);
    border-radius: 8px;
    border: 1px solid var(--border-strong);
    font-size: 0.85rem;
    contain: content;
  }

  .vpn-item.connected {
    border-color: var(--ok-tint-strong);
  }

  .vpn-status {
    font-size: 0.7rem;
    color: var(--text-dim);
  }

  .vpn-status.connected {
    color: var(--ok);
  }

  .vpn-name {
    font-weight: 600;
    color: var(--text-strong);
    flex: 1;
  }

  .vpn-type {
    font-size: 0.7rem;
    background: var(--border-strong);
    color: var(--text-muted);
    padding: 2px 6px;
    border-radius: 4px;
  }

  .vpn-state {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .client-card {
    background: var(--surface-2);
    border: 1px solid var(--accent-tint-strong);
    border-radius: 8px;
    padding: 12px;
  }

  .client-header {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 8px;
  }

  .client-name {
    font-weight: 700;
    font-size: 1rem;
    color: var(--text-strong);
  }

  .client-ver {
    font-family: "SF Mono", "Fira Code", monospace;
    font-size: 0.75rem;
    color: var(--info);
    background: var(--info-tint);
    padding: 2px 6px;
    border-radius: 4px;
  }

  .client-type {
    font-size: 0.7rem;
    background: var(--border-strong);
    color: var(--text-muted);
    padding: 2px 6px;
    border-radius: 4px;
    text-transform: uppercase;
  }

  .client-details {
    display: flex;
    gap: 16px;
    font-size: 0.8rem;
  }

  .detail {
    display: flex;
    gap: 6px;
    align-items: center;
  }

  .detail-label {
    color: var(--text-muted);
  }

  .detail-value {
    color: var(--text-strong);
    font-family: "SF Mono", "Fira Code", monospace;
    font-size: 0.8rem;
  }

  .no-client {
    color: var(--text-dim);
    font-size: 0.85rem;
  }

  .node-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .node-group {
    border: 1px solid var(--surface-3);
    border-radius: 8px;
    overflow: hidden;
    contain: content;
  }

  .node-header {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 10px 12px;
    background: var(--surface-2);
    border: none;
    color: var(--text);
    font-size: 0.85rem;
    cursor: pointer;
    text-align: left;
    transition: background 0.15s;
  }

  .node-header:hover {
    background: var(--surface-3);
  }

  .node-name {
    font-weight: 600;
    color: var(--text-strong);
    flex: 1;
  }

  .node-type {
    font-size: 0.65rem;
    background: var(--border-strong);
    color: var(--text-muted);
    padding: 2px 6px;
    border-radius: 4px;
    text-transform: uppercase;
  }

  .node-selected {
    font-family: "SF Mono", "Fira Code", monospace;
    font-size: 0.8rem;
    color: var(--info);
    max-width: 200px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .node-delay {
    font-family: "SF Mono", "Fira Code", monospace;
    font-size: 0.8rem;
    font-weight: 500;
    min-width: 50px;
    text-align: right;
  }

  .expand-icon {
    font-size: 0.7rem;
    color: var(--text-dim);
    width: 12px;
  }

  .node-children {
    background: var(--surface);
    border-top: 1px solid var(--surface-3);
    max-height: 200px;
    overflow-y: auto;
  }

  .node-child {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 12px 6px 24px;
    font-size: 0.8rem;
    color: var(--text-muted);
    border-bottom: 1px solid var(--border);
  }

  .node-child.selected {
    color: var(--info);
    font-weight: 500;
    background: var(--info-tint-weak);
  }

  .child-check {
    color: var(--info);
    font-size: 0.75rem;
    width: 14px;
  }

  .child-spacer {
    width: 14px;
  }

  .child-name {
    flex: 1;
    font-family: "SF Mono", "Fira Code", monospace;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .error {
    color: var(--danger);
    font-size: 0.85rem;
    text-align: center;
    padding: 20px;
  }

  .loading {
    color: var(--text-dim);
    font-size: 0.9rem;
    text-align: center;
    padding: 30px;
  }
</style>
