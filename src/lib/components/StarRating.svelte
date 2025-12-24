<script lang="ts">
  import { Star } from "lucide-svelte";

  interface Props {
    rating: number | null;
    onRatingChange: (rating: number | null) => void;
  }

  let { rating, onRatingChange }: Props = $props();
  let hoverValue = $state<number | null>(null);

  function handleClick(starIndex: number, isRightHalf: boolean) {
    const clickedValue = isRightHalf ? starIndex : starIndex - 0.5;
    const newRating = rating === clickedValue ? null : clickedValue;
    onRatingChange(newRating);
  }

  function getStarFill(starIndex: number): "full" | "half" | "empty" {
    if (rating === null) return "empty";
    if (rating >= starIndex) return "full";
    if (rating >= starIndex - 0.5) return "half";
    return "empty";
  }

  function getHoverFill(starIndex: number): "full" | "half" | "empty" {
    if (hoverValue === null) return "empty";
    if (hoverValue >= starIndex) return "full";
    if (hoverValue >= starIndex - 0.5) return "half";
    return "empty";
  }
</script>

<div class="flex items-center gap-0.5" role="group" aria-label="Rating" onmouseleave={() => hoverValue = null}>
  {#each [1, 2, 3, 4, 5] as star}
    {@const fill = getStarFill(star)}
    {@const hover = getHoverFill(star)}
    <div class="relative w-6 h-6 cursor-pointer">
      <!-- Background empty star -->
      <Star class="absolute inset-0 w-6 h-6 text-text-muted transition-colors" />

      <!-- Hover preview (muted yellow filled) -->
      {#if hoverValue !== null && hover === "half"}
        <div class="absolute inset-0 overflow-hidden" style="width: 50%;">
          <Star class="w-6 h-6 fill-yellow-400/60 text-yellow-400/60" />
        </div>
      {:else if hoverValue !== null && hover === "full"}
        <Star class="absolute inset-0 w-6 h-6 fill-yellow-400/60 text-yellow-400/60" />
      {/if}

      <!-- Current rating (solid yellow) -->
      {#if fill === "half"}
        <div class="absolute inset-0 overflow-hidden" style="width: 50%;">
          <Star class="w-6 h-6 fill-yellow-400 text-yellow-400" />
        </div>
      {:else if fill === "full"}
        <Star class="absolute inset-0 w-6 h-6 fill-yellow-400 text-yellow-400" />
      {/if}

      <!-- Invisible click/hover targets for left/right halves -->
      <button
        type="button"
        class="absolute inset-y-0 left-0 w-1/2 z-10"
        onclick={() => handleClick(star, false)}
        onmouseenter={() => hoverValue = star - 0.5}
        aria-label={`Rate ${star - 0.5} stars`}
      ></button>
      <button
        type="button"
        class="absolute inset-y-0 right-0 w-1/2 z-10"
        onclick={() => handleClick(star, true)}
        onmouseenter={() => hoverValue = star}
        aria-label={`Rate ${star} stars`}
      ></button>
    </div>
  {/each}
</div>
