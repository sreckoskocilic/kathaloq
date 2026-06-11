<script lang="ts">
  import { onMount } from "svelte";
  import { catalogs, activeCatalogId, catalogVersion } from "../stores/catalog";
  import { sidebarState } from "../stores/sidebar";
  import { getChildren } from "../services/tauri";
  import { SvelteSet, SvelteMap } from "svelte/reactivity";
  import TreeNode from "./TreeNode.svelte";
  import type { BreadcrumbItem, Catalog, FileEntry } from "../types";

  export let onAddCatalog: () => void;

  export let onSelectFolder: (
    catalogId: number,
    folder: { id: number; name: string } | null,
    path?: BreadcrumbItem[]
  ) => void;
  export let onRequestDelete: (catalog: Catalog) => void;
  export let onRequestUpdate: (catalog: Catalog) => void;
  export let onOpenSettings: () => void;

  let expandedCatalogs: SvelteSet<number> = new SvelteSet($sidebarState.expandedCatalogIds);
  let rootFolders: SvelteMap<number, FileEntry[]> = new SvelteMap();
  let selectedFolderPath: BreadcrumbItem[] = $sidebarState.selectedFolderPath;
  $: selectedFolderId =
    selectedFolderPath.length > 0 ? selectedFolderPath[selectedFolderPath.length - 1].id : null;

  $: if (
    $catalogs.length > 0 &&
    $activeCatalogId === null &&
    $sidebarState.activeCatalogId !== null
  ) {
    const exists = $catalogs.find((c) => c.id === $sidebarState.activeCatalogId);
    if (exists) {
      activeCatalogId.set($sidebarState.activeCatalogId);
      const path = $sidebarState.selectedFolderPath;
      const last = path[path.length - 1];
      if (last && last.id !== null) {
        onSelectFolder($sidebarState.activeCatalogId, { id: last.id, name: last.name }, path);
      } else {
        onSelectFolder($sidebarState.activeCatalogId, null);
      }
    }
  }

  $: if ($catalogs.length > 0) {
    restoreExpandedFolders();
  }

  // Drop the cached folder tree on any catalog mutation.
  // Imperative (not $:) so the async refetch isn't flagged a reactive loop.
  let lastVersion = $catalogVersion;
  onMount(() =>
    catalogVersion.subscribe((v) => {
      if (v === lastVersion) return;
      lastVersion = v;
      rootFolders = new SvelteMap();
      restoreExpandedFolders();
    })
  );

  async function restoreExpandedFolders() {
    for (const catalogId of expandedCatalogs) {
      if (!rootFolders.has(catalogId)) {
        const catalog = $catalogs.find((c) => c.id === catalogId);
        if (catalog) {
          try {
            const children = sortByName(
              (await getChildren(catalogId, null)).filter((f) => f.is_dir)
            );
            rootFolders.set(catalogId, children);
            rootFolders = rootFolders;
          } catch (e) {
            console.error(e);
          }
        }
      }
    }
  }

  function persistState() {
    sidebarState.set({
      expandedCatalogIds: [...expandedCatalogs],
      activeCatalogId: $activeCatalogId,
      selectedFolderPath,
    });
  }

  function selectCatalog(catalog: Catalog) {
    activeCatalogId.set(catalog.id);
    selectedFolderPath = [];
    onSelectFolder(catalog.id, null);
    persistState();
  }

  function sortByName(entries: FileEntry[]): FileEntry[] {
    return [...entries].sort((a, b) => a.name.localeCompare(b.name, undefined, { numeric: true }));
  }

  async function toggleCatalog(e: MouseEvent, catalog: Catalog) {
    e.stopPropagation();
    if (expandedCatalogs.has(catalog.id)) {
      expandedCatalogs.delete(catalog.id);
      expandedCatalogs = expandedCatalogs;
    } else {
      if (!rootFolders.has(catalog.id)) {
        const children = sortByName((await getChildren(catalog.id, null)).filter((f) => f.is_dir));
        rootFolders.set(catalog.id, children);
        rootFolders = rootFolders;
      }
      expandedCatalogs.add(catalog.id);
      expandedCatalogs = expandedCatalogs;
    }
    persistState();
  }

  function handleTreeSelect(entry: FileEntry, path: BreadcrumbItem[]) {
    activeCatalogId.set(entry.catalog_id);
    selectedFolderPath = path;
    onSelectFolder(entry.catalog_id, { id: entry.id, name: entry.name }, path);
    persistState();
  }

  function handleDeleteClick(e: MouseEvent, catalog: Catalog) {
    e.stopPropagation();
    onRequestDelete(catalog);
  }

  function handleUpdateClick(e: MouseEvent, catalog: Catalog) {
    e.stopPropagation();
    onRequestUpdate(catalog);
  }
</script>

<aside class="sidebar">
  <div class="sidebar-section-header">
    <span class="section-title">Catalogs</span>
    <button class="header-btn" on:click={onAddCatalog} title="Add catalog">
      <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
        <path d="M7 2v10M2 7h10" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
      </svg>
    </button>
  </div>

  <div class="catalog-list">
    {#each $catalogs as catalog (catalog.id)}
      <div class="catalog-group">
        <div
          class="catalog-item"
          class:active={$activeCatalogId === catalog.id && selectedFolderId === null}
          on:click={() => selectCatalog(catalog)}
          on:keydown={(e) => e.key === "Enter" && selectCatalog(catalog)}
          role="treeitem"
          aria-selected={$activeCatalogId === catalog.id && selectedFolderId === null}
          tabindex="0"
          aria-expanded={expandedCatalogs.has(catalog.id)}
        >
          <button
            class="expand-btn"
            on:click={(e) => toggleCatalog(e, catalog)}
            tabindex="-1"
            aria-label="Expand"
          >
            <svg
              width="10"
              height="10"
              viewBox="0 0 10 10"
              class:rotated={expandedCatalogs.has(catalog.id)}
            >
              <path d="M3 2l4 3-4 3z" fill="currentColor" />
            </svg>
          </button>

          <svg class="catalog-icon" width="16" height="16" viewBox="0 0 16 16">
            <path
              d="M1.5 3C1.5 2.17 2.17 1.5 3 1.5H6l1.5 1.5H13c.83 0 1.5.67 1.5 1.5V12c0 .83-.67 1.5-1.5 1.5H3c-.83 0-1.5-.67-1.5-1.5V3z"
              fill="var(--icon-folder)"
            />
          </svg>

          <div class="catalog-info">
            <span class="catalog-name">{catalog.name}</span>
          </div>

          <div class="item-actions">
            <button class="action-btn" on:click={(e) => handleUpdateClick(e, catalog)} title="Sync">
              <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
                <path
                  d="M1.5 6a4.5 4.5 0 018.18-2.6M10.5 6a4.5 4.5 0 01-8.18 2.6"
                  stroke="currentColor"
                  stroke-width="1.3"
                  stroke-linecap="round"
                />
                <path
                  d="M10 1v2.5H7.5"
                  stroke="currentColor"
                  stroke-width="1.3"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                />
                <path
                  d="M2 11V8.5h2.5"
                  stroke="currentColor"
                  stroke-width="1.3"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                />
              </svg>
            </button>
            <button
              class="action-btn danger"
              on:click={(e) => handleDeleteClick(e, catalog)}
              title="Remove"
            >
              <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
                <path
                  d="M2.5 3h7M4.5 3V2h3v1M3.5 3v7h5V3"
                  stroke="currentColor"
                  stroke-width="1.1"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                />
              </svg>
            </button>
          </div>
        </div>

        {#if expandedCatalogs.has(catalog.id)}
          <div class="tree-container" role="group">
            {#each rootFolders.get(catalog.id) ?? [] as folder (folder.id)}
              <TreeNode
                entry={folder}
                depth={1}
                selectedId={selectedFolderId}
                onSelect={handleTreeSelect}
              />
            {/each}
            {#if (rootFolders.get(catalog.id) ?? []).length === 0}
              <div class="tree-empty">No subfolders</div>
            {/if}
          </div>
        {/if}
      </div>
    {/each}

    {#if $catalogs.length === 0}
      <div class="empty-state">
        <svg width="36" height="36" viewBox="0 0 36 36" fill="none" opacity="0.25">
          <path
            d="M6 10c0-1.5 1.2-2.7 2.7-2.7H15l2.7 2.7h10.6c1.5 0 2.7 1.2 2.7 2.7v14c0 1.5-1.2 2.7-2.7 2.7H8.7C7.2 29.4 6 28.2 6 26.7V10z"
            stroke="currentColor"
            stroke-width="1.3"
          />
        </svg>
        <p>No catalogs yet</p>
        <p class="empty-hint">Click + to index a folder</p>
      </div>
    {/if}
  </div>

  <div class="sidebar-footer">
    <button class="footer-btn" on:click={onOpenSettings} title="Settings">
      <svg
        width="14"
        height="14"
        viewBox="0 0 16 16"
        fill="none"
        stroke="currentColor"
        stroke-width="1.3"
      >
        <path
          d="M6.5 1.5h3l.4 2 1.5.8 1.8-1 1.5 2.6-1.4 1.2v1.8l1.4 1.2-1.5 2.6-1.8-1-1.5.8-.4 2h-3l-.4-2-1.5-.8-1.8 1-1.5-2.6 1.4-1.2V6.1L1.3 4.9l1.5-2.6 1.8 1 1.5-.8.4-2z"
        />
        <circle cx="8" cy="8" r="2" />
      </svg>
      <span>Settings</span>
    </button>
  </div>
</aside>

<style>
  .sidebar {
    width: 250px;
    min-width: 250px;
    background: var(--sidebar-bg);
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    height: 100%;
    user-select: none;
  }

  .sidebar-section-header {
    height: 36px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 12px 0 16px;
    background: var(--sidebar-header-bg);
    border-bottom: 1px solid var(--sidebar-section-border);
  }

  .section-title {
    font-size: 11.5px;
    font-weight: 600;
    letter-spacing: 0.04em;
    color: var(--text-muted);
    text-transform: uppercase;
  }

  .header-btn {
    width: 24px;
    height: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    color: var(--text-muted);
    transition: all 0.12s;
  }

  .header-btn:hover {
    background: var(--sidebar-item-hover);
    color: var(--text-primary);
  }

  .catalog-list {
    flex: 1;
    overflow-y: auto;
    padding: 6px 0;
  }

  .catalog-group {
    margin-bottom: 2px;
  }

  .catalog-item {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 8px 6px 6px;
    margin: 0 6px;
    border-radius: 5px;
    cursor: pointer;
    transition: background 0.1s;
  }

  .catalog-item:hover {
    background: var(--sidebar-item-hover);
  }

  .catalog-item.active {
    background: var(--sidebar-item-active);
  }

  .expand-btn {
    width: 16px;
    height: 16px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    color: var(--text-muted);
    border-radius: 3px;
  }

  .expand-btn:hover {
    color: var(--text-primary);
  }

  .expand-btn svg {
    transition: transform 0.15s;
  }

  .expand-btn svg.rotated {
    transform: rotate(90deg);
  }

  .catalog-icon {
    flex-shrink: 0;
  }

  .catalog-info {
    flex: 1;
    min-width: 0;
  }

  .catalog-name {
    font-size: 13.5px;
    font-weight: 500;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .item-actions {
    display: flex;
    align-items: center;
    gap: 2px;
    visibility: hidden;
    opacity: 0;
    transition:
      opacity 0.1s,
      visibility 0.1s;
  }

  .catalog-item:hover .item-actions {
    visibility: visible;
    opacity: 1;
  }

  .action-btn {
    width: 22px;
    height: 22px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    color: var(--text-muted);
    transition: all 0.1s;
  }

  .action-btn:hover {
    background: var(--nav-btn-hover);
    color: var(--text-primary);
  }

  .action-btn.danger:hover {
    color: var(--danger);
  }

  .tree-container {
    padding-bottom: 4px;
  }

  .tree-empty {
    font-size: 12.5px;
    color: var(--text-muted);
    padding: 3px 0 3px 52px;
    font-style: italic;
  }

  .empty-state {
    padding: 40px 16px;
    text-align: center;
    color: var(--text-muted);
    font-size: 13px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
  }

  .empty-hint {
    font-size: 12px;
    opacity: 0.6;
  }

  .sidebar-footer {
    border-top: 1px solid var(--sidebar-section-border);
    padding: 6px;
  }

  .footer-btn {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 10px;
    border-radius: 5px;
    font-size: 12.5px;
    color: var(--text-muted);
    transition: all 0.1s;
  }

  .footer-btn:hover {
    background: var(--sidebar-item-hover);
    color: var(--text-secondary);
  }
</style>
