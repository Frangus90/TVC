import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import vm from 'node:vm';
import ts from 'typescript';

function load(path, dependencies, globals = {}) {
  const exports = {};
  const code = ts.transpileModule(readFileSync(path, 'utf8'), {
    compilerOptions: { target: ts.ScriptTarget.ES2021, module: ts.ModuleKind.CommonJS },
  }).outputText;
  vm.runInNewContext(code, {
    exports, $state: value => value, console, ...globals,
    require: name => {
      if (!(name in dependencies)) throw new Error(`Missing test dependency: ${name}`);
      return dependencies[name];
    },
  }, { filename: path });
  return exports;
}

const quietHours = load('src/lib/utils/quietHours.ts', {});
const baseline = {
  enabled: true, sound_enabled: false, sound_volume: 50, sound_choice: 'chime',
  popup_position: 'top-right', popup_duration: 8000, max_visible: 3,
  os_fallback: false, tray_notifications: true, racing_enabled: true,
  plex_enabled: true, premiere_enabled: true, update_enabled: true, system_enabled: true,
};
const settle = async () => { for (let i = 0; i < 32; i++) await Promise.resolve(); };
function deferred() {
  let resolve, reject;
  const promise = new Promise((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}
function notification(id) {
  return {
    id, type: 'system', title: `Notification ${id}`, body: '', icon: null,
    reference_id: null, reference_type: null, read: false, dismissed: false,
    created_at: '2026-10-24T10:00:00Z', expires_at: null,
  };
}
function retention(settings = baseline) {
  const calls = [], errors = [], intervals = new Map(), timeouts = new Map();
  let nextTimer = 0, receive, unlistened = 0;
  const handlers = {
    get_notification_settings: () => ({ ...settings }),
    get_notifications: () => [],
    get_unread_notification_count: () => 0,
    update_notification_settings: () => undefined,
    clear_old_notifications: () => [],
  };
  const api = load('src/lib/stores/notifications.svelte.ts', {
    '@tauri-apps/api/core': {
      invoke(command, args) {
        calls.push({ command, args });
        if (!handlers[command]) throw new Error(`Unexpected notification command: ${command}`);
        try { return Promise.resolve(handlers[command](args)); }
        catch (error) { return Promise.reject(error); }
      },
    },
    '@tauri-apps/api/event': { listen: async (_event, handler) => {
      receive = handler;
      return () => { unlistened++; };
    } },
    '../utils/logger': { logger: { error() {}, warn() {}, debug() {} } },
    './toast.svelte': { showError: message => errors.push(message) },
    '../utils/quietHours': quietHours,
    '../utils/notificationSound': { playNotificationSound() {} },
  }, {
    setTimeout: (callback, ms) => { const id = ++nextTimer; timeouts.set(id, { callback, ms }); return id; },
    clearTimeout: id => timeouts.delete(id),
    setInterval: (callback, ms) => { const id = ++nextTimer; intervals.set(id, { callback, ms }); return id; },
    clearInterval: id => intervals.delete(id),
  });
  return {
    api, calls, errors, intervals, timeouts, handlers,
    emit: value => receive({ payload: value }),
    runIntervals: ms => { for (const timer of [...intervals.values()]) if (timer.ms === ms) timer.callback(); },
    count: command => calls.filter(call => call.command === command).length,
    getUnlistened: () => unlistened,
  };
}

test('older notification preferences keep their history until auto-clear is enabled', async () => {
  const { api } = retention();
  await api.loadNotificationSettings();
  assert.equal(api.getNotificationSettings().auto_clear_after_days, 0);
});

test('notification cleanup shares a pending run and removes only the IDs cleared by the backend', async () => {
  const h = retention({ ...baseline, auto_clear_after_days: 1 });
  await h.api.loadNotificationSettings();
  const stop = h.api.setupNotificationListener();
  try {
    await settle();
    h.emit(notification(1));
    await settle();
    h.handlers.get_notifications = () => [notification(1), notification(2)];
    await h.api.loadNotifications();
    const pending = deferred();
    h.handlers.clear_old_notifications = () => pending.promise;
    h.handlers.get_unread_notification_count = () => 9;
    const before = h.count('clear_old_notifications');
    const first = h.api.clearOldNotifications();
    const second = h.api.clearOldNotifications();
    await settle();
    assert.equal(h.count('clear_old_notifications'), before + 1);
    assert.equal(h.calls.findLast(call => call.command === 'clear_old_notifications').args, undefined,
      'the backend must use the saved policy, not UI-supplied days');
    h.emit(notification(3));
    await settle();
    pending.resolve([1]);
    assert.equal(await first, true);
    assert.equal(await second, true);
    assert.deepEqual(Array.from(h.api.getNotifications(), item => item.id), [3, 2]);
    assert.deepEqual(Array.from(h.api.getActivePopups(), item => item.id), [3]);
    assert.equal(h.timeouts.size, 1, 'the removed popup cannot leave its dismissal timer running');
    assert.equal(h.api.getUnreadCount(), 9, 'cleanup reloads the authoritative unread count');
  } finally { stop(); }
});

test('a first history request retries after cleanup so recent entries remain visible', async () => {
  const h = retention({ ...baseline, auto_clear_after_days: 7 });
  await h.api.loadNotificationSettings();
  const pending = deferred();
  let historyReads = 0;
  h.handlers.get_notifications = () => ++historyReads === 1 ? pending.promise : [notification(2)];
  const staleRead = h.api.loadNotifications();
  h.handlers.clear_old_notifications = () => [1];
  assert.equal(await h.api.clearOldNotifications(), true);
  pending.resolve([notification(1), notification(2)]);
  await staleRead;
  assert.equal(historyReads, 2, 'the initial history load must recover after cleanup');
  assert.deepEqual(Array.from(h.api.getNotifications(), item => item.id), [2]);
});

test('a notification arriving during history loading remains visible after the older read resolves', async () => {
  const h = retention();
  await h.api.loadNotificationSettings();
  const stop = h.api.setupNotificationListener();
  try {
    await settle();
    const pending = deferred();
    let historyReads = 0;
    h.handlers.get_notifications = () => ++historyReads === 1 ? pending.promise : [notification(3), notification(2)];
    const staleRead = h.api.loadNotifications(15, true);
    h.emit(notification(3));
    await settle();
    pending.resolve([notification(2)]);
    await staleRead;
    assert.equal(historyReads, 2);
    assert.deepEqual(Array.from(h.api.getNotifications(), item => item.id), [3, 2]);
    const latestRead = h.calls.findLast(call => call.command === 'get_notifications');
    assert.equal(latestRead.args.limit, 15, 'the retry keeps the requested page size');
    assert.equal(latestRead.args.unreadOnly, true, 'the retry keeps the selected history filter');
  } finally { stop(); }
});

test('an unread-count response cannot erase a notification delivered during cleanup', async () => {
  const h = retention({ ...baseline, auto_clear_after_days: 1 });
  await h.api.loadNotificationSettings();
  const stop = h.api.setupNotificationListener();
  try {
    await settle();
    h.handlers.get_notifications = () => [notification(1)];
    await h.api.loadNotifications();
    const countRead = deferred();
    let countReads = 0;
    h.handlers.get_unread_notification_count = () => ++countReads === 1 ? countRead.promise : 1;
    h.handlers.clear_old_notifications = () => [1];
    const cleanup = h.api.clearOldNotifications();
    await settle();
    assert.equal(countReads, 1);
    h.emit(notification(2));
    await settle();
    countRead.resolve(0);
    assert.equal(await cleanup, true);
    await settle();
    assert.equal(h.api.getUnreadCount(), 1);
    assert.deepEqual(Array.from(h.api.getNotifications(), item => item.id), [2]);
  } finally { stop(); }
});

test('notification cleanup reports repeated failures once until a run succeeds', async () => {
  const h = retention({ ...baseline, auto_clear_after_days: 30 });
  await h.api.loadNotificationSettings();
  h.handlers.clear_old_notifications = () => { throw new Error('database unavailable'); };
  assert.equal(await h.api.clearOldNotifications(), false);
  assert.equal(await h.api.clearOldNotifications(), false);
  assert.equal(h.errors.length, 1);
  h.handlers.clear_old_notifications = () => [];
  assert.equal(await h.api.clearOldNotifications(), true);
  h.handlers.clear_old_notifications = () => { throw new Error('database unavailable again'); };
  assert.equal(await h.api.clearOldNotifications(), false);
  assert.equal(h.errors.length, 2, 'a new failure after recovery is visible');
});

test('saved auto-clear runs at startup and each minute even when notification delivery is disabled', async () => {
  const h = retention({ ...baseline, enabled: false, auto_clear_after_days: 7 });
  const settingsRead = deferred();
  h.handlers.get_notification_settings = () => settingsRead.promise;
  const stop = h.api.setupNotificationListener();
  try {
    await settle();
    assert.equal(h.count('clear_old_notifications'), 0, 'startup waits for preferences');
    settingsRead.resolve({ ...baseline, enabled: false, auto_clear_after_days: 7 });
    await settle();
    assert.equal(h.count('clear_old_notifications'), 1);
    assert.equal([...h.intervals.values()].filter(timer => timer.ms === 60000).length, 1);
    h.runIntervals(60000);
    await settle();
    assert.equal(h.count('clear_old_notifications'), 2);
  } finally { stop(); }
  assert.equal(h.intervals.size, 0);
  assert.equal(h.getUnlistened(), 1);
  h.runIntervals(60000);
  await settle();
  assert.equal(h.count('clear_old_notifications'), 2);
});

test('disposing notification delivery while settings load prevents a late startup cleanup', async () => {
  const h = retention();
  const settingsRead = deferred();
  h.handlers.get_notification_settings = () => settingsRead.promise;
  const stop = h.api.setupNotificationListener();
  await settle();
  stop();
  settingsRead.resolve({ ...baseline, auto_clear_after_days: 1 });
  await settle();
  assert.equal(h.count('clear_old_notifications'), 0);
  assert.equal(h.intervals.size, 0);
  assert.equal(h.getUnlistened(), 1);
});

test('failed auto-clear preference writes leave history untouched and retry applies the saved choice', async () => {
  const h = retention();
  await h.api.loadNotificationSettings();
  const write = deferred();
  h.handlers.update_notification_settings = () => write.promise;
  const save = h.api.updateNotificationSettings({ auto_clear_after_days: 1 });
  await settle();
  assert.equal(h.count('clear_old_notifications'), 0);
  write.reject(new Error('write failed'));
  assert.equal(await save, false);
  assert.equal(h.count('clear_old_notifications'), 0);
  h.handlers.update_notification_settings = () => undefined;
  assert.equal(await h.api.retryNotificationSettingsSave(), true);
  await settle();
  assert.equal(h.calls.findLast(call => call.command === 'update_notification_settings').args.settings.auto_clear_after_days, 1);
  assert.equal(h.count('clear_old_notifications'), 1);
});

test('a stale positive auto-clear save cannot delete history after Keep forever is selected', async () => {
  const h = retention();
  await h.api.loadNotificationSettings();
  const writes = [];
  h.handlers.update_notification_settings = () => {
    const pending = deferred();
    writes.push(pending);
    return pending.promise;
  };
  const first = h.api.updateNotificationSettings({ auto_clear_after_days: 30 });
  const latest = h.api.updateNotificationSettings({ auto_clear_after_days: 0 });
  await settle();
  assert.equal(writes.length, 1);
  writes[0].resolve();
  assert.equal(await first, true);
  await settle();
  assert.equal(writes.length, 2);
  assert.equal(h.count('clear_old_notifications'), 0);
  writes[1].resolve();
  assert.equal(await latest, true);
  assert.equal(h.api.getNotificationSettings().auto_clear_after_days, 0);
  assert.equal(h.count('clear_old_notifications'), 0);
});

test('the minute timer cannot act on an optimistic auto-clear choice that has not been saved', async () => {
  const h = retention();
  await h.api.loadNotificationSettings();
  const stop = h.api.setupNotificationListener();
  try {
    await settle();
    const write = deferred();
    h.handlers.update_notification_settings = () => write.promise;
    const save = h.api.updateNotificationSettings({ auto_clear_after_days: 7 });
    await settle();
    h.runIntervals(60000);
    await settle();
    assert.equal(h.count('clear_old_notifications'), 0, 'an in-progress write is not a confirmed policy');
    write.reject(new Error('write failed'));
    assert.equal(await save, false);
    h.runIntervals(60000);
    await settle();
    assert.equal(h.count('clear_old_notifications'), 0, 'a failed write does not authorize cleanup');
    h.handlers.update_notification_settings = () => undefined;
    assert.equal(await h.api.retryNotificationSettingsSave(), true);
    await settle();
    assert.equal(h.count('clear_old_notifications'), 1);
    h.runIntervals(60000);
    await settle();
    assert.equal(h.count('clear_old_notifications'), 2);
  } finally { stop(); }
});
