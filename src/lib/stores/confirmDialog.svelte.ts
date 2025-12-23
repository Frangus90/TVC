export type ConfirmDialogType = "danger" | "warning" | "info";

export interface ConfirmDialogOptions {
  title: string;
  message: string;
  type?: ConfirmDialogType;
  confirmLabel?: string;
  cancelLabel?: string;
}

let isOpen = $state(false);
let dialogOptions = $state<ConfirmDialogOptions | null>(null);
let resolveCallback = $state<((value: boolean) => void) | null>(null);

export function isConfirmDialogOpen() {
  return isOpen;
}

export function getConfirmDialogOptions() {
  return dialogOptions;
}

export function openConfirmDialog(options: ConfirmDialogOptions): Promise<boolean> {
  return new Promise((resolve) => {
    dialogOptions = options;
    isOpen = true;
    resolveCallback = resolve;
  });
}

export function closeConfirmDialog(confirmed: boolean) {
  isOpen = false;
  if (resolveCallback) {
    resolveCallback(confirmed);
    resolveCallback = null;
  }
  dialogOptions = null;
}

