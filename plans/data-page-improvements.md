# Data page improvements

## Scope and safeguards

- Implement the approved Data review on `codex/focus-redesign`, starting from `b582199`.
- Fix correctness issues and validate/checkpoint them before implementing the presentation changes and new features.
- Use only isolated preview fixtures and disposable test databases or `tvc_dev.db`. Never open, inspect, copy, change, or back up release `tvc.db` or its sidecars.
- Preserve `main`, the original recovery tag, and the pre-Focus Git bundle.
- Keep existing library/watch history, tiers, awards, integrations, and schedules working. Use DD.MM.YYYY and HH:mm for display.
- Do not edit released migrations or manually bump app versions. Add plain-language changelog notes.

## Stage 1: correctness fixes

- Match cleanup counts/previews/deletions using shared safe predicates: protect watch history, schedules, ratings, tags, change history, tier-only titles, and unresolved legacy titles.
- Confirm each destructive cleanup using current counts and explain the partial preview. Commit full-cleanup deletions together, and report optimization failure separately from deletion success.
- Surface database statistics/preview failures with retry and style failures as errors. Prevent earlier result timers from hiding newer messages.
- Invalidate old resolver searches on new input, clearing, closing and unmount. Surface search failures, share one busy state for each row, and use the app confirmation dialog for deletion.
- Refresh persistent library/search/navigation caches after duplicate merges.
- Show a successful whole-library status only when both libraries have succeeded. Freeze the movie release country for each sync run.
- Validate mocks and memory-database regressions, then commit the fixes checkpoint.

## Stage 2: approved improvements

- Data tabs: Overview, Sync, Backups, Archive, Maintenance. Preserve legacy calls to Duplicates/Cleanup by mapping them into Maintenance sections.
- Overview: compact library counts and links to failed syncs, unmatched shows, and last backup. Move database size/optimization to advanced Maintenance and keep missing air dates contextual.
- Sync: skipped-title reasons, last five runs, full-run results retained alongside retries, stop after current title, collapsed explanation of sync behavior.
- Backups: dedicated Create/Restore workflow, last successful backup, clear scope/preview and restart action. Opt-in local automatic backups with five recent managed files by default. Optional portable non-secret settings; preserve old backup compatibility.
- Archive: search/type filter, year and tracking status, open details, and Restore progress.
- Maintenance: duplicate comparisons with years/TMDB links and clearer merge actions; cleanup and unmatched-show repair; collapsed advanced/development tools.

## Ownership

- Sync agent: library sync Rust/store/UI, per-run movie country, sidebar status, associated tests.
- Maintenance agent: native cleanup predicates/transactions, resolver/store, Data store cache/error handling, associated tests and duplicate metadata.
- Backup agent: backup format/validation/automatic local backup service, Backups UI/store, portable settings and associated tests.
- Root: Data shell/Overview/Maintenance UI and confirmation/error presentation, Archive UI, native registration, integration, visual QA, changelog, final commits.

## Validation

- Full `npm run check`: Svelte diagnostics, all frontend tests, offline locked Rust tests against disposable data, production frontend build.
- Isolated real-app preview at 1920×1080 and 3840×2160, including larger app text.
- Check cancellation and failure paths without touching installed data. Automatic-backup tests use disposable directories and verify unrelated files are never removed.
- Review backup compatibility/security scope, cleanup rollback/personal-data preservation, old/new sync reports, navigation and cached displays.

## Progress

- Research complete; branch clean at task start. Existing dev identity and database selection verified from source.
- Stage 1 complete: full checks passed with 0 Svelte errors/warnings, 55 frontend tests, 103 Rust tests (1 live Wikipedia test ignored), and production build.
- Cleanup memory tests cover personal/legacy data preservation, matched counts/previews/deletion, rollback when the second deletion fails, capped previews and explicit statistics errors. Resolver/store tests cover stale replies, row action locking, cancelled deletion, tab switching, and saved-but-refresh-failed feedback.
- Isolated preview verified the full-cleanup confirmation, cancellation, protected-data explanation, and preview counts. No installed database was opened.
- Stage 1 saved and pushed as `f36cb12` before implementing the enhancements.
- Stage 2 complete: compact Overview/Sync/Backups/Archive/Maintenance navigation, separate maintenance subsections, full-run/retry summaries, five-run history, skipped reasons and stop-after-title, archive filtering/details/restore feedback, and optional automatic local backups.
- Backup files keep the existing v3 format unless portable preferences are selected, which uses v4. v1/v2/v3 remain supported; only explicitly allowed non-secret preferences transfer. Older restores preserve current settings. Portable restores replace the supplied racing choices while retaining local feed URLs.
- Restoration drains pending preference writes before replacing data. Navigation and remembered settings pause during restore and until restart, with a persistent Restart now action. Library and portable settings changes share one transaction.
- Automatic backups default to Off, settings excluded, five retained files. Only validated managed files for the current profile are pruned, after successful creation. Manual files, unrelated files, symlinks and other profiles remain untouched; the just-written backup survives clock changes. Failures preserve the last successful status.
- Final full checks: **0 Svelte errors/warnings, 75 frontend tests, 119 Rust tests passed (1 live Wikipedia test ignored), and successful production frontend build**. Test targets were memory/disposable databases and disposable folders; no installed database was opened.
- Regression coverage includes cleanup preparation failures, legacy navigation links, duplicate-summary error isolation, persistent archive/calendar refresh and retry, full report/history retention, stopping between libraries, schedule readback failures, portable-settings validation and atomic rollback, restore preference draining, and automatic-backup file isolation/retention.
- Visual QA uses the real app with isolated in-memory IPC fixtures at 1920×1080 and 3840×2160, including 150% app text. All five tabs remain within the page width. Merge and cleanup cancellations, archive year search/details/restore, skipped titles, history, successful retry with retained full counts, and stopping progress were exercised.
- A clean compiled fixture build verifies restore scope/confirmation, the persistent Restart now action, disabled Data navigation, inert app navigation, and blocked Ctrl K until restart. The long-running Vite preview retained different timestamped copies of the new backup store after hot reloads; the compiled build shares one store. Restart development processes before testing this stage.
- Proof images: `plans/focus-redesign-screenshots/data-backups-1080p.jpg`, `data-backups-4k.jpg`, `data-maintenance-1080p-large-text.jpg`, and `data-restore-restart-1080p.jpg`.
- Versions and released migrations remain unchanged. Main and the pre-Focus recovery tag/bundle remain intact. Native file dialogs, installed OS behavior and live TMDB calls are outside the isolated preview; their command behavior is covered by mocks and disposable native tests.
