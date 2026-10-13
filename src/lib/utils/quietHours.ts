export interface QuietHoursSettings {
  quiet_hours_enabled: boolean;
  quiet_hours_start: string;
  quiet_hours_end: string;
}

export const defaultQuietHours: QuietHoursSettings = {
  quiet_hours_enabled: false,
  quiet_hours_start: "22:00",
  quiet_hours_end: "08:00",
};

export function clockMinutes(value: string): number | null {
  if (!/^(?:[01]\d|2[0-3]):[0-5]\d$/.test(value)) return null;
  const [hours, minutes] = value.split(":").map(Number);
  return hours * 60 + minutes;
}

/** Uses the computer's local clock, including local daylight-saving transitions. */
export function isWithinQuietHours(settings: QuietHoursSettings, now = new Date()): boolean {
  if (!settings.quiet_hours_enabled) return false;
  const start = clockMinutes(settings.quiet_hours_start);
  const end = clockMinutes(settings.quiet_hours_end);
  if (start === null || end === null || start === end) return false;
  const minute = now.getHours() * 60 + now.getMinutes();
  return start < end ? minute >= start && minute < end : minute >= start || minute < end;
}
