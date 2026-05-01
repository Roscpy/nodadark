<!-- src/lib/components/HexViewer.svelte -->
<script lang="ts">
  export let bytes: number[] | null = null;

  $: chunks = bytes ? chunkArray(bytes, 16) : [];

  function chunkArray(arr: number[], size: number): number[][] {
    const result: number[][] = [];
    for (let i = 0; i < arr.length; i += size) {
      result.push(arr.slice(i, i + size));
    }
    return result;
  }

  function toHex(b: number): string {
    return b.toString(16).padStart(2, "0");
  }

  function toAscii(b: number): string {
    return b >= 32 && b < 127 ? String.fromCharCode(b) : ".";
  }
</script>

<div class="hex-viewer">
  {#if chunks.length === 0}
    <div class="empty">Aucune donnée à afficher.</div>
  {:else}
    <div class="hex-header">
      <span class="offset-col">Offset</span>
      <span class="hex-col">00 01 02 03 04 05 06 07 08 09 0A 0B 0C 0D 0E 0F</span>
      <span class="ascii-col">ASCII</span>
    </div>
    {#each chunks as chunk, i}
      <div class="hex-row">
        <span class="offset">{(i * 16).toString(16).padStart(8, "0")}</span>
        <span class="hex-bytes">
          {#each chunk as byte}
            <span class="byte {byte === 0 ? 'zero' : byte < 32 || byte > 126 ? 'nonprint' : 'print'}">{toHex(byte)} </span>
          {/each}
          {#each Array(16 - chunk.length) as _}
            <span class="byte pad">   </span>
          {/each}
        </span>
        <span class="ascii-repr">
          │{chunk.map(toAscii).join("")}
        </span>
      </div>
    {/each}
  {/if}
</div>

<style>
  .hex-viewer { font-size: 11px; padding: 10px 12px; overflow-x: auto; }
  .hex-header { display: flex; gap: 12px; color: var(--gray); border-bottom: 1px solid var(--border); padding-bottom: 4px; margin-bottom: 4px; }
  .offset-col { width: 80px; flex-shrink: 0; }
  .hex-col { flex: 1; }
  .ascii-col { width: 140px; flex-shrink: 0; }
  .hex-row { display: flex; gap: 12px; line-height: 1.6; }
  .offset { width: 80px; color: var(--gray); flex-shrink: 0; }
  .hex-bytes { flex: 1; }
  .ascii-repr { color: var(--green); font-family: monospace; width: 140px; }
  .byte.zero    { color: var(--gray); }
  .byte.nonprint { color: var(--red); }
  .byte.print   { color: var(--cyan); }
  .byte.pad     { color: transparent; }
  .empty { padding: 20px; color: var(--gray); text-align: center; }
</style>
