# Settings improvements

## Scope and safeguards

- Implement the accepted settings review on `codex/focus-redesign`, starting from `823b5d2`.
- Implement correctness fixes before new preferences and presentation changes.
- Use only isolated preview fixtures and disposable tests or `tvc_dev.db`. Never access release data or its sidecars.
- Keep the original app recovery tag/bundle and main branch intact.
- Keep DD.MM.YYYY dates, HH:mm times, existing saved preferences, integrations, and personal library/history intact.
- Do not edit released migrations or manually bump version files. Add human-readable changelog notes.

## User's tier-editor requirement

Presets and all tier edits operate on a local draft. Add/edit/delete/reorder/reset must not write current tiers or assignments. Only Apply, followed by an explicit warning, replaces the tier configuration and clears previous tier assignments/order for both movies and TV shows. The warning states that titles, watched status, and viewing history remain. Cancel/Discard writes nothing. Apply validates all definitions before one transaction; a failure rolls back everything.

Tiermaker import is a separate confirmed import workflow. Parsing/searching must not create or reorder tiers. Review shows planned changes before Import; movie/show choice cannot change while searching or importing.

## Stages

1. Fix settings saves/validation/error feedback, Plex start persistence, missing racing categories, unsupported custom preset, and premature Tiermaker writes. Add regression tests using mocks/memory databases. Checkpoint.
2. Add draft tier editor/confirmed Apply, clearer notification delivery/quiet hours/test button, compact settings rows, racing filters/effective reminder state, and collapsible setup/import tools.
3. Add General preferences (startup page, remembered calendar, tray/quit, installed-app Windows autostart and start-in-tray), release-country preference, independent poster sizing, and grouped Integrations. Complete saved-server connection checks and full import error visibility.
4. Validate with full checks, isolated visual/interaction tests at desktop sizes and larger text, update this record/changelog, and commit/push the Focus branch.

## Implementation ownership

- Tier/Racing agent: tier editor/store/backend Apply transaction, Tiermaker preview, racing settings/store, associated tests.
- Appearance/Notifications agent: stores and settings UI, quiet-hours backend/frontend delivery, validation and queue tests.
- General agent: new preferences store/tab/backend, Windows integration, validation tests.
- Root: integration fixes/UI, navigation/shell preference hooks, regional release lookup wiring, poster sizing, command registration, visual QA and checkpoints.

## Defaults and compatibility

- Quiet hours off by default (22:00–08:00 when enabled); preserve history while muting delivery.
- New General defaults preserve existing behavior: Calendar, Month, all titles, close to tray, US release-country lookup. Remembered calendar and Windows autostart are opt-in.
- Autostart is available in installed builds. Dev builds do not register a debug executable that would require a running Vite server at login.
- Start-in-tray applies to Windows autostart launches; manual launches stay visible.
- Poster size Medium preserves the existing dimensions; Small/Large scale library and tier-board posters independently of font size and compact spacing.
- New values use the existing generic settings table where feasible; no schema change is expected.

## Progress

- Correctness checkpoint complete: full checks passed with 0 Svelte errors/warnings, 34 frontend tests and 87 Rust tests (1 live Wikipedia test ignored), and production build.
- Tier configuration tests cover confirmed replacement, title/history/schedule preservation, rollback on failure, and rejected invalid drafts. Tier drafts persist in memory across page navigation.
- The draft editor, keyboard ordering, collapsed import, and racing followed/all controls share the rewritten UI and are included in the first checkpoint.
- General/preferences and notification quiet-hours work proceeds after this checkpoint.
