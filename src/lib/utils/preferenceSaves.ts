import { waitForGeneralPreferenceSaves } from "../stores/general.svelte";
import { waitForThemeSaves } from "../stores/theme.svelte";
import { waitForNotificationSettingsSaves } from "../stores/notifications.svelte";

// Restore first blocks navigation, then finishes writes already queued by Settings
// or remembered navigation. They must complete before portable settings replace them.
export async function waitForPendingPreferenceSaves(): Promise<void> {
  await Promise.all([
    waitForGeneralPreferenceSaves(),
    waitForThemeSaves(),
    waitForNotificationSettingsSaves(),
  ]);
}
