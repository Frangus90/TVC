# Racing in automatic sync

## Research

TVC already converts UTC using the offset for each session's date. The default MotoGP feed [moved the Qatar sessions one hour earlier on 29.10.2026](https://github.com/nixxo/nixxo.github.io/commit/337965035f16a085072d0b6de87c5adbd82f6e3c). Cached schedules kept the old times until refreshed. No timezone adjustment is needed.

## Implementation plan

1. Add Racing to the existing sync scheduler, shared lock, reports, retry, stop and history. Sync only followed series with the existing validated feed refresh. Off remains the default; offer the same daily, weekly and monthly intervals. Sync All also includes Racing.
2. Keep Data → Sync as the place to change schedules and review failures. Route both existing full Racing refresh buttons through the shared command and keep individual feed refresh available. Reload the open calendar and day details after refresh; rebuild racing reminders after changes to saved sessions.
3. Verify due and disabled schedules, stale-to-corrected session times, failed-feed preservation and the frontend completion path using disposable fixtures. Check the compiled UI at 1080p and 4K and run `npm run check`.

No release database access, migration changes, dependency updates or app-version bumps. Work stays on `codex/0.15.2`.

## Validation

- `npm run check` passed: zero Svelte errors or warnings, 117 frontend tests, 151 Rust tests and the frontend build. The existing live Wikipedia test remains ignored.
- In-memory native tests cover Off by default, no due run without a followed series, empty and partially failed runs, corrected UTC start times, cached sessions preserved on failure, retry and acceptance history, and stopping after the current series.
- Store tests verify the app-wide completion event refreshes the calendar, the full Refresh button uses the shared command, and individual refresh still targets one feed.
- The compiled fixture showed the Racing schedule alongside TV and movies at 1920×1080 and 3840×2160 without horizontal overflow. Changing Racing to Every day left the movie and TV schedules intact. The Qatar sprint changed from 18:00 to 17:00 in the calendar and an already open day detail after refresh. The preview had no browser warnings or errors.
- Checks used mocked or in-memory data. Racing schedules and downloaded feeds are device-local; no installed database was accessed.
