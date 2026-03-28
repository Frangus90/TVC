<script lang="ts">
  import { fade, scale } from "svelte/transition";
  import { X, Sparkles, ChevronDown, ChevronUp } from "lucide-svelte";
  import {
    isWhatsNewOpen,
    closeWhatsNew,
    getAppVersion,
  } from "../stores/whatsNew.svelte";
  import changelog from "../../../CHANGELOG.md?raw";

  let showOlderVersions = $state(false);

  interface ChangelogVersion {
    version: string;
    sections: { title: string; items: string[] }[];
  }

  function parseChangelog(): ChangelogVersion[] {
    const versions: ChangelogVersion[] = [];
    const lines = changelog.split("\n");
    let currentVersion: ChangelogVersion | null = null;
    let currentSection: { title: string; items: string[] } | null = null;

    for (const line of lines) {
      const trimmed = line.trim();

      // Version header: ## [0.8.1]
      const versionMatch = trimmed.match(/^##\s+\[(.+)\]/);
      if (versionMatch) {
        if (currentSection && currentVersion) {
          currentVersion.sections.push(currentSection);
        }
        currentVersion = { version: versionMatch[1], sections: [] };
        versions.push(currentVersion);
        currentSection = null;
        continue;
      }

      // Section header: ### Feature Name
      const sectionMatch = trimmed.match(/^###\s+(.+)/);
      if (sectionMatch && currentVersion) {
        if (currentSection) {
          currentVersion.sections.push(currentSection);
        }
        currentSection = { title: sectionMatch[1], items: [] };
        continue;
      }

      // Bullet item: - text
      if (trimmed.startsWith("- ") && currentSection) {
        currentSection.items.push(trimmed.slice(2));
        continue;
      }
    }

    // Push final section
    if (currentSection && currentVersion) {
      currentVersion.sections.push(currentSection);
    }

    return versions;
  }

  const allVersions = $derived(parseChangelog());
  const latestVersion = $derived(allVersions[0] || null);
  const olderVersions = $derived(allVersions.slice(1));
</script>

{#if isWhatsNewOpen()}
  <!-- Backdrop -->
  <button
    type="button"
    transition:fade={{ duration: 150 }}
    class="fixed inset-0 bg-black/60 z-50"
    onclick={closeWhatsNew}
    aria-label="Close modal"
  ></button>

  <!-- Modal -->
  <div
    transition:scale={{ duration: 200, start: 0.95, opacity: 0 }}
    class="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 z-50 bg-surface rounded-xl border border-border shadow-2xl w-full max-w-2xl max-h-[80vh] flex flex-col"
  >
    <!-- Header -->
    <div class="flex items-center justify-between p-5 border-b border-border">
      <div class="flex items-center gap-3">
        <div class="w-10 h-10 rounded-full bg-accent/20 flex items-center justify-center">
          <Sparkles class="w-5 h-5 text-accent" />
        </div>
        <div>
          <h2 class="text-lg font-semibold text-text">What's New</h2>
          <p class="text-sm text-text-muted">v{getAppVersion()}</p>
        </div>
      </div>
      <button
        type="button"
        onclick={closeWhatsNew}
        class="p-2 rounded-lg hover:bg-surface-hover transition-colors"
        aria-label="Close"
      >
        <X class="w-5 h-5 text-text-muted" />
      </button>
    </div>

    <!-- Content -->
    <div class="flex-1 overflow-auto p-5">
      {#if latestVersion}
        <!-- Latest version -->
        <div class="space-y-4">
          {#each latestVersion.sections as section}
            <div>
              <h3 class="text-sm font-semibold text-accent uppercase tracking-wide mb-2">
                {section.title}
              </h3>
              <ul class="space-y-1.5">
                {#each section.items as item}
                  <li class="text-sm text-text-muted leading-relaxed flex gap-2">
                    <span class="text-accent mt-1 flex-shrink-0">&#8226;</span>
                    <span>{@html item.replace(/\*\*(.+?)\*\*/g, '<span class="text-text font-medium">$1</span>')}</span>
                  </li>
                {/each}
              </ul>
            </div>
          {/each}
        </div>

        <!-- Older versions toggle -->
        {#if olderVersions.length > 0}
          <button
            type="button"
            onclick={() => showOlderVersions = !showOlderVersions}
            class="w-full flex items-center justify-center gap-2 mt-6 py-2 text-xs text-text-muted hover:text-text transition-colors border-t border-border"
          >
            {showOlderVersions ? "Hide" : "Show"} older versions
            {#if showOlderVersions}
              <ChevronUp class="w-3.5 h-3.5" />
            {:else}
              <ChevronDown class="w-3.5 h-3.5" />
            {/if}
          </button>

          {#if showOlderVersions}
            <div class="space-y-6 mt-4" transition:fade={{ duration: 150 }}>
              {#each olderVersions as version}
                <div>
                  <h3 class="text-xs font-semibold text-text-muted uppercase tracking-wide mb-3 pb-1 border-b border-border">
                    v{version.version}
                  </h3>
                  <div class="space-y-3">
                    {#each version.sections as section}
                      <div>
                        <h4 class="text-sm font-medium text-text mb-1.5">{section.title}</h4>
                        <ul class="space-y-1">
                          {#each section.items as item}
                            <li class="text-xs text-text-muted leading-relaxed flex gap-2">
                              <span class="text-text-muted mt-0.5 flex-shrink-0">&#8226;</span>
                              <span>{@html item.replace(/\*\*(.+?)\*\*/g, '<span class="text-text font-medium">$1</span>')}</span>
                            </li>
                          {/each}
                        </ul>
                      </div>
                    {/each}
                  </div>
                </div>
              {/each}
            </div>
          {/if}
        {/if}
      {:else}
        <p class="text-sm text-text-muted text-center py-8">No changelog entries found.</p>
      {/if}
    </div>
  </div>
{/if}
