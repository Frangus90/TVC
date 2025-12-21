<script lang="ts">
  import Toast from "./common/Toast.svelte";
  import { getToasts, removeToast } from "../stores/toast.svelte";
</script>

<div class="fixed bottom-4 right-4 z-50 flex flex-col gap-2 pointer-events-none">
  {#each getToasts() as toast}
    <div class="pointer-events-auto">
      <Toast
        message={toast.message}
        type={toast.type}
        duration={toast.duration}
        onClose={() => removeToast(toast.id)}
      />
    </div>
  {/each}
</div>







