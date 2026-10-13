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
- Stage 2 designs prepared while fixes were implemented; enhancements begin after saving this checkpoint.
