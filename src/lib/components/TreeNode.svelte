<script lang="ts">
  import { onMount } from "svelte";
  import { getChildren } from "../services/tauri";
  import { catalogVersion } from "../stores/catalog";
  import type { FileEntry } from "../types";

  import type { BreadcrumbItem } from "../types";

  export let entry: FileEntry;
  export let depth: number = 0;
  export let selectedId: number | null = null;
  export let onSelect: (entry: FileEntry, path: BreadcrumbItem[]) => void;
  export let ancestors: BreadcrumbItem[] = [];

  let expanded = false;
  let children: FileEntry[] = [];
  let loaded = false;

  // On a catalog mutation, drop the cached children. Refetch immediately if this
  // node is expanded so the visible tree stays current; otherwise lazy-load on next open.
  // Imperative subscription (not $:) so the async refetch isn't flagged as a reactive loop.
  let lastVersion = $catalogVersion;
  onMount(() =>
    catalogVersion.subscribe((v) => {
      if (v === lastVersion) return;
      lastVersion = v;
      loaded = false;
      if (expanded) fetchChildren();
    })
  );

  async function fetchChildren() {
    children = (await getChildren(entry.catalog_id, entry.id))
      .filter((e) => e.is_dir)
      .sort((a, b) => a.name.localeCompare(b.name, undefined, { numeric: true }));
    loaded = true;
  }

  async function toggle() {
    if (!entry.is_dir) return;
    if (!loaded) await fetchChildren();
    expanded = !expanded;
  }

  $: currentPath = [...ancestors, { id: entry.id, name: entry.name }];

  async function handleClick() {
    onSelect(entry, currentPath);
    if (entry.is_dir && !expanded) {
      await toggle();
    }
  }
</script>

<div class="tree-node">
  <div
    class="node-row"
    class:selected={selectedId === entry.id}
    style:padding-left="{8 + depth * 14}px"
    on:click={handleClick}
    on:keydown={(e) => e.key === "Enter" && handleClick()}
    role="treeitem"
    aria-selected={selectedId === entry.id}
    tabindex="0"
    aria-expanded={entry.is_dir ? expanded : undefined}
  >
    {#if entry.is_dir}
      <button class="toggle" on:click|stopPropagation={toggle} tabindex="-1" aria-label="Expand">
        <svg width="8" height="8" viewBox="0 0 8 8" class:rotated={expanded}>
          <path d="M2 1.5l3.5 2.5-3.5 2.5z" fill="currentColor" />
        </svg>
      </button>
    {:else}
      <span class="toggle-spacer"></span>
    {/if}

    <svg
      class="node-icon"
      width="13"
      height="13"
      viewBox="0 0 16 16"
      fill="var(--icon-folder)"
      opacity="0.7"
    >
      <path
        d="M1.5 3.5C1.5 2.67 2.17 2 3 2H6.17L7.67 3.5H13C13.83 3.5 14.5 4.17 14.5 5V12C14.5 12.83 13.83 13.5 13 13.5H3C2.17 13.5 1.5 12.83 1.5 12V3.5Z"
      />
    </svg>

    <span class="node-name">{entry.name}</span>
  </div>

  {#if expanded && children.length > 0}
    <div class="children" role="group">
      {#each children as child (child.id)}
        <svelte:self
          entry={child}
          depth={depth + 1}
          {selectedId}
          {onSelect}
          ancestors={currentPath}
        />
      {/each}
    </div>
  {/if}
</div>

<style>
  .tree-node {
    user-select: none;
  }

  .node-row {
    display: flex;
    align-items: center;
    gap: 4px;
    height: 26px;
    padding-right: 8px;
    cursor: pointer;
    transition: background 0.1s;
    border-radius: 5px;
    margin: 0 6px;
  }

  .node-row:hover {
    background: var(--sidebar-item-hover);
  }

  .node-row.selected {
    background: var(--sidebar-item-active);
  }

  .toggle {
    width: 16px;
    height: 16px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    color: var(--text-muted);
    padding: 0;
    transition: color 0.1s;
  }

  .toggle:hover {
    color: var(--text-secondary);
  }

  .toggle svg {
    transition: transform 0.15s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .toggle svg.rotated {
    transform: rotate(90deg);
  }

  .toggle-spacer {
    width: 16px;
    flex-shrink: 0;
  }

  .node-icon {
    flex-shrink: 0;
  }

  .node-name {
    font-size: 12.5px;
    color: var(--text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .node-row.selected .node-name {
    color: var(--text-primary);
  }
</style>
