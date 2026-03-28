<script lang="ts">
  import NotificationPopup from "./NotificationPopup.svelte";
  import {
    getActivePopups,
    getNotificationSettings,
    removePopup,
    markRead,
    openNotificationCenter,
  } from "../../stores/notifications.svelte";

  const popups = $derived(getActivePopups());
  const settings = $derived(getNotificationSettings());
  const duration = $derived(settings?.popup_duration ?? 8000);
  const position = $derived(settings?.popup_position ?? "top-right");

  const positionClasses: Record<string, string> = {
    "top-right": "top-16 right-4",
    "top-left": "top-16 left-4",
    "bottom-right": "bottom-4 right-4",
    "bottom-left": "bottom-4 left-4",
  };

  function handleDismiss(id: number) {
    removePopup(id);
  }

  function handleClick(id: number) {
    removePopup(id);
    markRead(id);
    openNotificationCenter();
  }
</script>

{#if popups.length > 0}
  <div
    class="fixed z-50 flex flex-col gap-2 pointer-events-none {positionClasses[position] || positionClasses['top-right']}"
  >
    {#each popups as popup (popup.id)}
      <div class="pointer-events-auto">
        <NotificationPopup
          notification={popup}
          {duration}
          onDismiss={() => handleDismiss(popup.id)}
          onClick={() => handleClick(popup.id)}
        />
      </div>
    {/each}
  </div>
{/if}
