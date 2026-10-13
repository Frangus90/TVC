# Accepting sync failures

## Request and scope

Let the owner mark a failed title sync as acceptable so it no longer stays in the attention list. Keep this on `codex/focus-redesign` for the next release. Never access the installed release database or sidecars; use memory or disposable test data. Do not release, change app version files or modify released migrations.

## Behaviour

- **Data → Sync → Accept failure** applies to a single title in the current completed run.
- Accepted failures move into a collapsed list with **Undo**. Sidebar and Overview stop counting them as needing attention.
- Errors and actual failed totals remain in the report and recent history. Accepting does not claim that a title synced successfully or change the last successful full-sync date.
- **Retry failed** retries only failures that have not been accepted. Once another run starts, acceptance stays in the previous run's history; Undo is available before starting another run.
- Manual and automatic full syncs continue to include every eligible title. Any new failure starts unaccepted.
- A stale report, running sync/restore or failed save must not silently change another run or hide an error.

## Plan

1. Persist acceptance in the existing report JSON and update matching current/history/full reports together; verify backwards compatibility and stale-action guards with disposable SQLite tests.
2. Add Accept/Undo controls and separate accepted results, update shared attention counts and refresh notifications; verify async save/read races and visible errors.
3. Update the next-release changelog, check the compiled UI with isolated memory fixtures, and run project diagnostics, tests and build.

## Progress

Implementation complete for **0.15.1 Unreleased**. App version files remain at the owner's released **0.15.0**.

## Implementation

- The existing sync report JSON stores an optional `accepted` flag per failure; older reports treat missing flags as false. No migration is needed.
- Native updates hold the same lock as sync/restore and check the current library, run timestamp and failed title. Current/history/full-run reports update atomically, and failed persistence rolls back all changes.
- A dedicated status-change event refreshes attention counts without reloading title libraries. Sidebar discards old responses; Overview finishes older reads before loading the changed status.
- Accept/Undo shows saved feedback and keeps keyboard focus on the feedback when the clicked row moves. A failed save leaves the failure actionable and shows its error.

## Validation

- Full project check: **86 frontend tests passed; 124 Rust tests passed; 1 live Wikipedia test ignored; production frontend build passed**.
- Fixed a Svelte warning about the new feedback-element binding, then reran diagnostics (**0 errors, 0 warnings**), all **11 sync frontend regression tests**, and the production frontend build successfully.
- `cargo check --release --locked --offline` passed. The release configuration was only compiled; no native app or installer was launched.
- Rust tests used only `sqlite::memory:`. Coverage includes old JSON, persisted acceptance/Undo, unchanged errors/counts/success dates/schedules, library isolation, stale and unfinished results, sync/restore lock contention, transaction rollback, retry excluding accepted failures, and a new full run retrying both accepted and unaccepted titles.
- Frontend tests cover failed saves, delayed status responses, a newer run arriving during acceptance, pending-action protection, open-history refresh, and sidebar/Overview event races.
- Real compiled UI with isolated memory IPC fixtures: Accept and Undo clear/restore the reminder, preserve the visible failed total, update Overview/history, and keep keyboard focus. Retry works after Undo. Reopening the Sync page keeps the accepted state within the fixture session.
- Layout checked at **1920×1080**, **3840×2160** and **900×600** with no document horizontal overflow. Small windows stack the library cards and retain access to Undo through vertical scrolling.
- Independent backend and frontend reviews found no blockers. No installed database or sidecar was accessed; no released migration or app version file changed.
- Local visual proof: `focus-redesign-screenshots/sync-accepted-failure.jpg` (ignored test artifact). Preview console reported no errors or warnings.

## Limits

UI verification used memory fixtures; live TMDB requests and the native installed app were not exercised. Accept applies to a completed current run, including a stopped run's recorded title failures. It does not dismiss a library-level error or unfinished-run warning. Starting another run retains acceptance in recent history and ends availability of Undo for that older run.

The earlier Focus installer validation records are for their documented prior source revision. This follow-up has compile/test/build validation; the owner's normal release script will generate the next installers. No release, tag, merge or installer publication is part of this task.
