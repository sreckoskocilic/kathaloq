<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";

  export let onSubmit: (path: string, name: string) => void;
  export let onClose: () => void;

  let path = "";
  let name = "";

  async function pickFolder() {
    const selected = await open({ directory: true, multiple: false });
    if (selected) {
      path = selected as string;
      if (!name) {
        const parts = path.split(/[/\\]/);
        name = parts[parts.length - 1] || "Untitled";
      }
    }
  }

  function handleSubmit() {
    if (path && name) {
      onSubmit(path, name);
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") onClose();
  }
</script>

<svelte:window on:keydown={handleKeydown} />

<div class="overlay" on:click={onClose} role="presentation">
  <div class="modal" on:click|stopPropagation role="dialog" aria-modal="true" tabindex="-1" on:keydown={handleKeydown}>
    <div class="modal-header">
      <h2>New Catalog</h2>
      <button class="btn-close-x" on:click={onClose} aria-label="Close">
        <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
          <path d="M3 3l8 8M11 3l-8 8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
        </svg>
      </button>
    </div>

    <div class="field">
      <label for="catalog-name">Catalog name</label>
      <input
        id="catalog-name"
        type="text"
        bind:value={name}
        placeholder="My External Drive"
      />
    </div>

    <div class="field">
      <label for="catalog-path">Source folder</label>
      <div class="path-row">
        <input
          id="catalog-path"
          type="text"
          bind:value={path}
          placeholder="Select a folder..."
          readonly
        />
        <button class="btn-browse" on:click={pickFolder}>Browse</button>
      </div>
    </div>

    <div class="actions">
      <button class="btn-cancel" on:click={onClose}>Cancel</button>
      <button class="btn-submit" on:click={handleSubmit} disabled={!path || !name}>
        Start Scan
      </button>
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
    z-index: 100;
  }

  .modal {
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 24px;
    width: 440px;
    box-shadow: var(--shadow-lg);
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 24px;
  }

  h2 {
    font-family: var(--font-family-body);
    font-size: 22px;
    font-weight: 500;
    color: var(--text-primary);
    letter-spacing: -0.02em;
  }

  .btn-close-x {
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 6px;
    color: var(--text-muted);
    transition: all 0.12s;
  }

  .btn-close-x:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .field {
    margin-bottom: 16px;
  }

  label {
    display: block;
    font-size: 11px;
    font-weight: 600;
    color: var(--text-muted);
    margin-bottom: 6px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  .field input {
    width: 100%;
    padding: 9px 12px;
    background: var(--bg-base);
    border: 1px solid var(--border-subtle);
    border-radius: 8px;
    font-size: 13px;
    color: var(--text-primary);
    transition: all 0.15s;
  }

  .field input:focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 2px var(--accent-muted);
  }

  .path-row {
    display: flex;
    gap: 8px;
  }

  .path-row input {
    flex: 1;
  }

  .btn-browse {
    padding: 9px 16px;
    background: var(--bg-hover);
    border: 1px solid var(--border);
    border-radius: 8px;
    font-size: 12.5px;
    font-weight: 500;
    color: var(--text-secondary);
    white-space: nowrap;
    transition: all 0.12s;
  }

  .btn-browse:hover {
    background: var(--accent-muted);
    color: var(--accent);
    border-color: var(--accent);
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 24px;
  }

  .btn-cancel {
    padding: 8px 16px;
    border-radius: 8px;
    font-size: 13px;
    color: var(--text-secondary);
    transition: all 0.12s;
  }

  .btn-cancel:hover {
    background: var(--bg-hover);
  }

  .btn-submit {
    padding: 8px 20px;
    border-radius: 8px;
    font-size: 13px;
    font-weight: 600;
    background: var(--accent);
    color: #fff;
    transition: all 0.15s;
  }

  .btn-submit:hover:not(:disabled) {
    background: var(--accent-hover);
    transform: translateY(-1px);
    box-shadow: var(--shadow-sm);
  }

  .btn-submit:disabled {
    opacity: 0.35;
    cursor: not-allowed;
  }
</style>
