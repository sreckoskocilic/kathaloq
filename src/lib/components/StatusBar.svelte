<script lang="ts">
  import { afterUpdate } from "svelte";
  import { activeCatalog, currentFiles, catalogVersion } from "../stores/catalog";
  import { theme } from "../stores/theme";
  import { formatSize } from "../services/format";
  import * as api from "../services/tauri";
  import type { FileEntry } from "../types";

  export let selectedEntries: FileEntry[] = [];

  $: fileCount = $currentFiles.filter((f) => !f.is_dir).length;
  $: folderCount = $currentFiles.filter((f) => f.is_dir).length;

  // Selection size must match PreviewPanel: a selected folder contributes its
  // recursive contents (getBulkStats), not its own 0-byte row. afterUpdate +
  // memo key mirrors PreviewPanel; catalogVersion busts it after a mutation.
  let selectionSize = 0;
  let lastKey = "";
  let sizeGen = 0;

  afterUpdate(() => {
    const catalogId = $activeCatalog?.id ?? null;
    const key =
      selectedEntries.length > 1 && catalogId !== null
        ? `${$catalogVersion}:${catalogId}:${selectedEntries.map((e) => e.id).join(",")}`
        : "";
    if (key === lastKey) return;
    lastKey = key;
    if (!key || catalogId === null) {
      selectionSize = 0;
      return;
    }
    loadSelectionSize(catalogId, selectedEntries);
  });

  async function loadSelectionSize(catalogId: number, entries: FileEntry[]) {
    const gen = ++sizeGen;
    try {
      const stats = await api.getBulkStats(
        catalogId,
        entries.map((e) => e.id)
      );
      if (gen === sizeGen) selectionSize = stats.total_size;
    } catch (e) {
      console.error(e);
    }
  }
</script>

<footer class="status-bar">
  <div class="status-left">
    {#if selectedEntries.length > 1}
      <span class="stat highlight">{selectedEntries.length} selected</span>
      <span class="sep">·</span>
      <span class="stat highlight">{formatSize(selectionSize)}</span>
    {:else if $activeCatalog}
      <span class="stat">{folderCount} folders</span>
      <span class="sep">·</span>
      <span class="stat">{fileCount} files</span>
      <span class="sep">·</span>
      <span class="stat">{formatSize($activeCatalog.total_size)} total</span>
    {:else}
      <span class="stat muted">Select a catalog</span>
    {/if}
  </div>
  <div class="status-right">
    <span class="theme-badge">{$theme}</span>
  </div>
</footer>

<style>
  .status-bar {
    height: 26px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 16px;
    background: var(--statusbar-bg);
    border-top: 1px solid var(--border-subtle);
    flex-shrink: 0;
  }

  .status-left {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .status-right {
    display: flex;
    align-items: center;
  }

  .stat {
    font-family: var(--font-family-mono);
    font-size: 12px;
    color: var(--statusbar-text);
  }

  .stat.highlight {
    color: var(--accent);
  }

  .stat.muted {
    font-family: var(--font-family-body);
    opacity: 0.5;
  }

  .sep {
    color: var(--text-muted);
    opacity: 0.3;
    font-size: 11px;
  }

  .theme-badge {
    font-size: 11px;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted);
    opacity: 0.5;
    padding: 2px 8px;
    border-radius: 4px;
    background: var(--bg-hover);
  }
</style>
