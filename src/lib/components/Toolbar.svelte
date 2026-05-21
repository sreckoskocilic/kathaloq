<script lang="ts">
  import { breadcrumbs, searchQuery, activeCatalog, mediaFilter } from "../stores/catalog";
  import type { BreadcrumbItem, MediaFilter } from "../types";

  export let onNavigate: (item: BreadcrumbItem) => void;
  export let onGoUp: () => void = () => {};

  let searchInput = "";
  let searchTimeout: ReturnType<typeof setTimeout>;

  function onSearchInput() {
    clearTimeout(searchTimeout);
    searchTimeout = setTimeout(() => {
      searchQuery.set(searchInput);
    }, 250);
  }

  function goBack() {
    if ($breadcrumbs.length > 0) {
      const prev = $breadcrumbs[$breadcrumbs.length - 2];
      onNavigate(prev ?? { id: null, name: $activeCatalog?.name ?? "" });
    }
  }

  function toggleFilter(type: "audio" | "video") {
    clearTimeout(searchTimeout);
    mediaFilter.set($mediaFilter === type ? null : type);
    searchInput = "";
    searchQuery.set("");
  }

  $: canGoBack = $breadcrumbs.length > 0;
  $: canGoUp = $breadcrumbs.length > 0;
</script>

<div class="navbar">
  <div class="nav-buttons">
    <button class="nav-btn" disabled={!canGoBack} on:click={goBack} title="Back">
      <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
        <path
          d="M10 3L5 8l5 5"
          stroke="currentColor"
          stroke-width="1.5"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>
    </button>
    <button class="nav-btn" disabled={!canGoUp} on:click={onGoUp} title="Up">
      <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
        <path
          d="M3 10l5-5 5 5"
          stroke="currentColor"
          stroke-width="1.5"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>
    </button>
  </div>

  <div class="address-bar">
    {#if $activeCatalog}
      <div class="address-segments">
        <button
          class="segment"
          on:click={() => onNavigate({ id: null, name: $activeCatalog?.name ?? "" })}
        >
          <svg width="14" height="14" viewBox="0 0 16 16" fill="none">
            <path
              d="M2 6.5L8 2.5l6 4V13a1 1 0 01-1 1H3a1 1 0 01-1-1V6.5z"
              stroke="currentColor"
              stroke-width="1.2"
            />
          </svg>
          <span>{$activeCatalog.name}</span>
        </button>
        {#each $breadcrumbs as item (item.id)}
          <span class="separator">
            <svg width="10" height="10" viewBox="0 0 10 10" fill="none">
              <path
                d="M3.5 2l4 3-4 3"
                stroke="currentColor"
                stroke-width="1.2"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
            </svg>
          </span>
          <button class="segment" on:click={() => onNavigate(item)}>
            <span>{item.name}</span>
          </button>
        {/each}
      </div>
    {:else}
      <span class="address-placeholder">Select a catalog</span>
    {/if}
  </div>

  {#if $activeCatalog}
    <div class="filter-buttons">
      <button
        class="filter-btn"
        class:active={$mediaFilter === "audio"}
        on:click={() => toggleFilter("audio")}
        title="Filter audio files"
      >
        <svg width="14" height="14" viewBox="0 0 16 16" fill="none">
          <path d="M12 2v9" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" />
          <circle cx="9" cy="11" r="2.5" stroke="currentColor" stroke-width="1.3" />
          <path d="M12 2l3-1v4l-3 1V2z" fill="currentColor" opacity="0.5" />
        </svg>
        <span>Audio</span>
      </button>
      <button
        class="filter-btn"
        class:active={$mediaFilter === "video"}
        on:click={() => toggleFilter("video")}
        title="Filter video files"
      >
        <svg width="14" height="14" viewBox="0 0 16 16" fill="none">
          <rect x="1" y="3" width="10" height="10" rx="1.5" stroke="currentColor" stroke-width="1.3" />
          <path d="M11 6l4-2v8l-4-2V6z" fill="currentColor" opacity="0.5" />
        </svg>
        <span>Video</span>
      </button>
    </div>
  {/if}

  <div class="search-area">
    {#if $activeCatalog}
      <div class="search-box">
        <svg width="14" height="14" viewBox="0 0 16 16" fill="none">
          <circle cx="6.5" cy="6.5" r="4.5" stroke="currentColor" stroke-width="1.3" />
          <path d="M10 10l4 4" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" />
        </svg>
        <input type="text" placeholder="Search" bind:value={searchInput} on:input={onSearchInput} />
      </div>
    {/if}
  </div>
</div>

<style>
  .navbar {
    height: 42px;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 10px;
    background: var(--nav-bg);
    border-bottom: 1px solid var(--nav-border);
  }

  .nav-buttons {
    display: flex;
    gap: 2px;
    flex-shrink: 0;
  }

  .nav-btn {
    width: 30px;
    height: 30px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 5px;
    color: var(--text-secondary);
    transition: all 0.12s;
  }

  .nav-btn:hover:not(:disabled) {
    background: var(--nav-btn-hover);
    color: var(--text-primary);
  }

  .nav-btn:disabled {
    opacity: 0.3;
    cursor: default;
  }

  .address-bar {
    flex: 1;
    min-width: 0;
    height: 30px;
    display: flex;
    align-items: center;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 5px;
    padding: 0 10px;
    overflow: hidden;
  }

  .address-bar:focus-within {
    border-color: var(--border-focus);
  }

  .address-segments {
    display: flex;
    align-items: center;
    gap: 0;
    overflow: hidden;
    white-space: nowrap;
  }

  .segment {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 13.5px;
    color: var(--text-primary);
    transition: background 0.1s;
    white-space: nowrap;
  }

  .segment:hover {
    background: var(--bg-hover);
  }

  .separator {
    color: var(--text-muted);
    display: flex;
    align-items: center;
    flex-shrink: 0;
  }

  .address-placeholder {
    font-size: 13.5px;
    color: var(--text-muted);
  }

  .filter-buttons {
    display: flex;
    gap: 4px;
    flex-shrink: 0;
  }

  .filter-btn {
    display: flex;
    align-items: center;
    gap: 4px;
    height: 30px;
    padding: 0 10px;
    border-radius: 5px;
    font-size: 12.5px;
    color: var(--text-secondary);
    border: 1px solid var(--border);
    background: var(--bg-input);
    transition: all 0.12s;
    white-space: nowrap;
  }

  .filter-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .filter-btn.active {
    background: var(--accent);
    color: var(--bg-base);
    border-color: var(--accent);
  }

  .search-area {
    flex-shrink: 0;
  }

  .search-box {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 30px;
    padding: 0 10px;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 5px;
    color: var(--text-muted);
    width: 220px;
    transition: border-color 0.15s;
  }

  .search-box:focus-within {
    border-color: var(--border-focus);
  }

  .search-box input {
    flex: 1;
    font-size: 13.5px;
    color: var(--text-primary);
    width: 100%;
  }

  .search-box input::placeholder {
    color: var(--text-muted);
  }
</style>
