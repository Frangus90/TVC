<script lang="ts">
  import { Tv, Film } from "lucide-svelte";
  import { getIsDragging, getDragData, getGhostPosition } from "../../stores/dragDrop.svelte";

  const isDragging = $derived(getIsDragging());
  const dragData = $derived(getDragData());
  const position = $derived(getGhostPosition());
</script>

{#if isDragging && dragData}
  <div
    class="fixed pointer-events-none z-[9999] flex items-center gap-2 px-3 py-2 bg-surface border border-accent rounded-lg shadow-lg transform -translate-x-1/2 -translate-y-1/2"
    style="left: {position.x}px; top: {position.y}px;"
  >
    {#if dragData.type === "show"}
      <Tv class="w-4 h-4 text-accent" />
      <span class="text-sm text-text">Dragging show...</span>
    {:else}
      <Film class="w-4 h-4 text-accent" />
      <span class="text-sm text-text">Dragging movie...</span>
    {/if}
  </div>
{/if}
