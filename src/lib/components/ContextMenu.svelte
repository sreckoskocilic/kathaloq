<script lang="ts">
  export let x: number;
  export let y: number;
  export let items: { label: string; action: () => void; danger?: boolean }[];
  export let onClose: () => void;

  function handleClick(action: () => void) {
    action();
    onClose();
  }

  function handleWindowClick() {
    onClose();
  }
</script>

<svelte:window on:click={handleWindowClick} on:contextmenu={handleWindowClick} />

<div class="context-menu" style:left="{x}px" style:top="{y}px" on:click|stopPropagation on:keydown|stopPropagation role="menu" tabindex="-1">
  {#each items as item (item.label)}
    <button
      class="menu-item"
      class:danger={item.danger}
      on:click={() => handleClick(item.action)}
      role="menuitem"
    >
      {item.label}
    </button>
  {/each}
</div>

<style>
  .context-menu {
    position: fixed;
    z-index: 300;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: 7px;
    padding: 4px;
    min-width: 180px;
    box-shadow: var(--shadow-lg);
  }

  .menu-item {
    display: block;
    width: 100%;
    padding: 7px 12px;
    font-size: 13.5px;
    color: var(--text-primary);
    text-align: left;
    border-radius: 5px;
    transition: background 0.08s;
  }

  .menu-item:hover {
    background: var(--bg-hover);
  }

  .menu-item.danger {
    color: var(--danger, #d45555);
  }

  .menu-item.danger:hover {
    background: var(--danger-bg, rgba(212, 85, 85, 0.1));
  }
</style>
