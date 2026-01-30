import { format, parseISO, isToday, isTomorrow } from "date-fns";

/**
 * App-wide date/time formatting defaults:
 * - 24-hour clock
 * - PC's local timezone
 * - Date format D.M.YYYY (e.g. 29.01.2026)
 */

const DB_UTC_REGEX = /^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}$/;
const DATE_ONLY_REGEX = /^\d{4}-\d{2}-\d{2}$/;

function toDate(value: Date | string): Date {
  if (value instanceof Date) return value;
  const s = value.trim();
  if (!s) return new Date(NaN);
  if (DB_UTC_REGEX.test(s)) {
    const iso = s.replace(" ", "T") + "Z";
    return new Date(iso);
  }
  if (DATE_ONLY_REGEX.test(s)) {
    const [y, m, d] = s.split("-").map(Number);
    return new Date(y, m - 1, d);
  }
  return parseISO(s);
}

/**
 * Parse date/time string (DB UTC "YYYY-MM-DD HH:MM:SS", ISO, or date-only) to Date.
 * Use for consistent UTC/local handling before formatting or relative-time logic.
 */
export function parseDateTime(value: Date | string): Date {
  return toDate(value);
}

/**
 * Format as date only: D.M.YYYY (e.g. 29.01.2026). Uses local timezone.
 */
export function formatDate(value: Date | string): string {
  const d = toDate(value);
  if (Number.isNaN(d.getTime())) return "";
  return format(d, "dd.MM.yyyy");
}

/**
 * Format as date and time: D.M.YYYY HH:mm, 24-hour, local timezone.
 */
export function formatDateTime(value: Date | string): string {
  const d = toDate(value);
  if (Number.isNaN(d.getTime())) return "";
  return format(d, "dd.MM.yyyy HH:mm");
}

/**
 * Format as time only: HH:mm, 24-hour, local timezone.
 */
export function formatTime(value: Date | string): string {
  const d = toDate(value);
  if (Number.isNaN(d.getTime())) return "";
  return format(d, "HH:mm");
}

/**
 * Long date with weekday, e.g. "Monday, 29.01.2026". Uses local timezone.
 */
export function formatLongDate(value: Date | string): string {
  const d = toDate(value);
  if (Number.isNaN(d.getTime())) return "";
  return format(d, "EEEE, dd.MM.yyyy");
}

/**
 * For list headers: "Today", "Tomorrow", or D.M.YYYY. Uses local timezone.
 */
export function formatDateHeader(value: Date | string): string {
  const d = toDate(value);
  if (Number.isNaN(d.getTime())) return "";
  if (isToday(d)) return "Today";
  if (isTomorrow(d)) return "Tomorrow";
  return format(d, "dd.MM.yyyy");
}

/**
 * Calendar week range: "d.M. - d.M.yyyy" (e.g. 29.01. - 04.02.2026). Uses local timezone.
 */
export function formatWeekRange(start: Date | string, end: Date | string): string {
  const s = toDate(start);
  const e = toDate(end);
  if (Number.isNaN(s.getTime()) || Number.isNaN(e.getTime())) return "";
  return `${format(s, "dd.MM")} - ${format(e, "dd.MM.yyyy")}`;
}

/**
 * Month + year: "MM.yyyy" (e.g. 01.2026). Uses local timezone.
 */
export function formatMonthYear(value: Date | string): string {
  const d = toDate(value);
  if (Number.isNaN(d.getTime())) return "";
  return format(d, "MM.yyyy");
}

/**
 * Month name + year: "January 2026". Uses local timezone.
 */
export function formatMonthYearLong(value: Date | string): string {
  const d = toDate(value);
  if (Number.isNaN(d.getTime())) return "";
  return format(d, "MMMM yyyy");
}

/**
 * ISO date key for grouping / API: yyyy-MM-dd. Uses local date.
 */
export function formatDateKey(value: Date | string): string {
  const d = toDate(value);
  if (Number.isNaN(d.getTime())) return "";
  return format(d, "yyyy-MM-dd");
}
