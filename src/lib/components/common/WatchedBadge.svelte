<script lang="ts">
  import { Check } from "lucide-svelte";
</script>

<span class="focus-watched-badge"><Check class="watched-check" strokeWidth={2.5} aria-hidden="true" />Watched</span>

<style>
  .focus-watched-badge { display: inline-flex; align-items: center; gap: 0.2rem; flex: none; padding: 0.12rem 0.35rem; border-radius: 0.3rem; background: var(--focus-watched); color: var(--focus-watched-ink); font-size: 0.625rem; font-weight: 700; line-height: 1.4; white-space: nowrap; }
  .focus-watched-badge :global(.watched-check) { width: 0.75rem; height: 0.75rem; }
</style>
