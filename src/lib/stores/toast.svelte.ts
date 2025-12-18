type ToastType = "success" | "error" | "info";

interface ToastItem {
  id: number;
  message: string;
  type: ToastType;
  duration: number;
}

let toasts = $state<ToastItem[]>([]);
let nextId = $state(0);

export function getToasts() {
  return toasts;
}

export function showToast(message: string, type: ToastType = "info", duration: number = 3000) {
  const id = nextId++;
  toasts = [...toasts, { id, message, type, duration }];
  return id;
}

export function removeToast(id: number) {
  toasts = toasts.filter((t) => t.id !== id);
}

export function showSuccess(message: string, duration: number = 3000) {
  return showToast(message, "success", duration);
}

export function showError(message: string, duration: number = 5000) {
  return showToast(message, "error", duration);
}

export function showInfo(message: string, duration: number = 3000) {
  return showToast(message, "info", duration);
}



