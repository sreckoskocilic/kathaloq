<script lang="ts">
  import { currentFiles, isLoading, breadcrumbs } from "../stores/catalog";
  import { columns } from "../stores/settings";
  import { SvelteSet } from "svelte/reactivity";
  import { formatSize, formatDate, getFileColor } from "../services/format";
  import type { FileEntry, SortField, SortDirection } from "../types";

  export let onOpen: (entry: FileEntry) => void;
  export let onGoUp: () => void;
  export let onRemoveEntries: (entries: FileEntry[]) => void;
  export let onGetInfo: (entries: FileEntry[]) => void;
  export let onSelectionChange: (entries: FileEntry[]) => void = () => {};

  let sortField: SortField = "name";
  let sortDir: SortDirection = "asc";
  let selectedIds: SvelteSet<number> = new SvelteSet();
  let lastClickedIndex: number | null = null;

  let contextMenu: { x: number; y: number } | null = null;

  function toggleSort(field: SortField) {
    if (sortField === field) {
      sortDir = sortDir === "asc" ? "desc" : "asc";
    } else {
      sortField = field;
      sortDir = "asc";
    }
    // Row order changed; a stale anchor would make the next shift-click select the wrong range.
    lastClickedIndex = null;
  }

  $: visibleCols = $columns.filter((c) => c.visible);

  $: sortedFiles = [...$currentFiles].sort((a, b) => {
    if (a.is_dir !== b.is_dir) return a.is_dir ? -1 : 1;

    let cmp = 0;
    switch (sortField) {
      case "name":
        cmp = a.name.localeCompare(b.name, undefined, { numeric: true });
        break;
      case "size":
        cmp = a.size - b.size;
        break;
      case "modified":
        cmp = (a.modified ?? "").localeCompare(b.modified ?? "");
        break;
      case "extension":
        cmp = (a.extension ?? "").localeCompare(b.extension ?? "");
        break;
    }
    return sortDir === "asc" ? cmp : -cmp;
  });

  $: if ($currentFiles) {
    selectedIds = new SvelteSet();
    lastClickedIndex = null;
    contextMenu = null;
    notifySelection();
  }

  function notifySelection() {
    const selected = sortedFiles.filter((f) => selectedIds.has(f.id));
    onSelectionChange(selected);
  }

  function getSortIndicator(field: string): string {
    if (sortField !== field) return "";
    return sortDir === "asc" ? " ▲" : " ▼";
  }

  function handleRowClick(e: MouseEvent, entry: FileEntry, index: number) {
    if (e.metaKey || e.ctrlKey) {
      if (selectedIds.has(entry.id)) {
        selectedIds.delete(entry.id);
      } else {
        selectedIds.add(entry.id);
      }
      lastClickedIndex = index;
    } else if (e.shiftKey && lastClickedIndex !== null) {
      const start = Math.min(lastClickedIndex, index);
      const end = Math.max(lastClickedIndex, index);
      for (let i = start; i <= end; i++) {
        selectedIds.add(sortedFiles[i].id);
      }
    } else {
      selectedIds = new SvelteSet([entry.id]);
      lastClickedIndex = index;
    }
    notifySelection();
  }

  function handleDblClick(entry: FileEntry) {
    if (entry.is_dir) onOpen(entry);
  }

  function handleKeydown(e: KeyboardEvent, entry: FileEntry) {
    if (e.key === "Enter" && entry.is_dir) onOpen(entry);
    if (e.key === "Delete" || e.key === "Backspace") {
      if (selectedIds.size > 0) {
        handleRemoveSelected();
      }
    }
  }

  function handleContextMenu(e: MouseEvent, entry: FileEntry, index: number) {
    e.preventDefault();
    e.stopPropagation();
    if (!selectedIds.has(entry.id)) {
      selectedIds = new SvelteSet([entry.id]);
      lastClickedIndex = index;
    }
    contextMenu = { x: e.clientX, y: e.clientY };
  }

  function handleRemoveSelected() {
    const selected = sortedFiles.filter((f) => selectedIds.has(f.id));
    if (selected.length > 0) {
      onRemoveEntries(selected);
    }
    contextMenu = null;
  }

  function handleGetInfo() {
    const selected = sortedFiles.filter((f) => selectedIds.has(f.id));
    if (selected.length > 0) {
      onGetInfo(selected);
    }
    contextMenu = null;
  }

  function closeContextMenu() {
    contextMenu = null;
  }

  function handleListClick(e: MouseEvent) {
    const target = e.target as HTMLElement;
    if (target.closest(".row")) return;
    selectedIds = new SvelteSet();
    lastClickedIndex = null;
  }

  function getCellValue(entry: FileEntry, colId: string): string {
    switch (colId) {
      case "extension":
        return entry.is_dir ? "Folder" : (entry.extension?.toUpperCase() ?? "File");
      case "size":
        return entry.is_dir ? "" : formatSize(entry.size);
      case "modified":
        return formatDate(entry.modified);
      default:
        return "";
    }
  }
</script>

<div class="file-list-container">
  {#if $isLoading}
    <div class="loading-state">
      <div class="spinner"></div>
      <span>Scanning...</span>
    </div>
  {:else if sortedFiles.length === 0}
    <div class="empty-state">
      <svg width="48" height="48" viewBox="0 0 48 48" fill="none" opacity="0.3">
        <path
          d="M8 12C8 10.9 8.9 10 10 10H20l3 3H38c1.1 0 2 .9 2 2v20c0 1.1-.9 2-2 2H10c-1.1 0-2-.9-2-2V12z"
          stroke="currentColor"
          stroke-width="1.5"
        />
        <path
          d="M20 25h8M24 21v8"
          stroke="currentColor"
          stroke-width="1.5"
          stroke-linecap="round"
        />
      </svg>
      <span>This folder is empty</span>
    </div>
  {:else}
    <div class="file-list">
      <div class="list-header">
        {#each visibleCols as col (col.id)}
          <button
            class="header-col col-{col.id}"
            style:width={col.width !== "flex" ? col.width : undefined}
            style:flex={col.width === "flex" ? "1" : undefined}
            on:click={() => toggleSort(col.id as SortField)}
          >
            <span class="header-label">{col.label}</span>
            {#if sortField === col.id}
              <span class="sort-indicator">{sortDir === "asc" ? "▲" : "▼"}</span>
            {/if}
          </button>
        {/each}
      </div>

      <div class="list-body" on:click={handleListClick} role="presentation">
        {#if $breadcrumbs.length > 0}
          <div
            class="row row-up"
            on:dblclick={onGoUp}
            on:keydown={(e) => {
              if (e.key === "Enter") onGoUp();
            }}
            role="row"
            tabindex="0"
          >
            <div class="cell col-name" style:flex="1">
              <span class="file-icon">
                <svg
                  width="16"
                  height="16"
                  viewBox="0 0 16 16"
                  fill="none"
                  stroke="var(--text-muted)"
                  stroke-width="1.4"
                >
                  <path d="M8 12V4M4 7l4-4 4 4" stroke-linecap="round" stroke-linejoin="round" />
                </svg>
              </span>
              <span class="file-name">..</span>
            </div>
            {#each visibleCols.filter((c) => c.id !== "name") as col (col.id)}
              <div class="cell col-{col.id}" style:width={col.width}></div>
            {/each}
          </div>
        {/if}

        {#each sortedFiles as entry, i (entry.id)}
          <div
            class="row"
            class:alt={i % 2 === 1}
            class:selected={selectedIds.has(entry.id)}
            on:click={(e) => handleRowClick(e, entry, i)}
            on:dblclick={() => handleDblClick(entry)}
            on:contextmenu={(e) => handleContextMenu(e, entry, i)}
            on:keydown={(e) => handleKeydown(e, entry)}
            role="row"
            tabindex="0"
          >
            {#each visibleCols as col (col.id)}
              {#if col.id === "name"}
                <div class="cell col-name" style:flex="1">
                  <span class="file-icon" style:color={getFileColor(entry.extension, entry.is_dir)}>
                    {#if entry.is_dir}
                      <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
                        <path
                          d="M1.5 3.5C1.5 2.67 2.17 2 3 2H6.17L7.67 3.5H13C13.83 3.5 14.5 4.17 14.5 5V12C14.5 12.83 13.83 13.5 13 13.5H3C2.17 13.5 1.5 12.83 1.5 12V3.5Z"
                        />
                      </svg>
                    {:else}
                      <svg
                        width="16"
                        height="16"
                        viewBox="0 0 16 16"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="1.1"
                      >
                        <path
                          d="M4.5 1.5h4.5l4 4V14a.5.5 0 01-.5.5H4.5A.5.5 0 014 14V2a.5.5 0 01.5-.5z"
                        />
                        <path d="M9 1.5V5.5h4" />
                      </svg>
                    {/if}
                  </span>
                  <span class="file-name" class:is-dir={entry.is_dir}>{entry.name}</span>
                </div>
              {:else}
                <div class="cell col-{col.id}" style:width={col.width}>
                  {getCellValue(entry, col.id)}
                </div>
              {/if}
            {/each}
          </div>
        {/each}
      </div>
    </div>
  {/if}
</div>

<svelte:window
  on:click={() => {
    if (contextMenu) closeContextMenu();
  }}
  on:contextmenu={() => {
    if (contextMenu) closeContextMenu();
  }}
/>

{#if contextMenu}
  <div
    class="context-menu"
    style:left="{contextMenu.x}px"
    style:top="{contextMenu.y}px"
    on:click|stopPropagation
    on:keydown|stopPropagation
    role="menu"
    tabindex="-1"
  >
    <button class="menu-item" on:click={handleGetInfo} role="menuitem">
      <svg
        width="14"
        height="14"
        viewBox="0 0 16 16"
        fill="none"
        stroke="currentColor"
        stroke-width="1.3"
      >
        <circle cx="8" cy="8" r="6" />
        <path d="M8 7v4" />
        <circle cx="8" cy="5" r="0.5" fill="currentColor" />
      </svg>
      Properties
    </button>
    <div class="menu-sep"></div>
    <button class="menu-item danger" on:click={handleRemoveSelected} role="menuitem">
      <svg
        width="14"
        height="14"
        viewBox="0 0 16 16"
        fill="none"
        stroke="currentColor"
        stroke-width="1.3"
      >
        <path d="M3 4h10M5.5 4V3h5v1M4.5 4v9.5h7V4" />
      </svg>
      Remove
    </button>
  </div>
{/if}

<style>
  .file-list-container {
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    background: var(--bg-base);
  }

  .loading-state,
  .empty-state {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    color: var(--text-muted);
    font-size: 13.5px;
  }

  .spinner {
    width: 20px;
    height: 20px;
    border: 2px solid var(--border);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .file-list {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .list-header {
    display: flex;
    height: 28px;
    align-items: center;
    padding: 0 8px;
    background: var(--list-header-bg);
    border-bottom: 1px solid var(--list-header-border);
    flex-shrink: 0;
  }

  .header-col {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 0 8px;
    height: 100%;
    font-size: 12.5px;
    font-weight: 500;
    color: var(--text-secondary);
    text-align: left;
    transition: color 0.1s;
    border-right: 1px solid var(--border-subtle);
  }

  .header-col:last-child {
    border-right: none;
  }

  .header-col:hover {
    color: var(--text-primary);
  }

  .header-label {
    white-space: nowrap;
  }

  .sort-indicator {
    font-size: 8px;
    opacity: 0.7;
  }

  .list-body {
    flex: 1;
    overflow-y: auto;
  }

  .row {
    display: flex;
    align-items: center;
    padding: 0 8px;
    height: 28px;
    outline: none;
    transition: background 0.06s;
  }

  .row:hover {
    background: var(--list-row-hover);
  }

  .row.alt {
    background: var(--list-row-alt);
  }

  .row.alt:hover {
    background: var(--list-row-hover);
  }

  .row.selected {
    background: var(--list-row-selected);
  }

  .row.selected:hover {
    background: var(--list-row-selected);
  }

  .row.selected .file-name {
    color: var(--text-bright);
  }

  .row:focus-visible {
    outline: 1px solid var(--border-focus);
    outline-offset: -1px;
  }

  .row-up {
    opacity: 0.7;
  }

  .row-up:hover {
    opacity: 1;
  }

  .cell {
    display: flex;
    align-items: center;
    font-size: 13.5px;
    color: var(--text-secondary);
    flex-shrink: 0;
    padding: 0 8px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .col-name {
    gap: 6px;
    color: var(--text-primary);
    min-width: 0;
  }

  .col-size {
    font-family: var(--font-family-mono);
    font-size: 12.5px;
    justify-content: flex-end;
  }

  .col-modified {
    font-family: var(--font-family-mono);
    font-size: 12.5px;
  }

  .col-extension {
    font-size: 12.5px;
  }

  .file-icon {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    width: 16px;
    justify-content: center;
  }

  .file-name {
    font-size: 13.5px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .file-name.is-dir {
    font-weight: 500;
  }

  .context-menu {
    position: fixed;
    z-index: 300;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: 7px;
    padding: 4px;
    min-width: 180px;
    box-shadow: var(--shadow-lg);
  }

  .menu-item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 6px 10px;
    font-size: 13.5px;
    color: var(--text-primary);
    text-align: left;
    border-radius: 4px;
    transition: background 0.06s;
  }

  .menu-item:hover {
    background: var(--bg-active);
  }

  .menu-sep {
    height: 1px;
    background: var(--border-subtle);
    margin: 4px 6px;
  }

  .menu-item.danger {
    color: var(--danger, #d45555);
  }

  .menu-item.danger:hover {
    background: var(--danger-bg, rgba(212, 85, 85, 0.1));
  }
</style>
