<script lang="ts">
  import { columns, toggleColumn } from "../stores/settings";
  import { theme } from "../stores/theme";
  import type { Theme } from "../types";

  export let onClose: () => void;

  const themes: { id: Theme; label: string; desc: string }[] = [
    { id: "obsidian", label: "Obsidian", desc: "Neutral dark, blue accents" },
    { id: "ember", label: "Ember", desc: "Deep indigo night" },
    { id: "slate", label: "Slate", desc: "Warm charcoal" },
    { id: "terminal", label: "Terminal", desc: "Green phosphor CRT" },
  ];

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") onClose();
  }
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
    <div class="modal-header">
      <h2>Settings</h2>
      <button class="btn-close-x" on:click={onClose} aria-label="Close">
        <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
          <path
            d="M3 3l8 8M11 3l-8 8"
            stroke="currentColor"
            stroke-width="1.5"
            stroke-linecap="round"
          />
        </svg>
      </button>
    </div>

    <section>
      <h3>Theme</h3>
      <div class="theme-grid">
        {#each themes as t (t.id)}
          <button
            class="theme-option"
            class:active={$theme === t.id}
            on:click={() => theme.set(t.id)}
          >
            <div class="theme-preview" data-theme-preview={t.id}></div>
            <span class="theme-label">{t.label}</span>
            <span class="theme-desc">{t.desc}</span>
          </button>
        {/each}
      </div>
    </section>

    <section>
      <h3>Columns</h3>
      <div class="column-list">
        {#each $columns as col (col.id)}
          <label class="column-toggle">
            <input
              type="checkbox"
              checked={col.visible}
              disabled={col.id === "name"}
              on:change={() => toggleColumn(col.id)}
            />
            <span class="toggle-label">{col.label}</span>
            {#if col.id === "name"}
              <span class="badge">Required</span>
            {/if}
          </label>
        {/each}
      </div>
    </section>

    <div class="actions">
      <button class="btn-done" on:click={onClose}>Done</button>
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
    width: 420px;
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

  section {
    margin-bottom: 24px;
  }

  h3 {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-muted);
    margin-bottom: 12px;
  }

  .theme-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 10px;
  }

  .theme-option {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    padding: 12px 10px;
    border: 1px solid var(--border);
    border-radius: 10px;
    transition: all 0.15s;
  }

  .theme-option:hover {
    border-color: var(--text-muted);
  }

  .theme-option.active {
    border-color: var(--accent);
    background: var(--accent-muted);
  }

  .theme-preview {
    width: 100%;
    height: 36px;
    border-radius: 6px;
  }

  .theme-preview[data-theme-preview="obsidian"] {
    background: linear-gradient(135deg, #1a1b1e 0%, #202124 40%, #282a2d 100%);
    border: 1px solid #33363a;
  }

  .theme-preview[data-theme-preview="ember"] {
    background: linear-gradient(135deg, #181a24 0%, #1e2030 40%, #252838 100%);
    border: 1px solid #2d3044;
  }

  .theme-preview[data-theme-preview="slate"] {
    background: linear-gradient(135deg, #1e2024 0%, #24262b 40%, #2b2e33 100%);
    border: 1px solid #343740;
  }

  .theme-preview[data-theme-preview="terminal"] {
    background: linear-gradient(135deg, #0c100c 0%, #111611 40%, #1a201a 100%);
    border: 1px solid #243024;
    box-shadow: inset 0 0 12px rgba(80, 184, 80, 0.06);
  }

  .theme-label {
    font-size: 12.5px;
    font-weight: 500;
    color: var(--text-primary);
  }

  .theme-desc {
    font-size: 10px;
    color: var(--text-muted);
  }

  .column-list {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .column-toggle {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 13px;
    color: var(--text-primary);
    cursor: pointer;
  }

  .column-toggle input {
    accent-color: var(--accent);
    width: 15px;
    height: 15px;
  }

  .column-toggle input:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }

  .toggle-label {
    flex: 1;
  }

  .badge {
    font-size: 9px;
    font-weight: 600;
    padding: 2px 6px;
    border-radius: 4px;
    background: var(--bg-hover);
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    padding-top: 4px;
  }

  .btn-done {
    padding: 8px 20px;
    border-radius: 8px;
    font-size: 13px;
    font-weight: 600;
    background: var(--accent);
    color: #fff;
    transition: all 0.15s;
    letter-spacing: -0.01em;
  }

  .btn-done:hover {
    background: var(--accent-hover);
    transform: translateY(-1px);
    box-shadow: var(--shadow-sm);
  }
</style>
