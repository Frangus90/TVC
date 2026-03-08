import { startOfDay } from "date-fns";

/** Reactive "current time" that ticks every 60 seconds. */
let now = $state(new Date());
let intervalId: ReturnType<typeof setInterval> | null = null;

/** Callbacks invoked when the calendar date rolls over to a new day. */
const dayChangeListeners: Array<() => void> = [];

export function getNow(): Date {
  return now;
}

/** Register a callback for when the date changes (midnight crossover). */
export function onDayChange(cb: () => void) {
  dayChangeListeners.push(cb);
  return () => {
    const idx = dayChangeListeners.indexOf(cb);
    if (idx >= 0) dayChangeListeners.splice(idx, 1);
  };
}

/** Start the 60-second tick. Call once from App.svelte onMount. */
export function startClock() {
  if (intervalId) return;

  intervalId = setInterval(() => {
    const prev = now;
    now = new Date();

    // Detect day rollover
    if (startOfDay(now).getTime() !== startOfDay(prev).getTime()) {
      for (const cb of dayChangeListeners) {
        cb();
      }
    }
  }, 60_000);
}

/** Stop the tick. Call from App.svelte cleanup. */
export function stopClock() {
  if (intervalId) {
    clearInterval(intervalId);
    intervalId = null;
  }
}
