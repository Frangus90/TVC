<script lang="ts">
  export let icon: any = null;
  export let title: string;
  export let message: string;
  export let action: { label: string; onclick: () => void } | null = null;
</script>

<div class="flex flex-col items-center justify-center gap-3 p-12 text-center">
  {#if icon}
    <svelte:component this={icon} class="w-16 h-16 text-text-muted" />
  {/if}
  <h3 class="text-lg font-semibold text-text">{title}</h3>
  <p class="text-sm text-text-muted max-w-md">{message}</p>
  {#if action}
    <button
      onclick={action.onclick}
      class="mt-2 px-4 py-2 bg-accent hover:bg-accent-hover text-white rounded-lg transition-colors"
    >
      {action.label}
    </button>
  {/if}
</div>










