<!-- src/lib/components/CookieEditor.svelte -->
<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { replayRequest } from "../api/tauri";

  export let headers: [string, string][] = [];
  export let requestId: string = "";

  const dispatch = createEventDispatcher();

  // Parser les cookies depuis les headers
  type CookieRow = { name: string; value: string; editing: boolean };

  let cookies: CookieRow[] = parseCookies(headers);

  function parseCookies(hdrs: [string, string][]): CookieRow[] {
    for (const [k, v] of hdrs) {
      if (k.toLowerCase() === "cookie") {
        return v.split(";").map(p => {
          const [name, ...rest] = p.trim().split("=");
          return { name: name.trim(), value: rest.join("=").trim(), editing: false };
        });
      }
    }
    return [];
  }

  function addRow() {
    cookies = [...cookies, { name: "", value: "", editing: true }];
  }

  function removeRow(i: number) {
    cookies = cookies.filter((_, idx) => idx !== i);
  }

  function buildCookieHeader(): string {
    return cookies
      .filter(c => c.name.trim())
      .map(c => `${c.name}=${c.value}`)
      .join("; ");
  }

  async function applyAndReplay() {
    const cookieHeader = buildCookieHeader();
    // Construire les headers modifiés
    const modifiedHeaders: Record<string, string> = {};
    for (const [k, v] of headers) {
      if (k.toLowerCase() !== "cookie") {
        modifiedHeaders[k] = v;
      }
    }
    if (cookieHeader) {
      modifiedHeaders["Cookie"] = cookieHeader;
    }
    await replayRequest(requestId, modifiedHeaders);
    dispatch("close");
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") dispatch("close");
  }
</script>

<svelte:window on:keydown={handleKeydown} />

<div class="overlay" on:click|self={() => dispatch("close")}>
  <div class="modal">
    <h3>🍪 Cookie Editor</h3>

    {#if cookies.length === 0}
      <p style="color:var(--gray); margin-bottom:12px;">Aucun cookie sur cette requête.</p>
    {:else}
      <div class="cookie-table">
        <div class="cookie-header">
          <span>Nom</span>
          <span>Valeur</span>
          <span></span>
        </div>
        {#each cookies as row, i}
          <div class="cookie-row">
            <input
              type="text"
              bind:value={row.name}
              placeholder="nom"
              style="width:100%;"
            />
            <input
              type="text"
              bind:value={row.value}
              placeholder="valeur"
              style="width:100%;"
            />
            <button class="danger icon-btn" on:click={() => removeRow(i)} title="Supprimer">✕</button>
          </div>
        {/each}
      </div>
    {/if}

    <div class="modal-footer">
      <button on:click={addRow}>+ Ajouter</button>
      <div style="flex:1" />
      <button on:click={() => dispatch("close")}>Annuler</button>
      <button class="primary" on:click={applyAndReplay}>
        ↪ Appliquer et Rejouer
      </button>
    </div>
  </div>
</div>

<style>
  .cookie-table { margin-bottom: 14px; }
  .cookie-header {
    display: grid;
    grid-template-columns: 1fr 1fr 28px;
    gap: 6px;
    color: var(--gray);
    font-size: 11px;
    font-weight: bold;
    text-transform: uppercase;
    padding-bottom: 4px;
    border-bottom: 1px solid var(--border);
    margin-bottom: 6px;
  }
  .cookie-row {
    display: grid;
    grid-template-columns: 1fr 1fr 28px;
    gap: 6px;
    align-items: center;
    margin-bottom: 5px;
  }
  .icon-btn { padding: 3px 7px; font-size: 12px; }
  .modal-footer {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 10px;
    border-top: 1px solid var(--border);
    padding-top: 12px;
  }
</style>
