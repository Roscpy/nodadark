<!-- src/lib/components/RequestList.svelte -->
<script lang="ts">
  import { filteredRequests, selectedId, filterText, getStatusClass, getMethodClass } from "../stores/proxyStore";

  function selectRequest(id: string) {
    selectedId.set(id);
  }

  function handleKeydown(e: KeyboardEvent, id: string) {
    if (e.key === "Enter" || e.key === " ") selectRequest(id);
  }

  function statusLabel(s: number | null): string {
    return s ? String(s) : "···";
  }

  function durationLabel(ms: number | null): string {
    if (ms === null) return "···";
    if (ms < 1000) return `${ms}ms`;
    return `${(ms / 1000).toFixed(1)}s`;
  }

  function truncate(s: string, n: number): string {
    return s.length > n ? s.slice(0, n) + "…" : s;
  }
</script>

<div class="list-header">
  <span class="col-method">Method</span>
  <span class="col-status">Status</span>
  <span class="col-host">Host / Path</span>
  <span class="col-dur">Durée</span>
</div>

<div class="list-body">
  {#each $filteredRequests as req (req.id)}
    <!-- svelte-ignore a11y-no-noninteractive-element-interactions -->
    <div
      role="listitem"
      class="row {req.state} {$selectedId === req.id ? 'selected' : ''}"
      on:click={() => selectRequest(req.id)}
      on:keydown={(e) => handleKeydown(e, req.id)}
      tabindex="0"
    >
      <span class="col-method">
        {#if req.tls}<span class="tls-icon">🔒</span>{/if}
        <span class={getMethodClass(req.method)}>{req.method}</span>
      </span>

      <span class="col-status {getStatusClass(req.response_status)}">
        {statusLabel(req.response_status)}
      </span>

      <span class="col-host" title={req.url}>
        <span class="host">{truncate(req.host, 26)}</span>
        <span class="path">{truncate(req.path || "/", 22)}</span>
      </span>

      <span class="col-dur">{durationLabel(req.duration_ms)}</span>
    </div>
  {:else}
    <div class="empty">Aucune requête interceptée.</div>
  {/each}
</div>

<style>
  .list-header {
    display: flex;
    padding: 5px 10px;
    background: var(--bg);
    border-bottom: 1px solid var(--border);
    color: var(--gray);
    font-size: 11px;
    font-weight: bold;
    flex-shrink: 0;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .list-body {
    flex: 1;
    overflow-y: auto;
  }

  .row {
    display: flex;
    align-items: center;
    padding: 5px 10px;
    border-bottom: 1px solid rgba(40,72,130,0.3);
    cursor: pointer;
    transition: background var(--transition);
    outline: none;
  }
  .row:hover     { background: var(--bg-hover); }
  .row.selected  { background: var(--bg-select); }
  .row.dropped   { opacity: 0.4; }
  .row.error     { border-left: 2px solid var(--red); }

  .col-method { width: 90px; flex-shrink: 0; display: flex; align-items: center; gap: 4px; }
  .col-status { width: 52px; flex-shrink: 0; font-weight: bold; }
  .col-host   { flex: 1; overflow: hidden; display: flex; flex-direction: column; }
  .col-dur    { width: 60px; flex-shrink: 0; color: var(--gray); font-size: 11px; text-align: right; }

  .host { color: var(--text); font-size: 12px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .path { color: var(--gray); font-size: 11px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }

  .tls-icon { font-size: 10px; }

  .empty { padding: 30px; text-align: center; color: var(--gray); }
</style>
