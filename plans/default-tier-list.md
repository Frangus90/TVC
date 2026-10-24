# Default tier list

## Request

For 0.15.1, add a Shows/Movies dropdown to **Settings → Tier List** that controls which list opens by default. Keep changes on the Focus branch and use only development or disposable data. Never access the release database or sidecars.

## Plan

1. Extend the existing saved preferences with a default tier list; existing users keep Shows. Preserve compatibility with older preferences and portable backups.
2. Add an immediately saved dropdown above the tier layout editor. Opening Tier lists uses the saved choice, while switching lists remains available without changing the default or any rankings. Delayed loading must not override an explicit list choice.
3. Update the 0.15.1 changelog, check persistence, load/save failures and navigation using disposable fixtures, and run project checks. Do not change app versions or publish a release.

## Validation

- Implemented the dropdown independently of tier layout Apply. It saves through the existing preference queue and restores the confirmed selection on write failure.
- Existing preferences default to Shows. Old v4 portable backups remain accepted; new backups carry the chosen default. Unknown keys and invalid choices remain rejected. No migrations or app version changes.
- `npm run check`: **0 Svelte errors or warnings; 89 frontend tests passed; 126 Rust tests passed; 1 live Wikipedia test ignored; production frontend build passed**.
- Targeted tests cover preference writes queued alongside navigation, failed-save rollback, delayed preference loading after an explicit list choice/unmount, unchanged tier drafts, and old/new portable backup validation and restore.
- Real compiled UI verified using isolated in-memory IPC fixtures: saved Movies while editing a five-tier draft, opened Movies, switched to Shows, navigated away and reopened Movies by default. Returned to Settings with Movies still selected and the five-tier draft retained. Discarding the draft restored the original four saved tiers; no layout Apply was used.
- Inspected the settings row at **1920×1080** and **3840×2160** with no document horizontal overflow. Preview console had no warnings or errors. Local proof: `focus-redesign-screenshots/default-tier-list-1080p.jpg` (ignored artifact).
- Independent frontend/backend review found no blockers. `git diff --check` passed. Only in-memory or disposable test databases were used; the installed release database and its sidecars were never accessed.
- Runtime UI checks used fixtures rather than a native installed app. Only Shows and Movies are accepted values; failed loads offer Retry and failed saves keep the prior choice.

The feature remains on `codex/focus-redesign` for **0.15.1 Unreleased**. No release script, installer, tag, merge or publication is part of this task.
