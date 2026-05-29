<script lang="ts">
  import { onMount } from "svelte";
  import Sidebar from "./lib/components/Sidebar.svelte";
  import Toolbar from "./lib/components/Toolbar.svelte";
  import FileList from "./lib/components/FileList.svelte";
  import StatusBar from "./lib/components/StatusBar.svelte";
  import PreviewPanel from "./lib/components/PreviewPanel.svelte";
  import AddCatalogModal from "./lib/components/AddCatalogModal.svelte";
  import ConfirmModal from "./lib/components/ConfirmModal.svelte";
  import SettingsModal from "./lib/components/SettingsModal.svelte";
  import UpdateCatalogModal from "./lib/components/UpdateCatalogModal.svelte";
  import InfoModal from "./lib/components/InfoModal.svelte";
  import {
    catalogs,
    activeCatalogId,
    currentFiles,
    breadcrumbs,
    isLoading,
    searchQuery,
    mediaFilter,
    activeCatalog,
  } from "./lib/stores/catalog";
  import { theme } from "./lib/stores/theme";
  import * as api from "./lib/services/tauri";
  import type { FileEntry, BreadcrumbItem, Catalog } from "./lib/types";

  let showAddModal = false;
  let showSettings = false;
  let deleteTarget: Catalog | null = null;
  let updateTarget: Catalog | null = null;

  onMount(async () => {
    document.documentElement.setAttribute("data-theme", $theme);
    await loadCatalogs();
  });

  async function loadCatalogs() {
    try {
      $catalogs = await api.listCatalogs();
    } catch (e) {
      console.error("Failed to load catalogs:", e);
    }
  }

  async function handleAddCatalog(path: string, name: string) {
    showAddModal = false;
    $isLoading = true;
    try {
      const id = await api.startScan(path, name);
      await loadCatalogs();
      navigateToFolder(id);
    } catch (e) {
      console.error("Scan failed:", e);
    } finally {
      $isLoading = false;
    }
  }

  function handleSelectFolder(
    catalogId: number,
    folder: { id: number; name: string } | null,
    path?: BreadcrumbItem[]
  ) {
    $activeCatalogId = catalogId;
    $searchQuery = "";
    $breadcrumbs = path ?? (folder ? [{ id: folder.id, name: folder.name }] : []);
  }

  function handleRequestDelete(catalog: Catalog) {
    deleteTarget = catalog;
  }

  async function confirmDelete() {
    if (!deleteTarget) return;
    const id = deleteTarget.id;
    deleteTarget = null;
    await api.deleteCatalog(id);
    await loadCatalogs();
    if ($activeCatalogId === id) {
      $activeCatalogId = null;
      $currentFiles = [];
      $breadcrumbs = [];
    }
  }

  $: {
    const filter = $mediaFilter;
    const query = $searchQuery;
    const catalogId = $activeCatalogId;
    const crumbs = $breadcrumbs;

    if (query && catalogId !== null) {
      performSearch(catalogId, query);
    } else if (catalogId !== null) {
      const parentId = crumbs[crumbs.length - 1]?.id ?? null;
      loadChildren(catalogId, parentId, filter);
    }
  }

  function navigateToFolder(catalogId: number) {
    $activeCatalogId = catalogId;
    $breadcrumbs = [];
    $searchQuery = "";
  }

  let requestId = 0;

  async function loadChildren(catalogId: number, parentId: number | null, filter?: string | null) {
    const thisRequest = ++requestId;
    try {
      const files = filter
        ? await api.getChildrenFiltered(catalogId, parentId, filter)
        : await api.getChildren(catalogId, parentId);
      if (thisRequest === requestId) $currentFiles = files;
    } catch (e) {
      console.error("Failed to load files:", e);
    }
  }

  async function performSearch(catalogId: number, query: string) {
    const thisRequest = ++requestId;
    try {
      const files = await api.searchFiles(catalogId, query);
      if (thisRequest === requestId) $currentFiles = files;
    } catch (e) {
      console.error("Search failed:", e);
    }
  }

  function handleOpenEntry(entry: FileEntry) {
    if (!entry.is_dir || $activeCatalogId === null) return;
    $breadcrumbs = [...$breadcrumbs, { id: entry.id, name: entry.name }];
    $searchQuery = "";
  }

  function handleGoUp() {
    if ($activeCatalogId === null || $breadcrumbs.length === 0) return;
    $breadcrumbs = $breadcrumbs.slice(0, -1);
  }

  let removeTargets: FileEntry[] = [];
  let infoTargets: FileEntry[] = [];
  let selectedEntries: FileEntry[] = [];
  let showPreview = true;

  function handleRemoveEntries(entries: FileEntry[]) {
    removeTargets = entries;
  }

  async function confirmRemoveEntries() {
    if (removeTargets.length === 0 || $activeCatalogId === null) return;
    const ids = removeTargets.map((e) => e.id);
    const catalogId = $activeCatalogId;
    removeTargets = [];
    try {
      await api.removeFileEntries(catalogId, ids);
      await loadCatalogs();
      const lastCrumb = $breadcrumbs[$breadcrumbs.length - 1];
      await loadChildren(catalogId, lastCrumb?.id ?? null, $mediaFilter);
    } catch (e) {
      console.error("Failed to remove entries:", e);
    }
  }

  function handleNavigate(item: BreadcrumbItem) {
    if ($activeCatalogId === null) return;
    if (item.id === null) {
      navigateToFolder($activeCatalogId);
    } else {
      const idx = $breadcrumbs.findIndex((b) => b.id === item.id);
      if (idx >= 0) {
        $breadcrumbs = $breadcrumbs.slice(0, idx + 1);
      }
    }
  }
</script>

<div class="app-layout">
  <Sidebar
    onAddCatalog={() => (showAddModal = true)}
    onSelectFolder={handleSelectFolder}
    onRequestDelete={handleRequestDelete}
    onRequestUpdate={(catalog) => (updateTarget = catalog)}
    onOpenSettings={() => (showSettings = true)}
  />
  <div class="main-area">
    <Toolbar onNavigate={handleNavigate} onGoUp={handleGoUp} />
    <div class="content-area">
      <FileList
        onOpen={handleOpenEntry}
        onGoUp={handleGoUp}
        onRemoveEntries={handleRemoveEntries}
        onGetInfo={(entries) => (infoTargets = entries)}
        onSelectionChange={(entries) => (selectedEntries = entries)}
      />
      {#if showPreview && $activeCatalogId !== null}
        <PreviewPanel entries={selectedEntries} catalogId={$activeCatalogId} />
      {/if}
    </div>
    <StatusBar {selectedEntries} />
  </div>
</div>

{#if showAddModal}
  <AddCatalogModal onSubmit={handleAddCatalog} onClose={() => (showAddModal = false)} />
{/if}

{#if deleteTarget}
  <ConfirmModal
    message={`Delete catalog "${deleteTarget.name}" and all its indexed files?`}
    onConfirm={confirmDelete}
    onCancel={() => (deleteTarget = null)}
  />
{/if}

{#if removeTargets.length > 0}
  <ConfirmModal
    message={removeTargets.length === 1
      ? `Remove "${removeTargets[0].name}" from the catalog?`
      : `Remove ${removeTargets.length} items from the catalog?`}
    onConfirm={confirmRemoveEntries}
    onCancel={() => (removeTargets = [])}
  />
{/if}

{#if updateTarget}
  <UpdateCatalogModal
    catalog={updateTarget}
    onComplete={async () => {
      updateTarget = null;
      await loadCatalogs();
      if ($activeCatalogId !== null) {
        const lastCrumb = $breadcrumbs[$breadcrumbs.length - 1];
        await loadChildren($activeCatalogId, lastCrumb?.id ?? null, $mediaFilter);
      }
    }}
    onClose={() => (updateTarget = null)}
  />
{/if}

{#if infoTargets.length > 0 && $activeCatalogId !== null}
  <InfoModal
    entries={infoTargets}
    catalogId={$activeCatalogId}
    onClose={() => (infoTargets = [])}
  />
{/if}

{#if showSettings}
  <SettingsModal onClose={() => (showSettings = false)} />
{/if}

<style>
  .app-layout {
    display: flex;
    height: 100%;
    overflow: hidden;
  }

  .main-area {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    background: var(--bg-base);
  }

  .content-area {
    flex: 1;
    display: flex;
    min-height: 0;
    overflow: hidden;
  }
</style>
