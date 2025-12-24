import { addMonths, subMonths, addWeeks, subWeeks, startOfMonth, endOfMonth, startOfWeek, endOfWeek, format } from "date-fns";

let currentDate = $state(new Date());
let viewMode = $state<"month" | "week" | "agenda" | "tier">("month");

export function getCurrentDate() {
  return currentDate;
}

export function getViewMode() {
  return viewMode;
}

export function setViewMode(mode: "month" | "week" | "agenda" | "tier") {
  viewMode = mode;
}

export function previousPeriod() {
  if (viewMode === "week") {
    currentDate = subWeeks(currentDate, 1);
  } else {
    currentDate = subMonths(currentDate, 1);
  }
}

export function nextPeriod() {
  if (viewMode === "week") {
    currentDate = addWeeks(currentDate, 1);
  } else {
    currentDate = addMonths(currentDate, 1);
  }
}

export function goToToday() {
  currentDate = new Date();
}

export function getMonthRange(): { start: string; end: string } {
  const monthStart = startOfMonth(currentDate);
  const monthEnd = endOfMonth(currentDate);
  return {
    start: format(monthStart, "yyyy-MM-dd"),
    end: format(monthEnd, "yyyy-MM-dd"),
  };
}

export function getWeekRange(): { start: string; end: string } {
  const weekStart = startOfWeek(currentDate, { weekStartsOn: 1 });
  const weekEnd = endOfWeek(currentDate, { weekStartsOn: 1 });
  return {
    start: format(weekStart, "yyyy-MM-dd"),
    end: format(weekEnd, "yyyy-MM-dd"),
  };
}
