// Custom mouse-based drag and drop system
// Uses a full-screen overlay to reliably capture all mouse events

export interface DragData {
  type: "show" | "movie";
  id: number;
}

interface DragState {
  isTracking: boolean;
  isDragging: boolean;
  dragData: DragData | null;
  startX: number;
  startY: number;
  ghostX: number;
  ghostY: number;
}

const DRAG_THRESHOLD = 3;

let state = $state<DragState>({
  isTracking: false,
  isDragging: false,
  dragData: null,
  startX: 0,
  startY: 0,
  ghostX: 0,
  ghostY: 0,
});

const dropZones = new Map<string, {
  element: HTMLElement;
  onDrop: (data: DragData) => void;
  onDragEnter?: () => void;
  onDragLeave?: () => void;
}>();

let currentHoveredZone: string | null = null;
let overlay: HTMLDivElement | null = null;

export function getIsDragging() {
  return state.isDragging;
}

export function getDragData() {
  return state.dragData;
}

export function getGhostPosition() {
  return { x: state.ghostX, y: state.ghostY };
}

function createOverlay() {
  overlay = document.createElement("div");
  overlay.style.cssText = `
    position: fixed;
    top: 0;
    left: 0;
    width: 100vw;
    height: 100vh;
    z-index: 9998;
    cursor: grabbing;
  `;
  overlay.addEventListener("mousemove", handleMouseMove);
  overlay.addEventListener("mouseup", handleMouseUp);
  document.body.appendChild(overlay);
}

function removeOverlay() {
  if (overlay) {
    overlay.removeEventListener("mousemove", handleMouseMove);
    overlay.removeEventListener("mouseup", handleMouseUp);
    overlay.remove();
    overlay = null;
  }
}

export function startDrag(data: DragData, x: number, y: number) {
  state.isTracking = true;
  state.isDragging = false;
  state.dragData = data;
  state.startX = x;
  state.startY = y;
  state.ghostX = x;
  state.ghostY = y;

  // Create overlay to capture all mouse events
  createOverlay();
}

function handleMouseMove(e: MouseEvent) {
  if (!state.isTracking) return;

  state.ghostX = e.clientX;
  state.ghostY = e.clientY;

  if (!state.isDragging) {
    const dx = Math.abs(e.clientX - state.startX);
    const dy = Math.abs(e.clientY - state.startY);
    if (dx > DRAG_THRESHOLD || dy > DRAG_THRESHOLD) {
      state.isDragging = true;
    } else {
      return;
    }
  }

  // Check drop zones (look through the overlay)
  if (overlay) overlay.style.pointerEvents = "none";
  const elementUnder = document.elementFromPoint(e.clientX, e.clientY);
  if (overlay) overlay.style.pointerEvents = "auto";

  const dropZoneEl = elementUnder?.closest("[data-drop-zone]") as HTMLElement | null;
  const zoneId = dropZoneEl?.dataset.dropZone ?? null;

  if (zoneId !== currentHoveredZone) {
    if (currentHoveredZone && dropZones.has(currentHoveredZone)) {
      dropZones.get(currentHoveredZone)?.onDragLeave?.();
    }
    if (zoneId && dropZones.has(zoneId)) {
      dropZones.get(zoneId)?.onDragEnter?.();
    }
    currentHoveredZone = zoneId;
  }
}

function handleMouseUp(e: MouseEvent) {
  if (!state.isTracking) {
    cleanup();
    return;
  }

  if (state.isDragging && state.dragData) {
    // Look through overlay to find drop zone
    if (overlay) overlay.style.pointerEvents = "none";
    const elementUnder = document.elementFromPoint(e.clientX, e.clientY);
    if (overlay) overlay.style.pointerEvents = "auto";

    const dropZoneEl = elementUnder?.closest("[data-drop-zone]") as HTMLElement | null;
    const zoneId = dropZoneEl?.dataset.dropZone ?? null;

    if (zoneId && dropZones.has(zoneId)) {
      dropZones.get(zoneId)!.onDrop(state.dragData);
    }
  }

  cleanup();
}

function cleanup() {
  if (currentHoveredZone && dropZones.has(currentHoveredZone)) {
    dropZones.get(currentHoveredZone)?.onDragLeave?.();
  }
  currentHoveredZone = null;

  state.isTracking = false;
  state.isDragging = false;
  state.dragData = null;

  removeOverlay();
}

export function registerDropZone(
  id: string,
  element: HTMLElement,
  callbacks: {
    onDrop: (data: DragData) => void;
    onDragEnter?: () => void;
    onDragLeave?: () => void;
  }
) {
  dropZones.set(id, { element, ...callbacks });
  return () => dropZones.delete(id);
}
