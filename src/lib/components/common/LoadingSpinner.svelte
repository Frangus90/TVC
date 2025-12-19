<script lang="ts">
  export let size: "sm" | "md" | "lg" = "md";
  export let text: string | null = null;

  const sizeClasses = {
    sm: "w-4 h-4",
    md: "w-8 h-8",
    lg: "w-12 h-12",
  };
</script>

<div class="flex flex-col items-center justify-center gap-2 p-4">
  <div
    class="border-2 border-border border-t-accent rounded-full animate-spin {sizeClasses[size]}"
  ></div>
  {#if text}
    <p class="text-sm text-text-muted">{text}</p>
  {/if}
</div>




