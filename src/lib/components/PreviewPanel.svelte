<script lang="ts">
  import { afterUpdate } from "svelte";
  import { formatSize, formatDate, formatDuration, formatBitrate, formatSampleRate, getFileColor } from "../services/format";
  import * as api from "../services/tauri";
  import type { FileEntry, FolderStats, MediaTags } from "../types";

  export let entries: FileEntry[];
  export let catalogId: number;

  let folderStats: FolderStats | null = null;
  let mediaTags: MediaTags | null = null;
  let loading = false;
  let lastLoadKey = "";
  let loadGeneration = 0;

  $: isSingle = entries.length === 1;
  $: entry = entries[0];
  $: isMulti = entries.length > 1;

  afterUpdate(() => {
    const loadKey = entries.length > 0 && catalogId
      ? `${catalogId}:${entries.map(e => e.id).join(",")}`
      : "";

    if (loadKey === lastLoadKey) return;
    lastLoadKey = loadKey;

    if (!loadKey) {
      folderStats = null;
      mediaTags = null;
      return;
    }

    loadDetails(catalogId, entries);
  });

  async function loadDetails(catId: number, items: FileEntry[]) {
    const gen = ++loadGeneration;
    folderStats = null;
    mediaTags = null;
    loading = true;

    try {
      if (items.length === 1 && items[0].is_dir) {
        const result = await api.getFolderStats(catId, items[0].id);
        if (gen !== loadGeneration) return;
        folderStats = result;
      } else if (items.length === 1 && !items[0].is_dir) {
        const result = await api.getMediaTags(items[0].id);
        if (gen !== loadGeneration) return;
        mediaTags = result;
      } else if (items.length > 1) {
        const ids = items.map((e) => e.id);
        const result = await api.getBulkStats(catId, ids);
        if (gen !== loadGeneration) return;
        folderStats = result;
      }
    } catch { /* */ }

    if (gen !== loadGeneration) return;
    loading = false;
  }

  $: multiFileCount = isMulti ? entries.filter((e) => !e.is_dir).length : 0;
  $: multiFolderCount = isMulti ? entries.filter((e) => e.is_dir).length : 0;
  $: multiSize = isMulti ? entries.filter((e) => !e.is_dir).reduce((s, e) => s + e.size, 0) : 0;
</script>

<aside class="preview-panel">
  {#if entries.length === 0}
    <div class="no-selection">
      <svg width="32" height="32" viewBox="0 0 32 32" fill="none" opacity="0.3">
        <path d="M6 8c0-1.1.9-2 2-2h5l2 2h9c1.1 0 2 .9 2 2v14c0 1.1-.9 2-2 2H8c-1.1 0-2-.9-2-2V8z" stroke="currentColor" stroke-width="1.3"/>
      </svg>
      <span>Select a file to preview</span>
    </div>

  {:else if isSingle}
    <div class="preview-content">
      <div class="preview-icon" style:color={getFileColor(entry.extension, entry.is_dir)}>
        {#if entry.is_dir}
          <svg width="48" height="48" viewBox="0 0 48 48" fill="currentColor">
            <path d="M4 10c0-2.2 1.8-4 4-4h10l4 4h18c2.2 0 4 1.8 4 4v24c0 2.2-1.8 4-4 4H8c-2.2 0-4-1.8-4-4V10z"/>
          </svg>
        {:else}
          <svg width="44" height="44" viewBox="0 0 44 44" fill="none" stroke="currentColor" stroke-width="1.5">
            <path d="M12 4h14l12 12v24a2 2 0 01-2 2H12a2 2 0 01-2-2V6a2 2 0 012-2z"/>
            <path d="M26 4v12h12"/>
          </svg>
        {/if}
      </div>

      <h3 class="preview-name">{entry.name}</h3>
      <span class="preview-type">{entry.is_dir ? "Folder" : (entry.extension?.toUpperCase() ?? "File")}</span>

      <div class="info-section">
        <div class="info-row">
          <span class="info-label">Path</span>
          <span class="info-value path">{entry.path}</span>
        </div>

        {#if !entry.is_dir}
          <div class="info-row">
            <span class="info-label">Size</span>
            <span class="info-value">{formatSize(entry.size)}</span>
          </div>
        {/if}

        {#if entry.modified}
          <div class="info-row">
            <span class="info-label">Modified</span>
            <span class="info-value">{formatDate(entry.modified)}</span>
          </div>
        {/if}
      </div>

      {#if loading}
        <div class="info-section">
          <span class="loading-text">Loading...</span>
        </div>
      {/if}

      {#if entry.is_dir && folderStats}
        <div class="info-section">
          <div class="section-header">Contents</div>
          <div class="info-row">
            <span class="info-label">Files</span>
            <span class="info-value">{folderStats.file_count.toLocaleString()}</span>
          </div>
          <div class="info-row">
            <span class="info-label">Folders</span>
            <span class="info-value">{folderStats.folder_count.toLocaleString()}</span>
          </div>
          <div class="info-row">
            <span class="info-label">Total size</span>
            <span class="info-value">{formatSize(folderStats.total_size)}</span>
          </div>
        </div>
      {/if}

      {#if !entry.is_dir && mediaTags}
        <div class="info-section">
          <div class="section-header">Media</div>
          {#if mediaTags.title}
            <div class="info-row">
              <span class="info-label">Title</span>
              <span class="info-value">{mediaTags.title}</span>
            </div>
          {/if}
          {#if mediaTags.artist}
            <div class="info-row">
              <span class="info-label">Artist</span>
              <span class="info-value">{mediaTags.artist}</span>
            </div>
          {/if}
          {#if mediaTags.album}
            <div class="info-row">
              <span class="info-label">Album</span>
              <span class="info-value">{mediaTags.album}</span>
            </div>
          {/if}
          {#if mediaTags.genre}
            <div class="info-row">
              <span class="info-label">Genre</span>
              <span class="info-value">{mediaTags.genre}</span>
            </div>
          {/if}
          {#if mediaTags.year}
            <div class="info-row">
              <span class="info-label">Year</span>
              <span class="info-value">{mediaTags.year}</span>
            </div>
          {/if}
          {#if mediaTags.track_number}
            <div class="info-row">
              <span class="info-label">Track</span>
              <span class="info-value">{mediaTags.track_number}</span>
            </div>
          {/if}
          {#if mediaTags.duration_secs}
            <div class="info-row">
              <span class="info-label">Duration</span>
              <span class="info-value">{formatDuration(mediaTags.duration_secs)}</span>
            </div>
          {/if}
          {#if mediaTags.bitrate}
            <div class="info-row">
              <span class="info-label">Bitrate</span>
              <span class="info-value">{formatBitrate(mediaTags.bitrate)}</span>
            </div>
          {/if}
          {#if mediaTags.sample_rate}
            <div class="info-row">
              <span class="info-label">Sample rate</span>
              <span class="info-value">{formatSampleRate(mediaTags.sample_rate)}</span>
            </div>
          {/if}
          {#if mediaTags.channels}
            <div class="info-row">
              <span class="info-label">Channels</span>
              <span class="info-value">{mediaTags.channels === 1 ? "Mono" : mediaTags.channels === 2 ? "Stereo" : String(mediaTags.channels)}</span>
            </div>
          {/if}
        </div>
      {/if}
    </div>

  {:else if isMulti}
    <div class="preview-content">
      <div class="preview-icon multi">
        <svg width="44" height="44" viewBox="0 0 44 44" fill="none" stroke="currentColor" stroke-width="1.5" opacity="0.5">
          <rect x="6" y="10" width="24" height="28" rx="2"/>
          <rect x="14" y="6" width="24" height="28" rx="2"/>
        </svg>
      </div>

      <h3 class="preview-name">{entries.length} items selected</h3>

      <div class="info-section">
        {#if multiFileCount > 0}
          <div class="info-row">
            <span class="info-label">Files</span>
            <span class="info-value">{multiFileCount}</span>
          </div>
        {/if}
        {#if multiFolderCount > 0}
          <div class="info-row">
            <span class="info-label">Folders</span>
            <span class="info-value">{multiFolderCount}</span>
          </div>
        {/if}
        <div class="info-row">
          <span class="info-label">Size</span>
          <span class="info-value">{formatSize(multiSize)}</span>
        </div>
      </div>

      {#if loading}
        <div class="info-section">
          <span class="loading-text">Loading...</span>
        </div>
      {:else if folderStats}
        <div class="info-section">
          <div class="section-header">Total contents</div>
          <div class="info-row">
            <span class="info-label">All files</span>
            <span class="info-value">{folderStats.file_count.toLocaleString()}</span>
          </div>
          {#if folderStats.folder_count > 0}
            <div class="info-row">
              <span class="info-label">All folders</span>
              <span class="info-value">{folderStats.folder_count.toLocaleString()}</span>
            </div>
          {/if}
          <div class="info-row">
            <span class="info-label">Total size</span>
            <span class="info-value">{formatSize(folderStats.total_size)}</span>
          </div>
        </div>
      {/if}
    </div>
  {/if}
</aside>

<style>
  .preview-panel {
    width: 270px;
    min-width: 270px;
    background: var(--bg-surface);
    border-left: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    overflow-y: auto;
    user-select: none;
  }

  .no-selection {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    color: var(--text-muted);
    font-size: 12.5px;
  }

  .preview-content {
    padding: 20px 16px;
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  .preview-icon {
    margin-bottom: 12px;
  }

  .preview-icon.multi {
    color: var(--text-muted);
  }

  .preview-name {
    font-size: 15px;
    font-weight: 500;
    color: var(--text-bright);
    text-align: center;
    word-break: break-word;
    margin-bottom: 4px;
  }

  .preview-type {
    font-size: 11.5px;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin-bottom: 16px;
  }

  .info-section {
    width: 100%;
    padding: 12px 0;
    border-top: 1px solid var(--border-subtle);
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .section-header {
    font-size: 11.5px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted);
    margin-bottom: 4px;
  }

  .info-row {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 8px;
  }

  .info-label {
    font-size: 12.5px;
    color: var(--text-muted);
    flex-shrink: 0;
  }

  .info-value {
    font-size: 12.5px;
    color: var(--text-primary);
    text-align: right;
    word-break: break-all;
  }

  .info-value.path {
    font-family: var(--font-family-mono);
    font-size: 11.5px;
    color: var(--text-secondary);
  }

  .loading-text {
    font-size: 12.5px;
    color: var(--text-muted);
    font-style: italic;
  }
</style>
