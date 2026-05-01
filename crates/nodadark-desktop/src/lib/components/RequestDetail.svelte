<!-- src/lib/components/RequestDetail.svelte -->
<script lang="ts">
  import { selectedRequest, selectedId, getStatusClass, bytesToString, formatSize, statusMessage } from "../stores/proxyStore";
  import { replayRequest, dropRequest } from "../api/tauri";
  import HexViewer from "./HexViewer.svelte";
  import CookieEditor from "./CookieEditor.svelte";

  type Tab = "headers" | "body" | "hex";
  let activeTab: Tab = "headers";
  let showCookieEditor = false;

  $: req = $selectedRequest;

  function formatHeaders(headers: [string, string][]): string {
    return headers.map(([k, v]) => `${k}: ${v}`).join("\n");
  }

  function formatBody(bytes: number[] | null): string {
    if (!bytes || bytes.length === 0) return "(Body vide)";
    const raw = bytesToString(bytes);
    try {
      return JSON.stringify(JSON.parse(raw), null, 2);
    } catch {
      return raw;
    }
  }

  function isHighlightedHeader(name: string): boolean {
    const n = name.toLowerCase();
    return n === "cookie" || n === "authorization" || n === "set-cookie" || n === "x-auth-token";
  }

  async function handleReplay() {
    if (!req) return;
    await replayRequest(req.id);
  }

  async function handleDrop() {
    if (!req) return;
    await dropRequest(req.id);
    selectedId.set(null);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "1") activeTab = "headers";
    if (e.key === "2") activeTab = "body";
    if (e.key === "3") activeTab = "hex";
    if (e.ctrlKey && e.key === "r") { e.preventDefault(); handleReplay(); }
  }
</script>

<svelte:window on:keydown={handleKeydown} />

{#if req}
<!-- ── Info bar ── -->
<div class="detail-info">
  <span class="method-badge method-{req.method}">{req.method}</span>
  {#if req.tls}<span class="tls">🔒 HTTPS</span>{:else}<span class="http">HTTP</span>{/if}
  <span class="url" title={req.url}>{req.url}</span>
  {#if req.response_status}
    <span class="status {getStatusClass(req.response_status)}">
      → {req.response_status}
    </span>
  {:else}
    <span class="status status-pending">→ ···</span>
  {/if}
  {#if req.duration_ms}
    <span class="duration">{req.duration_ms}ms</span>
  {/if}
  <span class="resp-size">{formatSize(req.response_body)}</span>
</div>

<!-- ── Actions ── -->
<div class="actions-bar">
  <button on:click={handleReplay} title="Ctrl+R — Rejouer">↪ Replay</button>
  <button on:click={() => showCookieEditor = true} title="Éditer les cookies">🍪 Cookies</button>
  <button class="danger" on:click={handleDrop} title="Dropper la requête">✂ Drop</button>
  <span class="spacer" />
  <button on:click={() => selectedId.set(null)} title="Fermer (Échap)">✕</button>
</div>

<!-- ── Onglets ── -->
<div class="tabs">
  <div class="tab {activeTab === 'headers' ? 'active' : ''}" on:click={() => activeTab = 'headers'}>
    [1] Headers
  </div>
  <div class="tab {activeTab === 'body' ? 'active' : ''}" on:click={() => activeTab = 'body'}>
    [2] Body {req.response_body ? `(${formatSize(req.response_body)})` : ''}
  </div>
  <div class="tab {activeTab === 'hex' ? 'active' : ''}" on:click={() => activeTab = 'hex'}>
    [3] Hex
  </div>
</div>

<!-- ── Contenu ── -->
<div class="tab-content">
  {#if activeTab === 'headers'}
    <div class="headers-panel">
      <div class="section-title">── REQUEST HEADERS</div>
      {#each req.request_headers as [k, v]}
        <div class="header-row {isHighlightedHeader(k) ? 'highlighted' : ''}">
          <span class="hdr-key">{k}:</span>
          <span class="hdr-val">{v}</span>
        </div>
      {/each}

      {#if req.response_headers.length > 0}
        <div class="section-title" style="margin-top:16px;">── RESPONSE HEADERS</div>
        {#each req.response_headers as [k, v]}
          <div class="header-row">
            <span class="hdr-key">{k}:</span>
            <span class="hdr-val">{v}</span>
          </div>
        {/each}
      {/if}
    </div>

  {:else if activeTab === 'body'}
    <pre class="body-panel">{formatBody(req.response_body ?? req.request_body)}</pre>

  {:else if activeTab === 'hex'}
    <HexViewer bytes={req.response_body ?? req.request_body} />
  {/if}
</div>

<!-- ── Cookie Editor modal ── -->
{#if showCookieEditor}
  <CookieEditor
    headers={req.request_headers}
    requestId={req.id}
    on:close={() => showCookieEditor = false}
  />
{/if}

{:else}
  <div class="no-req">Sélectionnez une requête dans la liste.</div>
{/if}

<style>
  .detail-info {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    background: var(--bg-panel);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    overflow: hidden;
  }
  .method-badge { font-weight: bold; font-size: 11px; padding: 2px 6px; border-radius: 3px; }
  .method-GET    { color: var(--green);  border: 1px solid var(--green); }
  .method-POST   { color: var(--cyan);   border: 1px solid var(--cyan); }
  .method-PUT    { color: var(--yellow); border: 1px solid var(--yellow); }
  .method-DELETE { color: var(--red);    border: 1px solid var(--red); }
  .method-PATCH  { color: #c87ade;       border: 1px solid #c87ade; }
  .tls  { color: var(--cyan); font-size: 11px; }
  .http { color: var(--gray); font-size: 11px; }
  .url  { flex: 1; color: var(--text); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 12px; }
  .status { font-weight: bold; flex-shrink: 0; }
  .duration { color: var(--gray); font-size: 11px; flex-shrink: 0; }
  .resp-size { color: var(--gray); font-size: 11px; flex-shrink: 0; }

  .actions-bar {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 5px 10px;
    background: var(--bg);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .spacer { flex: 1; }

  .tab-content {
    flex: 1;
    overflow-y: auto;
    background: var(--bg);
    min-height: 0;
  }

  .headers-panel { padding: 10px 14px; }
  .section-title { color: var(--accent); font-weight: bold; font-size: 11px; margin-bottom: 6px; letter-spacing: 0.5px; }

  .header-row {
    display: flex;
    gap: 8px;
    padding: 3px 0;
    border-bottom: 1px solid rgba(40,72,130,0.2);
    font-size: 12px;
    line-height: 1.4;
  }
  .header-row.highlighted { background: rgba(220,180,50,0.08); border-radius: 2px; }
  .hdr-key { color: var(--cyan); flex-shrink: 0; min-width: 180px; }
  .hdr-val { color: var(--text); word-break: break-all; }

  .body-panel {
    padding: 12px 14px;
    color: var(--text);
    font-size: 12px;
    line-height: 1.6;
    white-space: pre-wrap;
    word-break: break-all;
    background: var(--bg);
  }

  .no-req { padding: 40px; text-align: center; color: var(--gray); }
</style>
