# Notification history auto-clear

## Scope

For 0.15.2 add **Settings → Notifications → Automatically clear history** with **Off**, **1 day**, **1 week** and **1 month**. A month is 30 days. Age is measured from creation time; both read and unread entries are cleared. Match existing Clear all behavior by dismissing history entries rather than deleting unrelated app data. Default Off preserves current behavior.

## Plan

1. Persist the selected period in existing generic settings with old preferences/backups defaulting to Off. Validate only 0/1/7/30 days; preserve released migrations and application versions.
2. Run cleanup after successful enabling/changing of the setting, on app startup and every minute while TVC is open. Use the saved native policy, skip failed/pending preference saves, remove only returned cleared IDs from history/popups and update the unread count. Stop the timer on app disposal.
3. Match the Focus settings rows, explain unread clearing and the 30-day month, cover boundary timestamps/save failures/concurrent events/backups with disposable tests, check the compiled UI at 1080p/4K and run project checks.

## Safeguards

Stay on `codex/0.15.2`. Never access the release database or sidecars. Use only mocked/in-memory/disposable data for validation; no real notification cleanup or preference change is run against an installed library. Do not run the release script, bump version files, merge main or publish a release.

## Validation

- `npm run check` passed: no Svelte errors or warnings, 114 frontend tests, 149 Rust tests and the frontend build. One existing live Wikipedia test remains intentionally ignored. Cargo reported a harmless cache hard-link fallback to copying files.
- Native in-memory tests cover Off/1/7/30-day policies, the exact cutoff, read and unread entries, offset and SQLite timestamps, missing/malformed/future dates, repeated cleanup, settings validation and transaction rollback. The saved policy and history are read within one transaction; no caller-supplied retention period can override it.
- Store tests cover startup and the one-minute timer even with alerts disabled, disposal, failed/pending saves, ordered updates, concurrent cleanup, popup removal and unread counts. History reads that overlap cleanup or new arrivals retry so they retain recent entries without restoring cleared ones.
- Old backups and settings default to Off. Backup round-trip tests preserve each allowed policy, reset old restored settings to Off and reject invalid values before restoring library data.
- The compiled Focus UI was checked with an in-memory browser fixture at 1920×1080 and 3840×2160: all four dropdown options work, controls remain readable and the page has no horizontal overflow. Exercising 30/7/1-day periods cleared the expected old entries and changed the unread badge from 4 to 3 to 3 to 2. There were no browser warnings or errors.
- Visual proof: `plans/focus-redesign-screenshots/notification-auto-clear-1080p.jpg` and `plans/focus-redesign-screenshots/notification-auto-clear-4k.jpg` (local ignored artifacts).
- Assumptions: one month is 30 elapsed days, age follows the computer clock and cleanup runs only while TVC is open. Unparseable creation dates are preserved. Clearing matches the existing soft-dismiss behavior; it does not purge stored rows.
- All verification used mocked or in-memory fixtures. No installed database was opened or changed, no released migration or app-version file changed, and no new network or credential access was added.
