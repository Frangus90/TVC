/** Release notes support limited formatting; source HTML must remain text. */
export function inlineReleaseNotes(text: string): string {
  return text
    .replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;").replace(/'/g, "&#39;")
    .replace(/\*\*(.+?)\*\*/g, '<strong class="text-text font-medium">$1</strong>')
    .replace(/`(.+?)`/g, '<code class="px-1 py-0.5 rounded bg-surface-hover text-xs font-mono">$1</code>');
}
