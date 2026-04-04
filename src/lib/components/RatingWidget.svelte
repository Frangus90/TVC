<script lang="ts">
  import StarRating from "./StarRating.svelte";
  import TierPicker from "./TierPicker.svelte";
  import { getTiers, getTierPreset, getTierById } from "../stores/tiers.svelte";

  interface Props {
    tierId: number | null;
    onTierChange: (tierId: number | null) => void;
  }

  let { tierId, onTierChange }: Props = $props();

  const preset = $derived(getTierPreset());
  const tiers = $derived(getTiers());

  // For star presets, convert tier position to a star rating value
  const starRating = $derived.by(() => {
    if (!tierId) return null;
    const tier = getTierById(tierId);
    if (!tier) return null;

    if (preset === "10-star") {
      // Position 1-10 maps to 0.5-5.0
      return tier.position / 2;
    } else if (preset === "5-star") {
      // Position 1-5 maps to 1.0-5.0
      return tier.position;
    }
    return null;
  });

  // When star is clicked, find the matching tier by position
  function handleStarChange(starValue: number | null) {
    if (starValue === null) {
      onTierChange(null);
      return;
    }

    let targetPosition: number;
    if (preset === "10-star") {
      // Star value 0.5-5.0 maps to position 1-10
      targetPosition = Math.round(starValue * 2);
    } else {
      // Star value 1-5 maps to position 1-5
      targetPosition = Math.round(starValue);
    }

    const matchingTier = tiers.find(t => t.position === targetPosition);
    onTierChange(matchingTier?.id ?? null);
  }
</script>

{#if preset === "custom" || (preset !== "10-star" && preset !== "5-star")}
  <TierPicker {tierId} {onTierChange} />
{:else}
  <StarRating rating={starRating} onRatingChange={handleStarChange} />
{/if}
