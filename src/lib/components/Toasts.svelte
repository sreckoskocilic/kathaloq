<script lang="ts">
  import { notices, dismissNotice } from "../stores/notify";
</script>

{#if $notices.length > 0}
  <div class="stack" role="alert" aria-live="assertive">
    {#each $notices as notice (notice.id)}
      <div class="toast">
        <svg width="16" height="16" viewBox="0 0 20 20" fill="none" aria-hidden="true">
          <path
            d="M10 2L18 17H2L10 2z"
            stroke="currentColor"
            stroke-width="1.5"
            stroke-linejoin="round"
          />
          <path
            d="M10 8v4M10 14v0.5"
            stroke="currentColor"
            stroke-width="1.5"
            stroke-linecap="round"
          />
        </svg>
        <span class="message">{notice.message}</span>
        <button class="close" on:click={() => dismissNotice(notice.id)} aria-label="Dismiss"
          >×</button
        >
      </div>
    {/each}
  </div>
{/if}

<style>
  .stack {
    position: fixed;
    bottom: 44px;
    right: 16px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    z-index: 300;
    max-width: 420px;
  }

  .toast {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 11px 12px;
    border-radius: 10px;
    background: var(--bg-elevated);
    border: 1px solid var(--danger, #d45555);
    box-shadow: var(--shadow-lg);
    color: var(--danger, #d45555);
  }

  .message {
    flex: 1;
    font-size: 12.5px;
    line-height: 1.5;
    color: var(--text-primary);
    word-break: break-word;
  }

  .close {
    font-size: 16px;
    line-height: 1;
    padding: 0 2px;
    color: var(--text-secondary);
  }

  .close:hover {
    color: var(--text-primary);
  }
</style>
