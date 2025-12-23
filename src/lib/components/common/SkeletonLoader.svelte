<script lang="ts">
  export let width: string = "100%";
  export let height: string = "1rem";
  export let rounded: boolean = true;
</script>

<div
  class="animate-pulse bg-border {rounded ? 'rounded' : ''}"
  style="width: {width}; height: {height};"
  aria-label="Loading..."
  role="status"
></div>

