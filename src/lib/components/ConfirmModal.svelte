<script lang="ts">
  export let message: string;
  export let onConfirm: () => void;
  export let onCancel: () => void;

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") onCancel();
    if (e.key === "Enter") onConfirm();
  }
</script>

<svelte:window on:keydown={handleKeydown} />

<div class="overlay" on:click={onCancel} role="presentation">
  <div class="modal" on:click|stopPropagation on:keydown={handleKeydown} role="alertdialog" aria-modal="true" tabindex="-1">
    <div class="icon-wrap">
      <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
        <path d="M10 2L18 17H2L10 2z" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/>
        <path d="M10 8v4M10 14v0.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
      </svg>
    </div>
    <p class="message">{message}</p>
    <div class="actions">
      <button class="btn btn-cancel" on:click={onCancel}>Cancel</button>
      <button class="btn btn-confirm" on:click={onConfirm}>Confirm</button>
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
    width: 380px;
    box-shadow: var(--shadow-lg);
  }

  .icon-wrap {
    width: 36px;
    height: 36px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 8px;
    background: var(--danger-bg, rgba(212, 85, 85, 0.1));
    color: var(--danger, #d45555);
    margin-bottom: 16px;
  }

  .message {
    font-size: 13.5px;
    color: var(--text-primary);
    margin-bottom: 24px;
    line-height: 1.6;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .btn {
    padding: 8px 18px;
    border-radius: 8px;
    font-size: 13px;
    font-weight: 500;
    transition: all 0.12s;
  }

  .btn-cancel {
    color: var(--text-secondary);
  }

  .btn-cancel:hover {
    background: var(--bg-hover);
  }

  .btn-confirm {
    background: var(--danger, #d45555);
    color: #fff;
  }

  .btn-confirm:hover {
    background: #e06060;
    transform: translateY(-1px);
    box-shadow: var(--shadow-sm);
  }
</style>
