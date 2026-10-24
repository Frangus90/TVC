import { showError } from "./toast.svelte";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { logger } from "../utils/logger";
import { defaultQuietHours, isWithinQuietHours, type QuietHoursSettings } from "../utils/quietHours";
import { playNotificationSound } from "../utils/notificationSound";

export interface AppNotification {
  id: number;
  type: "racing" | "plex" | "premiere" | "update" | "system";
  title: string;
  body: string;
  icon: string | null;
  reference_id: string | null;
  reference_type: string | null;
  read: boolean;
  dismissed: boolean;
  created_at: string;
  expires_at: string | null;
}

export interface NotificationSettings extends QuietHoursSettings {
  enabled: boolean;
  sound_enabled: boolean;
  sound_volume: number;
  sound_choice: string;
  popup_position: string;
  popup_duration: number;
  max_visible: number;
  os_fallback: boolean;
  tray_notifications: boolean;
  racing_enabled: boolean;
  plex_enabled: boolean;
  premiere_enabled: boolean;
  update_enabled: boolean;
  system_enabled: boolean;
  auto_clear_after_days: number;
}

// State
let notifications = $state<AppNotification[]>([]);
let unreadCount = $state(0);
let settings = $state<NotificationSettings | null>(null);
let activePopups = $state<AppNotification[]>([]);
let notificationCenterOpen = $state(false);
let settingsLoading = $state(false);
let settingsSaving = $state(false);
let settingsLoadError = $state<string | null>(null);
let settingsSaveError = $state<string | null>(null);
let settingsLoadPromise: Promise<void> | null = null;
let settingsSaveQueue: Promise<void> = Promise.resolve();
let pendingSettingsSaves = 0;
let settingsRevision = 0;
let confirmedAutoClearDays = 0;
let notificationsRequest = 0;
let notificationsRevision = 0;
let unreadRequest = 0;
let unreadRevision = 0;
let cleanupPromise: Promise<boolean> | null = null;
let cleanupErrorShown = false;

export function isNotificationSettingsLoading() { return settingsLoading; }
export function waitForNotificationSettingsSaves(): Promise<void> { return settingsSaveQueue; }
export function isNotificationSettingsSaving() { return settingsSaving; }
export function getNotificationSettingsLoadError() { return settingsLoadError; }
export function getNotificationSettingsSaveError() { return settingsSaveError; }

let popupTimers = new Map<number, ReturnType<typeof setTimeout>>();

// Getters
export function getNotifications(): AppNotification[] {
  return notifications;
}

export function getUnreadCount(): number {
  return unreadCount;
}

export function getNotificationSettings(): NotificationSettings | null {
  return settings;
}

export function getActivePopups(): AppNotification[] {
  return activePopups;
}

export function isNotificationCenterOpen(): boolean {
  return notificationCenterOpen;
}

// Notification center
export function openNotificationCenter() {
  notificationCenterOpen = true;
}

export function closeNotificationCenter() {
  notificationCenterOpen = false;
}

export function toggleNotificationCenter() {
  notificationCenterOpen = !notificationCenterOpen;
}

// Load notifications from backend
export async function loadNotifications(
  limit: number = 50,
  unreadOnly: boolean = false,
): Promise<void> {
  const request = ++notificationsRequest;
  const revision = notificationsRevision;
  try {
    const result = await invoke<AppNotification[]>("get_notifications", {
      limit,
      offset: 0,
      unreadOnly,
    });
    if (request !== notificationsRequest) return;
    if (revision !== notificationsRevision) {
      await loadNotifications(limit, unreadOnly);
      return;
    }
    notifications = result;
  } catch (e) {
    logger.error("[Notifications] Failed to load notifications", e);
  }
}

// Load unread count
export async function loadUnreadCount() {
  const request = ++unreadRequest;
  const revision = unreadRevision;
  try {
    const count = await invoke<number>("get_unread_notification_count");
    if (request !== unreadRequest) return;
    if (revision !== unreadRevision) {
      await loadUnreadCount();
      return;
    }
    unreadCount = count;
  } catch (e) {
    logger.error("[Notifications] Failed to load unread count", e);
  }
}

// Load settings, sharing in-flight reads so a retry cannot replace a newer response.
export function loadNotificationSettings(): Promise<void> {
  if (settingsLoadPromise) return settingsLoadPromise;
  settingsLoading = true;
  settingsLoadError = null;
  const revision = settingsRevision;
  settingsLoadPromise = invoke<NotificationSettings>("get_notification_settings")
    .then(result => {
      if (revision === settingsRevision) {
        settings = { ...defaultQuietHours, ...result, auto_clear_after_days: result.auto_clear_after_days ?? 0 };
        confirmedAutoClearDays = settings.auto_clear_after_days;
      }
    })
    .catch(error => {
      logger.error("[Notifications] Failed to load settings", error);
      settingsLoadError = `Could not load notification settings: ${error}`;
    })
    .finally(() => { settingsLoading = false; settingsLoadPromise = null; });
  return settingsLoadPromise;
}

// Apply changes immediately, then save full snapshots in order.
export async function updateNotificationSettings(updates: Partial<NotificationSettings>): Promise<boolean> {
  if (!settings) return false;
  settings = { ...settings, ...updates };
  if (!settings.enabled || isWithinQuietHours(settings)) clearPopups();
  const snapshot = { ...settings };
  const revision = ++settingsRevision;
  pendingSettingsSaves++;
  settingsSaving = true;
  settingsSaveError = null;
  const operation = settingsSaveQueue.then(() => invoke("update_notification_settings", { settings: snapshot }));
  settingsSaveQueue = operation.then(() => {}, () => {}); // Failure is surfaced below; allow a later save/retry.
  try {
    await operation;
    confirmedAutoClearDays = snapshot.auto_clear_after_days;
    if (revision === settingsRevision) {
      settingsSaveError = null;
      if (confirmedAutoClearDays > 0) await clearOldNotifications();
    }
    return true;
  } catch (error) {
    logger.error("[Notifications] Failed to update settings", error);
    if (revision === settingsRevision) settingsSaveError = `Notification changes were not saved: ${error}`;
    return false;
  } finally {
    settingsSaving = --pendingSettingsSaves > 0;
  }
}

export function retryNotificationSettingsSave() { return updateNotificationSettings({}); }

// Native cleanup uses the saved policy; optimistic or failed preference changes
// cannot select a more aggressive retention period for this operation.
export function clearOldNotifications(): Promise<boolean> {
  if (cleanupPromise) return cleanupPromise;
  cleanupPromise = invoke<number[]>("clear_old_notifications")
    .then(async ids => {
      if (ids.length) {
        const cleared = new Set(ids);
        ++notificationsRevision;
        notifications = notifications.filter(notification => !cleared.has(notification.id));
        for (const id of ids) removePopup(id);
        ++unreadRevision;
      }
      await loadUnreadCount();
      cleanupErrorShown = false;
      return true;
    })
    .catch(error => {
      logger.error("[Notifications] Automatic cleanup failed", error);
      if (!cleanupErrorShown) showError(`Could not automatically clear notification history: ${error}`);
      cleanupErrorShown = true;
      return false;
    })
    .finally(() => { cleanupPromise = null; });
  return cleanupPromise;
}

// Mark a notification as read
export async function markRead(id: number) {
  try {
    await invoke("mark_notification_read", { id });
    ++notificationsRevision;
    const notification = notifications.find(notification => notification.id === id);
    notifications = notifications.map((n) =>
      n.id === id ? { ...n, read: true } : n,
    );
    ++unreadRevision;
    if (notification && !notification.read) unreadCount = Math.max(0, unreadCount - 1);
  } catch (e) {
    logger.error("[Notifications] Failed to mark read", e);
  }
}

// Mark all as read
export async function markAllRead() {
  try {
    await invoke("mark_all_notifications_read");
    ++notificationsRevision;
    ++unreadRevision;
    notifications = notifications.map((n) => ({ ...n, read: true }));
    unreadCount = 0;
  } catch (e) {
    logger.error("[Notifications] Failed to mark all read", e);
  }
}

// Dismiss a notification
export async function dismissNotification(id: number) {
  try {
    await invoke("dismiss_notification", { id });
    ++notificationsRevision;
    ++unreadRevision;
    const dismissed = notifications.find((n) => n.id === id);
    notifications = notifications.filter((n) => n.id !== id);
    if (dismissed && !dismissed.read) {
      unreadCount = Math.max(0, unreadCount - 1);
    }
  } catch (e) {
    logger.error("[Notifications] Failed to dismiss notification", e);
  }
}

// Dismiss all
export async function dismissAllNotifications() {
  try {
    await invoke("dismiss_all_notifications");
    ++notificationsRevision;
    ++unreadRevision;
    unreadCount = 0;
    notifications = [];
  } catch (e) {
    logger.error("[Notifications] Failed to dismiss all", e);
  }
}

// Test the current saved notification preferences.
export async function testNotification(
  notificationType?: string,
): Promise<boolean> {
  try {
    await settingsSaveQueue;
    if (settingsSaveError) return false;
    await invoke("test_in_app_notification", {
      notificationType,
    });
    return true;
  } catch (e) {
    logger.error("[Notifications] Failed to send test notification", e);
    showError(`Could not send a test notification: ${e}`);
    return false;
  }
}

// Popup management
function addPopup(notification: AppNotification) {
  const max = settings?.max_visible ?? 3;
  activePopups = [notification, ...activePopups].slice(0, max);

  const duration = settings?.popup_duration ?? 8000;
  if (duration > 0) {
    const timer = setTimeout(() => removePopup(notification.id), duration);
    popupTimers.set(notification.id, timer);
  }
}

export function removePopup(id: number) {
  const timer = popupTimers.get(id);
  if (timer) {
    clearTimeout(timer);
    popupTimers.delete(id);
  }
  activePopups = activePopups.filter((n) => n.id !== id);
}

function clearPopups() {
  for (const id of popupTimers.keys()) removePopup(id);
  activePopups = [];
}

// Keep history/counts during quiet hours; only delivery is suppressed.
export function setupNotificationListener(): () => void {
  let unlisten: UnlistenFn | undefined;
  let disposed = false;
  const deliver = async (notification: AppNotification) => {
    if (!settings) await loadNotificationSettings();
    if (disposed || !settings?.enabled || isWithinQuietHours(settings)) return;
    addPopup(notification);
    if (settings.sound_enabled) playNotificationSound(settings.sound_choice, settings.sound_volume);
  };
  listen<AppNotification>("notification", (event) => {
    const notif = event.payload;
    ++notificationsRevision;
    notifications = [notif, ...notifications];
    ++unreadRevision;
    unreadCount++;
    void deliver(notif);
  }).then((fn) => {
    if (disposed) fn(); else unlisten = fn;
  }).catch(error => showError(`Could not listen for notifications: ${error}`));
  // Remove persistent popups when quiet hours begin, even without a new event.
  const timer = setInterval(() => { if (settings && isWithinQuietHours(settings)) clearPopups(); }, 1000);
  const cleanup = () => {
    if (!disposed && confirmedAutoClearDays > 0 && settings?.auto_clear_after_days
      && !settingsSaving && !settingsSaveError) void clearOldNotifications();
  };
  void loadNotificationSettings().then(cleanup);
  const cleanupTimer = setInterval(cleanup, 60_000);
  return () => { disposed = true; clearInterval(timer); clearInterval(cleanupTimer); unlisten?.(); clearPopups(); };
}
