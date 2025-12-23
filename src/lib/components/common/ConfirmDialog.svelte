<script lang="ts">
  import { fade, scale } from "svelte/transition";
  import { AlertTriangle, Info, X } from "lucide-svelte";
  import {
    isConfirmDialogOpen,
    getConfirmDialogOptions,
    closeConfirmDialog,
    type ConfirmDialogType,
  } from "../../stores/confirmDialog.svelte";

  function getIcon(type: ConfirmDialogType | undefined) {
    switch (type) {
      case "danger":
      case "warning":
        return AlertTriangle;
      case "info":
        return Info;
      default:
        return AlertTriangle;
    }
  }

  function getTypeClasses(type: ConfirmDialogType | undefined) {
    switch (type) {
      case "danger":
        return {
          icon: "text-red-400",
          button: "bg-red-500 hover:bg-red-600 text-white",
        };
      case "warning":
        return {
          icon: "text-yellow-400",
          button: "bg-yellow-500 hover:bg-yellow-600 text-white",
        };
      case "info":
        return {
          icon: "text-blue-400",
          button: "bg-blue-500 hover:bg-blue-600 text-white",
        };
      default:
        return {
          icon: "text-text-muted",
          button: "bg-accent hover:bg-accent-hover text-white",
        };
    }
  }

  function handleConfirm() {
    closeConfirmDialog(true);
  }

  function handleCancel() {
    closeConfirmDialog(false);
  }

  function handleKeyDown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      handleCancel();
    } else if (event.key === "Enter") {
      handleConfirm();
    }
  }
</script>

<svelte:window on:keydown={handleKeyDown} />

{#if isConfirmDialogOpen()}
  {@const options = getConfirmDialogOptions()}
  {@const type = options?.type || "info"}
  {@const Icon = getIcon(type)}
  {@const classes = getTypeClasses(type)}

  <!-- Backdrop -->
  <div
    transition:fade={{ duration: 150 }}
    class="fixed inset-0 bg-black/60 z-50"
    onclick={handleCancel}
    role="button"
    tabindex="0"
    onkeydown={(e) => { if (e.key === 'Escape' || e.key === 'Enter' || e.key === ' ') { handleCancel(); } }}
    aria-label="Close dialog"
  ></div>

  <!-- Dialog -->
  <div
    transition:scale={{ duration: 200, start: 0.95, opacity: 0 }}
    class="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 z-[60] bg-surface rounded-xl border border-border shadow-2xl w-[400px] max-w-[95vw]"
    role="dialog"
    aria-modal="true"
    aria-labelledby="confirm-dialog-title"
  >
    <div class="p-6">
      <!-- Icon and Title -->
      <div class="flex items-start gap-4 mb-4">
        <div class="flex-shrink-0">
          <Icon class="w-6 h-6 {classes.icon}" />
        </div>
        <div class="flex-1 min-w-0">
          <h2 id="confirm-dialog-title" class="text-lg font-semibold text-text mb-1">
            {options?.title || "Confirm"}
          </h2>
          <p class="text-sm text-text-muted">
            {options?.message || ""}
          </p>
        </div>
        <button
          onclick={handleCancel}
          class="p-1.5 rounded-lg hover:bg-surface-hover transition-colors flex-shrink-0"
          aria-label="Close"
        >
          <X class="w-4 h-4 text-text-muted" />
        </button>
      </div>

      <!-- Actions -->
      <div class="flex items-center justify-end gap-3 mt-6">
        <button
          onclick={handleCancel}
          class="px-4 py-2 text-sm text-text-muted hover:text-text hover:bg-surface-hover rounded-lg transition-colors"
        >
          {options?.cancelLabel || "Cancel"}
        </button>
        <button
          onclick={handleConfirm}
          class="px-4 py-2 text-sm font-medium rounded-lg transition-colors {classes.button}"
        >
          {options?.confirmLabel || "Confirm"}
        </button>
      </div>
    </div>
  </div>
{/if}

