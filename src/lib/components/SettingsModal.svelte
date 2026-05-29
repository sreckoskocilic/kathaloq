<script lang="ts">
  import { columns, toggleColumn } from "../stores/settings";
  import { theme } from "../stores/theme";
  import { getThirdPartyLicenses } from "../services/tauri";
  import type { Theme } from "../types";

  export let onClose: () => void;

  const themes: { id: Theme; label: string; desc: string }[] = [
    { id: "deep-void", label: "Deep Void", desc: "Near-black, electric blue" },
    { id: "warm-carbon", label: "Warm Carbon", desc: "Charcoal with amber glow" },
    { id: "sage", label: "Sage", desc: "Earthy olive green" },
  ];

  let showLicenses = false;
  let licensesHtml = "";
  let licensesLoading = false;

  async function openLicenses() {
    showLicenses = true;
    if (licensesHtml) return;
    licensesLoading = true;
    try {
      licensesHtml = await getThirdPartyLicenses();
    } catch (e) {
      console.error(e);
      licensesHtml = "<p>Failed to load licenses.</p>";
    }
    licensesLoading = false;
  }

  function closeLicenses() {
    showLicenses = false;
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key !== "Escape") return;
    if (showLicenses) closeLicenses();
    else onClose();
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

    <section>
      <h3>About</h3>
      <button class="btn-licenses" on:click={openLicenses}>Open Source Licenses</button>
    </section>

    <div class="actions">
      <button class="btn-done" on:click={onClose}>Done</button>
    </div>
  </div>
</div>

{#if showLicenses}
  <div class="overlay licenses-overlay" on:click={closeLicenses} role="presentation">
    <div
      class="licenses-modal"
      on:click|stopPropagation
      on:keydown={handleKeydown}
      role="dialog"
      aria-modal="true"
      tabindex="-1"
    >
      <div class="modal-header">
        <h2>Open Source Licenses</h2>
        <button class="btn-close-x" on:click={closeLicenses} aria-label="Close">
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
      {#if licensesLoading}
        <p class="lic-status">Loading…</p>
      {:else}
        <iframe class="lic-frame" title="Third party licenses" sandbox="" srcdoc={licensesHtml}
        ></iframe>
      {/if}
    </div>
  </div>
{/if}

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

  .theme-preview[data-theme-preview="deep-void"] {
    background: linear-gradient(135deg, #0f1012 0%, #1a1c20 40%, #262830 100%);
    border: 1px solid #2e3340;
    box-shadow: inset 0 0 12px rgba(107, 159, 255, 0.06);
  }

  .theme-preview[data-theme-preview="warm-carbon"] {
    background: linear-gradient(135deg, #141210 0%, #1e1c18 40%, #2a2722 100%);
    border: 1px solid #342f28;
    box-shadow: inset 0 0 12px rgba(232, 168, 76, 0.06);
  }

  .theme-preview[data-theme-preview="sage"] {
    background: linear-gradient(135deg, #111410 0%, #1a1e18 40%, #242820 100%);
    border: 1px solid #303828;
    box-shadow: inset 0 0 12px rgba(140, 192, 96, 0.06);
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

  .btn-licenses {
    padding: 8px 14px;
    border: 1px solid var(--border);
    border-radius: 8px;
    font-size: 13px;
    color: var(--text-primary);
    transition: all 0.12s;
  }

  .btn-licenses:hover {
    background: var(--bg-hover);
    border-color: var(--text-muted);
  }

  .licenses-overlay {
    z-index: 110;
  }

  .licenses-modal {
    display: flex;
    flex-direction: column;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 24px;
    width: min(760px, 90vw);
    height: min(80vh, 700px);
    box-shadow: var(--shadow-lg);
  }

  .lic-frame {
    flex: 1;
    width: 100%;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: #fff;
  }

  .lic-status {
    color: var(--text-muted);
    font-size: 13px;
  }
</style>
