<script lang="ts">
  import { Film, Tv, X } from "lucide-svelte";
  import { modalFocus } from "../utils/modalFocus";
  import { formatDateTime } from "../utils/dateFormat";
  import type { WatchHistoryItem } from "../stores/statistics.svelte";

  let { item, onClose }: { item: WatchHistoryItem; onClose: () => void } = $props();
</script>

<button
  type="button"
  class="fixed inset-0 bg-black/60 z-[60]"
  onclick={onClose}
  aria-label="Close Plex watch preview"
></button>

<div
  role="dialog"
  aria-modal="true"
  aria-label="Plex watch details"
  use:modalFocus={onClose}
  class="focus-dialog fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 z-[60] bg-surface rounded-xl border border-border shadow-2xl w-[600px] max-w-[95vw] max-h-[90vh] overflow-auto"
>
  <div class="flex items-start gap-4 p-6 border-b border-border">
    <div class="w-20 h-28 rounded bg-background border border-border flex items-center justify-center flex-shrink-0">
      {#if item.item_type === "movie"}
        <Film class="w-7 h-7 text-text-muted" />
      {:else}
        <Tv class="w-7 h-7 text-text-muted" />
      {/if}
    </div>
    <div class="flex-1 min-w-0">
      <div class="flex items-start justify-between gap-3">
        <div>
          <p class="text-xs uppercase tracking-wide text-accent mb-1">Plex play · Preview</p>
          <h2 class="text-xl font-semibold text-text">
            {item.item_type === "episode" ? item.show_name || "Unknown show" : item.name}
          </h2>
          {#if item.item_type === "episode"}
            <p class="text-sm text-text-muted mt-1">
              {#if item.season_number !== null}Season {item.season_number}, {/if}
              {#if item.episode_number !== null}Episode {item.episode_number}{/if}
              {#if item.name} · {item.name}{/if}
            </p>
          {:else if item.year}
            <p class="text-sm text-text-muted mt-1">{item.year}</p>
          {/if}
        </div>
        <button
          type="button"
          onclick={onClose}
          class="p-2 rounded-lg hover:bg-surface-hover transition-colors flex-shrink-0"
          aria-label="Close"
        >
          <X class="w-5 h-5 text-text-muted" />
        </button>
      </div>
      <p class="text-sm text-text-muted mt-4">Watched {formatDateTime(item.watched_at)}</p>
      <p class="text-xs text-text-muted mt-4">This title is not in your TVC library. This preview does not add or change anything.</p>
    </div>
  </div>
</div>
