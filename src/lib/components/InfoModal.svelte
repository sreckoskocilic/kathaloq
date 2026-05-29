<script lang="ts">
  import { onMount } from "svelte";
  import {
    formatSize,
    formatDate,
    formatDuration,
    formatBitrate,
    formatSampleRate,
    getFileColor,
  } from "../services/format";
  import * as api from "../services/tauri";
  import type { FileEntry, FolderStats, MediaTags } from "../types";

  export let entries: FileEntry[];
  export let catalogId: number;
  export let onClose: () => void;

  let folderStats: FolderStats | null = null;
  let bulkStats: FolderStats | null = null;
  let mediaTags: MediaTags | null = null;
  let bulkMediaTags: MediaTags[] = [];
  let loading = true;

  $: isSingle = entries.length === 1;
  $: entry = entries[0];
  $: isMulti = entries.length > 1;

  onMount(async () => {
    const promises: Promise<void>[] = [];

    if (isSingle && entry.is_dir) {
      promises.push(
        api
          .getFolderStats(catalogId, entry.id)
          .then((s) => {
            folderStats = s;
          })
          .catch((e) => console.error(e))
      );
    } else if (isSingle && !entry.is_dir) {
      promises.push(
        api
          .getMediaTags(entry.id)
          .then((t) => {
            mediaTags = t;
          })
          .catch((e) => console.error(e))
      );
    } else if (isMulti) {
      promises.push(
        api
          .getBulkStats(
            catalogId,
            entries.map((e) => e.id)
          )
          .then((s) => {
            bulkStats = s;
          })
          .catch((e) => console.error(e))
      );
      const fileIds = entries.filter((e) => !e.is_dir).map((e) => e.id);
      if (fileIds.length > 0) {
        promises.push(
          api
            .getMediaTagsBulk(fileIds)
            .then((t) => {
              bulkMediaTags = t;
            })
            .catch((e) => console.error(e))
        );
      }
    }

    await Promise.all(promises);
    loading = false;
  });

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") onClose();
  }

  function getCommonValue<T>(values: (T | null | undefined)[]): T | null {
    const defined = values.filter((v): v is T => v != null);
    if (defined.length === 0) return null;
    const first = defined[0];
    return defined.every((v) => v === first) ? first : null;
  }

  $: multiExtension = isMulti
    ? getCommonValue(entries.filter((e) => !e.is_dir).map((e) => e.extension))
    : null;

  $: multiFileCount = entries.filter((e) => !e.is_dir).length;
  $: multiFolderCount = entries.filter((e) => e.is_dir).length;
  $: multiDirectSize = entries.filter((e) => !e.is_dir).reduce((sum, e) => sum + e.size, 0);

  $: commonArtist =
    bulkMediaTags.length > 0 ? getCommonValue(bulkMediaTags.map((t) => t.artist)) : null;
  $: commonAlbum =
    bulkMediaTags.length > 0 ? getCommonValue(bulkMediaTags.map((t) => t.album)) : null;
  $: commonGenre =
    bulkMediaTags.length > 0 ? getCommonValue(bulkMediaTags.map((t) => t.genre)) : null;
  $: commonYear =
    bulkMediaTags.length > 0 ? getCommonValue(bulkMediaTags.map((t) => t.year)) : null;
  $: totalDuration = bulkMediaTags.reduce((sum, t) => sum + (t.duration_secs ?? 0), 0);
  $: hasMultiMediaTags = bulkMediaTags.length > 0;
</script>

<svelte:window on:keydown={handleKeydown} />

<div class="overlay" on:click={onClose} role="presentation">
  <div
    class="modal"
    on:click|stopPropagation
    on:keydown={handleKeydown}
    role="dialog"
    aria-modal="true"
    tabindex="-1"
  >
    {#if isSingle}
      <div class="info-header">
        <span class="info-icon" style:color={getFileColor(entry.extension, entry.is_dir)}>
          {#if entry.is_dir}
            <svg width="28" height="28" viewBox="0 0 16 16" fill="currentColor">
              <path
                d="M1.5 3.5C1.5 2.67 2.17 2 3 2H6.17L7.67 3.5H13C13.83 3.5 14.5 4.17 14.5 5V12C14.5 12.83 13.83 13.5 13 13.5H3C2.17 13.5 1.5 12.83 1.5 12V3.5Z"
              />
            </svg>
          {:else}
            <svg
              width="26"
              height="26"
              viewBox="0 0 16 16"
              fill="none"
              stroke="currentColor"
              stroke-width="1.2"
            >
              <path d="M4 1.5h5l4 4V14a.5.5 0 01-.5.5h-8A.5.5 0 014 14V2a.5.5 0 01.5-.5z" />
              <path d="M9 1.5V5.5h4" />
            </svg>
          {/if}
        </span>
        <h3 class="info-name">{entry.name}</h3>
      </div>

      <div class="info-divider"></div>

      <div class="info-rows">
        <div class="info-row">
          <span class="info-label">Kind</span>
          <span class="info-value"
            >{entry.is_dir ? "Folder" : (entry.extension?.toUpperCase() ?? "File")}</span
          >
        </div>
        <div class="info-row">
          <span class="info-label">Path</span>
          <span class="info-value mono">{entry.path}</span>
        </div>

        {#if !entry.is_dir}
          <div class="info-row">
            <span class="info-label">Size</span>
            <span class="info-value">{formatSize(entry.size)}</span>
          </div>
        {/if}

        <div class="info-row">
          <span class="info-label">Modified</span>
          <span class="info-value">{formatDate(entry.modified)}</span>
        </div>

        {#if entry.is_dir && loading}
          <div class="info-row">
            <span class="info-label">Contents</span>
            <span class="info-value muted">Calculating...</span>
          </div>
        {:else if entry.is_dir && folderStats}
          <div class="info-divider"></div>
          <div class="info-row">
            <span class="info-label">Files</span>
            <span class="info-value">{folderStats.file_count.toLocaleString()}</span>
          </div>
          <div class="info-row">
            <span class="info-label">Folders</span>
            <span class="info-value">{folderStats.folder_count.toLocaleString()}</span>
          </div>
          <div class="info-row">
            <span class="info-label">Total Size</span>
            <span class="info-value">{formatSize(folderStats.total_size)}</span>
          </div>
        {/if}

        {#if !entry.is_dir && mediaTags && !loading}
          <div class="info-divider"></div>
          <div class="section-title">Media Info</div>

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
              <span class="info-label">Sample Rate</span>
              <span class="info-value">{formatSampleRate(mediaTags.sample_rate)}</span>
            </div>
          {/if}
          {#if mediaTags.channels}
            <div class="info-row">
              <span class="info-label">Channels</span>
              <span class="info-value"
                >{mediaTags.channels === 1
                  ? "Mono"
                  : mediaTags.channels === 2
                    ? "Stereo"
                    : mediaTags.channels}</span
              >
            </div>
          {/if}
        {/if}
      </div>
    {:else if isMulti}
      <div class="info-header">
        <span class="info-icon multi">
          <svg
            width="26"
            height="26"
            viewBox="0 0 16 16"
            fill="none"
            stroke="currentColor"
            stroke-width="1.2"
          >
            <rect x="2" y="3" width="9" height="11" rx="1" />
            <rect x="5" y="2" width="9" height="11" rx="1" />
          </svg>
        </span>
        <h3 class="info-name">{entries.length} items selected</h3>
      </div>

      <div class="info-divider"></div>

      <div class="info-rows">
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

        {#if multiExtension !== null}
          <div class="info-row">
            <span class="info-label">Type</span>
            <span class="info-value">{multiExtension?.toUpperCase()}</span>
          </div>
        {:else if multiFileCount > 0}
          <div class="info-row">
            <span class="info-label">Type</span>
            <span class="info-value muted">Multiple</span>
          </div>
        {/if}

        <div class="info-divider"></div>

        {#if loading}
          <div class="info-row">
            <span class="info-label">Total Size</span>
            <span class="info-value muted">Calculating...</span>
          </div>
        {:else if bulkStats}
          <div class="info-row">
            <span class="info-label">Total Files</span>
            <span class="info-value">{bulkStats.file_count.toLocaleString()}</span>
          </div>
          {#if bulkStats.folder_count > 0}
            <div class="info-row">
              <span class="info-label">Total Folders</span>
              <span class="info-value">{bulkStats.folder_count.toLocaleString()}</span>
            </div>
          {/if}
          <div class="info-row">
            <span class="info-label">Total Size</span>
            <span class="info-value">{formatSize(bulkStats.total_size)}</span>
          </div>
        {:else}
          <div class="info-row">
            <span class="info-label">Direct Size</span>
            <span class="info-value">{formatSize(multiDirectSize)}</span>
          </div>
        {/if}

        {#if !loading && hasMultiMediaTags}
          <div class="info-divider"></div>
          <div class="section-title">Shared Media Info ({bulkMediaTags.length} tracks)</div>

          {#if totalDuration > 0}
            <div class="info-row">
              <span class="info-label">Total Duration</span>
              <span class="info-value">{formatDuration(totalDuration)}</span>
            </div>
          {/if}
          {#if commonArtist !== null}
            <div class="info-row">
              <span class="info-label">Artist</span>
              <span class="info-value">{commonArtist}</span>
            </div>
          {:else if bulkMediaTags.some((t) => t.artist)}
            <div class="info-row">
              <span class="info-label">Artist</span>
              <span class="info-value muted">Multiple</span>
            </div>
          {/if}
          {#if commonAlbum !== null}
            <div class="info-row">
              <span class="info-label">Album</span>
              <span class="info-value">{commonAlbum}</span>
            </div>
          {:else if bulkMediaTags.some((t) => t.album)}
            <div class="info-row">
              <span class="info-label">Album</span>
              <span class="info-value muted">Multiple</span>
            </div>
          {/if}
          {#if commonGenre !== null}
            <div class="info-row">
              <span class="info-label">Genre</span>
              <span class="info-value">{commonGenre}</span>
            </div>
          {:else if bulkMediaTags.some((t) => t.genre)}
            <div class="info-row">
              <span class="info-label">Genre</span>
              <span class="info-value muted">Multiple</span>
            </div>
          {/if}
          {#if commonYear !== null}
            <div class="info-row">
              <span class="info-label">Year</span>
              <span class="info-value">{commonYear}</span>
            </div>
          {:else if bulkMediaTags.some((t) => t.year)}
            <div class="info-row">
              <span class="info-label">Year</span>
              <span class="info-value muted">Multiple</span>
            </div>
          {/if}
        {/if}
      </div>
    {/if}

    <div class="actions">
      <button class="btn btn-primary" on:click={onClose}>Close</button>
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 200;
  }

  .modal {
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 24px;
    width: 400px;
    max-height: 80vh;
    overflow-y: auto;
    box-shadow: var(--shadow-lg);
  }

  .info-header {
    display: flex;
    align-items: center;
    gap: 14px;
    margin-bottom: 4px;
  }

  .info-icon {
    flex-shrink: 0;
    display: flex;
    align-items: center;
  }

  .info-icon.multi {
    color: var(--text-muted);
  }

  .info-name {
    font-family: var(--font-family-body);
    font-size: 18px;
    font-weight: 500;
    color: var(--text-primary);
    word-break: break-word;
    letter-spacing: -0.01em;
  }

  .info-divider {
    height: 1px;
    background: var(--border-subtle);
    margin: 14px 0;
  }

  .section-title {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--accent);
    margin-bottom: 6px;
  }

  .info-rows {
    display: flex;
    flex-direction: column;
    gap: 7px;
  }

  .info-row {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 16px;
    font-size: 13px;
  }

  .info-label {
    color: var(--text-muted);
    flex-shrink: 0;
    min-width: 90px;
    font-size: 12px;
  }

  .info-value {
    color: var(--text-primary);
    text-align: right;
    word-break: break-all;
  }

  .info-value.mono {
    font-family: var(--font-family-mono);
    font-size: 11.5px;
    color: var(--text-secondary);
  }

  .info-value.muted {
    color: var(--text-muted);
    font-style: italic;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    margin-top: 22px;
  }

  .btn {
    padding: 8px 18px;
    border-radius: 8px;
    font-size: 13px;
    font-weight: 500;
    transition: all 0.12s;
  }

  .btn-primary {
    background: var(--accent);
    color: #fff;
    font-weight: 600;
  }

  .btn-primary:hover {
    background: var(--accent-hover);
    transform: translateY(-1px);
    box-shadow: var(--shadow-sm);
  }
</style>
