<script lang="ts">
  import type { Catalog, UpdatePreview } from "../types";
  import * as api from "../services/tauri";

  export let catalog: Catalog;
  export let onComplete: () => void;
  export let onClose: () => void;

  type Step = "ready" | "scanning" | "preview" | "applying" | "done" | "error";

  let step: Step = "ready";
  let preview: UpdatePreview | null = null;
  let result: UpdatePreview | null = null;
  let errorMsg = "";

  async function scanForChanges() {
    step = "scanning";
    try {
      preview = await api.previewCatalogUpdate(catalog.id);
      step = "preview";
    } catch (e) {
      errorMsg = String(e);
      step = "error";
    }
  }

  async function applyChanges() {
    step = "applying";
    try {
      result = await api.applyCatalogUpdate(catalog.id);
      step = "done";
    } catch (e) {
      errorMsg = String(e);
      step = "error";
    }
  }

  function handleDone() {
    onComplete();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") onClose();
  }

  $: hasChanges = preview
    ? preview.added > 0 || preview.updated > 0 || preview.deleted_files > 0 || preview.deleted_folders > 0 || preview.tags_to_backfill > 0
    : false;
</script>

<svelte:window on:keydown={handleKeydown} />

<div class="overlay" on:click={onClose} role="presentation">
  <div class="modal" on:click|stopPropagation on:keydown={handleKeydown} role="dialog" aria-modal="true" tabindex="-1">
    <h3 class="title">Update Catalog</h3>

    <div class="catalog-info">
      <span class="label">Catalog:</span>
      <span class="value">{catalog.name}</span>
    </div>
    <div class="catalog-info">
      <span class="label">Path:</span>
      <span class="value path">{catalog.root_path}</span>
    </div>

    {#if step === "ready"}
      <p class="hint">Scan the original path for changes since last import.</p>
      <div class="actions">
        <button class="btn btn-secondary" on:click={onClose}>Close</button>
        <button class="btn btn-primary" on:click={scanForChanges}>Scan for Changes</button>
      </div>

    {:else if step === "scanning"}
      <div class="status">
        <div class="spinner"></div>
        <span>Scanning for changes...</span>
      </div>

    {:else if step === "preview" && preview}
      <div class="preview-stats">
        <div class="stat-row">
          <span class="stat-label">Added files:</span>
          <span class="stat-value" class:has-change={preview.added > 0}>{preview.added}</span>
        </div>
        <div class="stat-row">
          <span class="stat-label">Updated files:</span>
          <span class="stat-value" class:has-change={preview.updated > 0}>{preview.updated}</span>
        </div>
        <div class="stat-row">
          <span class="stat-label">Deleted files:</span>
          <span class="stat-value" class:has-change={preview.deleted_files > 0}>{preview.deleted_files}</span>
        </div>
        <div class="stat-row">
          <span class="stat-label">Deleted folders:</span>
          <span class="stat-value" class:has-change={preview.deleted_folders > 0}>{preview.deleted_folders}</span>
        </div>
        <div class="stat-row">
          <span class="stat-label">Unchanged:</span>
          <span class="stat-value">{preview.unchanged}</span>
        </div>
        {#if preview.tags_to_backfill > 0}
          <div class="stat-row">
            <span class="stat-label">Media tags to extract:</span>
            <span class="stat-value has-change">{preview.tags_to_backfill}</span>
          </div>
        {/if}
      </div>

      {#if hasChanges}
        <p class="hint">Apply these changes to the catalog?</p>
        <div class="actions">
          <button class="btn btn-secondary" on:click={onClose}>Cancel</button>
          <button class="btn btn-primary" on:click={applyChanges}>Update</button>
        </div>
      {:else}
        <p class="hint no-changes">Catalog is up to date.</p>
        <div class="actions">
          <button class="btn btn-primary" on:click={onClose}>Close</button>
        </div>
      {/if}

    {:else if step === "applying"}
      <div class="status">
        <div class="spinner"></div>
        <span>Applying changes...</span>
      </div>

    {:else if step === "done" && result}
      <div class="preview-stats">
        <div class="stat-row">
          <span class="stat-label">Added:</span>
          <span class="stat-value" class:has-change={result.added > 0}>{result.added}</span>
        </div>
        <div class="stat-row">
          <span class="stat-label">Updated:</span>
          <span class="stat-value" class:has-change={result.updated > 0}>{result.updated}</span>
        </div>
        <div class="stat-row">
          <span class="stat-label">Deleted:</span>
          <span class="stat-value" class:has-change={result.deleted_files + result.deleted_folders > 0}>{result.deleted_files + result.deleted_folders}</span>
        </div>
        <div class="stat-row">
          <span class="stat-label">Unchanged:</span>
          <span class="stat-value">{result.unchanged}</span>
        </div>
      </div>
      <p class="hint success">Catalog updated successfully.</p>
      <div class="actions">
        <button class="btn btn-primary" on:click={handleDone}>Done</button>
      </div>

    {:else if step === "error"}
      <p class="error">{errorMsg}</p>
      <div class="actions">
        <button class="btn btn-secondary" on:click={onClose}>Close</button>
      </div>
    {/if}
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
    width: 420px;
    box-shadow: var(--shadow-lg);
  }

  .title {
    font-family: var(--font-family-body);
    font-size: 20px;
    font-weight: 500;
    color: var(--text-primary);
    margin-bottom: 18px;
    letter-spacing: -0.02em;
  }

  .catalog-info {
    display: flex;
    gap: 8px;
    margin-bottom: 6px;
    font-size: 13px;
  }

  .catalog-info .label {
    color: var(--text-muted);
    flex-shrink: 0;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    padding-top: 1px;
  }

  .catalog-info .value {
    color: var(--text-primary);
    font-weight: 500;
  }

  .catalog-info .path {
    font-family: var(--font-family-mono);
    font-size: 11.5px;
    font-weight: 400;
    word-break: break-all;
    color: var(--text-secondary);
  }

  .hint {
    font-size: 13px;
    color: var(--text-secondary);
    margin: 18px 0;
  }

  .hint.no-changes {
    color: var(--accent);
    font-weight: 500;
  }

  .hint.success {
    color: var(--accent);
    font-weight: 500;
  }

  .status {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 24px 0;
    color: var(--text-secondary);
    font-size: 13px;
  }

  .spinner {
    width: 16px;
    height: 16px;
    border: 2px solid var(--border);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .preview-stats {
    margin: 18px 0;
    padding: 14px 16px;
    background: var(--bg-base);
    border: 1px solid var(--border-subtle);
    border-radius: 8px;
  }

  .stat-row {
    display: flex;
    justify-content: space-between;
    padding: 5px 0;
    font-size: 13px;
  }

  .stat-label {
    color: var(--text-secondary);
  }

  .stat-value {
    color: var(--text-muted);
    font-family: var(--font-family-mono);
    font-size: 12px;
    font-variant-numeric: tabular-nums;
  }

  .stat-value.has-change {
    color: var(--accent);
    font-weight: 600;
  }

  .error {
    font-size: 13px;
    color: var(--danger, #d45555);
    margin: 16px 0;
    line-height: 1.5;
    padding: 10px 12px;
    background: var(--danger-bg, rgba(212, 85, 85, 0.06));
    border-radius: 6px;
    border: 1px solid rgba(212, 85, 85, 0.15);
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 18px;
  }

  .btn {
    padding: 8px 18px;
    border-radius: 8px;
    font-size: 13px;
    font-weight: 500;
    transition: all 0.12s;
  }

  .btn-secondary {
    color: var(--text-secondary);
  }

  .btn-secondary:hover {
    background: var(--bg-hover);
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
