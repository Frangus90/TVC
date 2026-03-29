import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { logger } from "../utils/logger";
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

export interface NotificationSettings {
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
}

// State
let notifications = $state<AppNotification[]>([]);
let unreadCount = $state(0);
let settings = $state<NotificationSettings | null>(null);
let activePopups = $state<AppNotification[]>([]);
let notificationCenterOpen = $state(false);
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
) {
  try {
    notifications = await invoke("get_notifications", {
      limit,
      offset: 0,
      unreadOnly,
    });
  } catch (e) {
    logger.error("[Notifications] Failed to load notifications", e);
  }
}

// Load unread count
export async function loadUnreadCount() {
  try {
    unreadCount = await invoke("get_unread_notification_count");
  } catch (e) {
    logger.error("[Notifications] Failed to load unread count", e);
  }
}

// Load settings
export async function loadNotificationSettings() {
  try {
    settings = await invoke("get_notification_settings");
  } catch (e) {
    logger.error("[Notifications] Failed to load settings", e);
  }
}

// Update settings
export async function updateNotificationSettings(
  updates: Partial<NotificationSettings>,
) {
  if (!settings) return;

  const updated = { ...settings, ...updates };

  try {
    await invoke("update_notification_settings", { settings: updated });
    settings = updated;
  } catch (e) {
    logger.error("[Notifications] Failed to update settings", e);
  }
}

// Mark a notification as read
export async function markRead(id: number) {
  try {
    await invoke("mark_notification_read", { id });
    notifications = notifications.map((n) =>
      n.id === id ? { ...n, read: true } : n,
    );
    unreadCount = Math.max(0, unreadCount - 1);
  } catch (e) {
    logger.error("[Notifications] Failed to mark read", e);
  }
}

// Mark all as read
export async function markAllRead() {
  try {
    await invoke("mark_all_notifications_read");
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
    unreadCount = 0;
    notifications = [];
  } catch (e) {
    logger.error("[Notifications] Failed to dismiss all", e);
  }
}

// Test notification (dev only)
export async function testNotification(
  notificationType?: string,
): Promise<void> {
  try {
    await invoke("test_in_app_notification", {
      notificationType,
    });
  } catch (e) {
    logger.error("[Notifications] Failed to send test notification", e);
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

// Setup event listener — call from App.svelte onMount
export function setupNotificationListener(): () => void {
  let unlisten: UnlistenFn | undefined;

  listen<AppNotification>("notification", (event) => {
    const notif = event.payload;

    // Add to notifications list
    notifications = [notif, ...notifications];
    unreadCount++;

    // Show popup
    addPopup(notif);

    // Play sound
    if (settings?.sound_enabled) {
      playNotificationSound(
        settings.sound_choice,
        settings.sound_volume,
      );
    }
  }).then((fn) => {
    unlisten = fn;
  });

  return () => unlisten?.();
}
