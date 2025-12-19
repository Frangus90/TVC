<script lang="ts">
  import { X, Download, Clock, RefreshCw, Check } from "lucide-svelte";
  import {
    isUpdateModalOpen,
    getUpdateInfo,
    closeUpdateModal,
    isDownloadingUpdate,
    getDownloadProgress,
    downloadAndInstallUpdate,
  } from "../stores/updates.svelte";

  // Simple markdown parser for release notes
  function parseMarkdown(text: string): string {
    if (!text) return "";

    return text
      .split("\n")
      .map((line) => {
        // Headers: ### Title
        if (line.startsWith("### ")) {
          return `<h3 class="text-base font-semibold text-text mt-4 mb-2">${line.slice(4)}</h3>`;
        }
        // Bullets: - Item
        if (line.startsWith("- ")) {
          return `<li class="text-sm text-text-muted ml-4">${line.slice(2)}</li>`;
        }
        // Empty lines
        if (line.trim() === "") {
          return "";
        }
        // Regular text
        return `<p class="text-sm text-text-muted">${line}</p>`;
      })
      .join("\n");
  }
</script>

{#if isUpdateModalOpen()}
  {@const info = getUpdateInfo()}
  {@const downloading = isDownloadingUpdate()}
  {@const progress = getDownloadProgress()}

  <!-- Backdrop -->
  <button
    type="button"
    class="fixed inset-0 bg-black/60 z-50"
    onclick={closeUpdateModal}
    aria-label="Close modal"
  ></button>

  <!-- Modal -->
  <div
    class="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 z-50 bg-surface rounded-xl border border-border shadow-2xl w-[500px] max-w-[95vw] max-h-[80vh] flex flex-col"
  >
    <!-- Header -->
    <div class="flex items-center justify-between p-5 border-b border-border">
      <div class="flex items-center gap-3">
        <div class="w-10 h-10 rounded-full bg-accent/20 flex items-center justify-center">
          <Download class="w-5 h-5 text-accent" />
        </div>
        <div>
          <h2 class="text-lg font-semibold text-text">Update Available</h2>
          <p class="text-sm text-text-muted">Version {info?.version}</p>
        </div>
      </div>
      <button
        type="button"
        onclick={closeUpdateModal}
        class="p-2 rounded-lg hover:bg-surface-hover transition-colors"
        aria-label="Close"
      >
        <X class="w-5 h-5 text-text-muted" />
      </button>
    </div>

    <!-- Release Notes -->
    <div class="flex-1 overflow-auto p-5">
      <h3 class="text-sm font-medium text-text-muted uppercase tracking-wide mb-3">
        What's New
      </h3>
      <div class="bg-background rounded-lg p-4 max-h-[300px] overflow-auto">
        {#if info?.body}
          {@html parseMarkdown(info.body)}
        {:else}
          <p class="text-sm text-text-muted">No release notes available.</p>
        {/if}
      </div>
    </div>

    <!-- Footer / Actions -->
    <div class="p-5 border-t border-border">
      {#if downloading}
        <!-- Download Progress -->
        <div class="space-y-3">
          <div class="flex items-center justify-between text-sm">
            <span class="text-text-muted flex items-center gap-2">
              <RefreshCw class="w-4 h-4 animate-spin" />
              Downloading update...
            </span>
            <span class="text-text font-medium">{progress}%</span>
          </div>
          <div class="w-full h-2 bg-background rounded-full overflow-hidden">
            <div
              class="h-full bg-accent transition-all duration-300"
              style="width: {progress}%"
            ></div>
          </div>
          {#if progress === 100}
            <p class="text-sm text-available flex items-center gap-2">
              <Check class="w-4 h-4" />
              Download complete! Installing and restarting...
            </p>
          {/if}
        </div>
      {:else}
        <!-- Action Buttons -->
        <div class="flex items-center justify-end gap-3">
          <button
            type="button"
            onclick={closeUpdateModal}
            class="px-4 py-2 text-sm font-medium text-text-muted hover:text-text hover:bg-surface-hover rounded-lg transition-colors flex items-center gap-2"
          >
            <Clock class="w-4 h-4" />
            Later
          </button>
          <button
            type="button"
            onclick={downloadAndInstallUpdate}
            class="px-4 py-2 text-sm font-medium bg-accent hover:bg-accent/90 text-white rounded-lg transition-colors flex items-center gap-2"
          >
            <Download class="w-4 h-4" />
            Update Now
          </button>
        </div>
      {/if}
    </div>
  </div>
{/if}
