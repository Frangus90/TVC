const stack: HTMLElement[] = [];

/** Keep keyboard focus in the topmost modal and restore its opener on close. */
export function modalFocus(node: HTMLElement, close: () => void) {
  const previous = document.activeElement instanceof HTMLElement ? document.activeElement : null;
  stack.push(node);
  const controls = () => Array.from(node.querySelectorAll<HTMLElement>(
    'button:not(:disabled), input:not(:disabled), select:not(:disabled), textarea:not(:disabled), a[href], [tabindex="0"]',
  )).filter(el => el.getClientRects().length > 0);
  const focusFirst = () => (node.querySelector<HTMLElement>('[data-modal-initial]') ?? controls()[0] ?? node).focus();
  const top = () => stack[stack.length - 1] === node;
  function keydown(event: KeyboardEvent) {
    if (!top()) return;
    if (event.key === 'Escape') {
      event.preventDefault(); event.stopImmediatePropagation(); close();
    } else if (event.key === 'Tab') {
      const items = controls();
      const index = items.indexOf(document.activeElement as HTMLElement);
      if (!items.length) { event.preventDefault(); node.focus(); return; }
      if (event.shiftKey ? index <= 0 : index === items.length - 1 || index < 0) {
        event.preventDefault();
        items[event.shiftKey ? items.length - 1 : 0].focus();
      }
    }
  }
  function focusin(event: FocusEvent) {
    if (top() && !node.contains(event.target as Node)) focusFirst();
  }
  node.tabIndex = -1;
  document.addEventListener('keydown', keydown, true);
  document.addEventListener('focusin', focusin);
  queueMicrotask(() => { if (top() && node.isConnected) focusFirst(); });
  return { destroy() {
    stack.splice(stack.indexOf(node), 1);
    document.removeEventListener('keydown', keydown, true);
    document.removeEventListener('focusin', focusin);
    if (previous?.isConnected) previous.focus();
  } };
}
