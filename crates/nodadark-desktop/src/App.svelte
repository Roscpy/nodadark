<!-- src/App.svelte -->
<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { startProxy, stopProxy, togglePause, getStatus, subscribeToEvents, clearRequests as apiClearRequests } from "./lib/api/tauri";
  import { proxyRunning, proxyPaused, proxyPort, engineConnected, filterText, statusMessage, filteredRequests, selectedId, clearAll, caPath } from "./lib/stores/proxyStore";
  import RequestList from "./lib/components/RequestList.svelte";
  import RequestDetail from "./lib/components/RequestDetail.svelte";

  let unlistenEvents: (() => void) | null = null;
  let portInput = 8080;
  let showSettings = false;

  onMount(async () => {
    await getStatus();
    unlistenEvents = await subscribeToEvents();
    // Auto-démarrer si configuré
    if (!$proxyRunning) {
      await startProxy(portInput);
    }
  });

  onDestroy(() => {
    if (unlistenEvents) unlistenEvents();
  });

  async function handleStartStop() {
    if ($proxyRunning) {
      await stopProxy();
    } else {
      await startProxy(portInput);
    }
  }

  async function handlePause() {
    await togglePause();
  }

  async function handleClear() {
    await apiClearRequests();
    clearAll();
    statusMessage.set("🗑 Historique effacé");
  }

  // Raccourcis clavier globaux
  function handleKeydown(e: KeyboardEvent) {
    if (e.ctrlKey && e.key === "p") { e.preventDefault(); handlePause(); }
    if (e.ctrlKey && e.key === "l") { e.preventDefault(); document.getElementById("search-input")?.focus(); }
    if (e.ctrlKey && e.key === "q") { e.preventDefault(); window.close?.(); }
    if (e.key === "Escape") { selectedId.set(null); showSettings = false; }
  }
</script>

<svelte:window on:keydown={handleKeydown} />

<!-- ══ Toolbar ══════════════════════════════════════════════ -->
<header class="toolbar">
  <div class="toolbar-left">
    <span class="logo">⬡ NodaDark</span>
    <span class="version">v0.1</span>

    <!-- Bouton Start/Stop -->
    <button
      class="primary {$proxyRunning ? 'stop' : 'start'}"
      on:click={handleStartStop}
      title="Démarrer / Arrêter le proxy"
    >
      {$proxyRunning ? "■ Stop" : "▶ Start"}
    </button>

    <!-- Port input -->
    {#if !$proxyRunning}
      <input
        type="number"
        bind:value={portInput}
        min="1" max="65535"
        style="width:70px;"
        title="Port du proxy"
      />
    {:else}
      <span class="port-badge">:{$proxyPort}</span>
    {/if}

    <!-- Pause -->
    <button
      class="{$proxyPaused ? 'active' : ''}"
      on:click={handlePause}
      disabled={!$proxyRunning}
      title="Ctrl+P — Pause / Reprise"
    >
      {$proxyPaused ? "▶ Reprendre" : "⏸ Pause"}
    </button>

    <!-- Indicateur connexion -->
    <span class="conn-dot {$engineConnected ? 'connected' : 'disconnected'}"
      title={$engineConnected ? "Moteur connecté" : "Moteur déconnecté"}>
      {$engineConnected ? "● Connecté" : "○ Déconnecté"}
    </span>
  </div>

  <div class="toolbar-center">
    <!-- Filtre global -->
    <div class="search-wrap">
      <span class="search-icon">🔍</span>
      <input
        id="search-input"
        type="text"
        placeholder="Filtrer par URL, host, statut… (Ctrl+L)"
        bind:value={$filterText}
        style="width:360px;"
      />
      {#if $filterText}
        <button class="clear-filter" on:click={() => filterText.set("")}>✕</button>
      {/if}
    </div>
  </div>

  <div class="toolbar-right">
    <button on:click={handleClear} class="danger" title="Effacer l'historique">🗑 Effacer</button>
    <button on:click={() => showSettings = !showSettings} title="Paramètres">⚙</button>
  </div>
</header>

<!-- ══ Corps principal ══════════════════════════════════════ -->
<main class="main-body">
  <section class="pane-list">
    <RequestList />
  </section>

  {#if $selectedId}
    <section class="pane-detail">
      <RequestDetail />
    </section>
  {:else}
    <section class="pane-empty">
      <div class="empty-state">
        <p class="big-icon">⬡</p>
        <p class="empty-title">NodaDark</p>
        <p class="empty-sub">Configurez votre appareil pour utiliser le proxy sur <strong>127.0.0.1:{$proxyPort}</strong></p>
        {#if $caPath}
          <p class="ca-hint">Installez le CA racine : <code>{$caPath}</code></p>
        {/if}
        <p class="empty-sub hint">Sélectionnez une requête pour voir son détail.</p>
      </div>
    </section>
  {/if}
</main>

<!-- ══ Barre de statut ══════════════════════════════════════ -->
<footer class="statusbar">
  <span>{$statusMessage || "Prêt"}</span>
  <span class="stat-right">{$filteredRequests.length} requête(s)</span>
</footer>

<!-- ══ Panel Paramètres ══════════════════════════════════════ -->
{#if showSettings}
  <div class="overlay" on:click|self={() => showSettings = false}>
    <div class="modal">
      <h3>⚙ Paramètres</h3>
      <p style="color:var(--text-dim); margin-bottom:12px;">
        Certificat CA racine à installer sur vos appareils :
      </p>
      <code style="display:block; background:var(--bg); padding:8px; border-radius:4px; word-break:break-all; color:var(--accent);">
        {$caPath || "Proxy non démarré"}
      </code>
      <p style="margin-top:16px; color:var(--text-dim); font-size:11px;">
        Sur Android : Paramètres → Sécurité → Installer depuis la mémoire<br/>
        Sur iOS : Profil → Télécharger → Faire confiance dans Paramètres<br/>
        Sur Windows : Double-clic → Installer dans "Autorités racines de confiance"
      </p>
      <div style="margin-top:16px; text-align:right;">
        <button on:click={() => showSettings = false}>Fermer</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 6px 12px;
    background: var(--bg-panel);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .toolbar-left, .toolbar-center, .toolbar-right {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .logo { font-weight: bold; color: var(--accent); font-size: 15px; letter-spacing: 1px; }
  .version { color: var(--gray); font-size: 11px; }
  .port-badge { color: var(--cyan); font-weight: bold; padding: 3px 8px; border: 1px solid var(--cyan); border-radius: 3px; }
  .conn-dot { font-size: 11px; }
  .conn-dot.connected { color: var(--green); }
  .conn-dot.disconnected { color: var(--red); }
  .search-wrap { position: relative; display: flex; align-items: center; }
  .search-icon { position: absolute; left: 8px; color: var(--gray); pointer-events: none; }
  .search-wrap input { padding-left: 26px; }
  .clear-filter { position: absolute; right: 4px; background: none; border: none; color: var(--gray); padding: 2px 6px; cursor: pointer; }
  .clear-filter:hover { color: var(--red); }
  button.stop { background: rgba(220,50,50,0.2); border-color: var(--red); color: var(--red); }
  button.start { background: rgba(50,200,80,0.15); border-color: var(--green); color: var(--green); }

  .main-body {
    display: flex;
    flex: 1;
    overflow: hidden;
    min-height: 0;
  }
  .pane-list {
    width: 42%;
    min-width: 280px;
    border-right: 1px solid var(--border);
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }
  .pane-detail {
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }
  .pane-empty {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .empty-state { text-align: center; color: var(--text-dim); }
  .big-icon { font-size: 52px; margin-bottom: 12px; color: var(--border); }
  .empty-title { font-size: 20px; color: var(--accent); margin-bottom: 6px; }
  .empty-sub { margin-top: 6px; }
  .empty-sub strong { color: var(--cyan); }
  .ca-hint { margin-top: 10px; color: var(--yellow); font-size: 11px; }
  .ca-hint code { color: var(--cyan); }
  .hint { font-size: 11px; color: var(--gray); margin-top: 14px; }

  .statusbar {
    display: flex;
    justify-content: space-between;
    padding: 3px 12px;
    background: #0e1020;
    border-top: 1px solid var(--border);
    color: var(--gray);
    font-size: 11px;
    flex-shrink: 0;
  }
  .stat-right { color: var(--text-dim); }
</style>
