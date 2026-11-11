const INTERACTIVE_OR_MODAL_TARGET = [
  'a[href]', 'button', 'input', 'select', 'textarea', 'option', 'form',
  '[contenteditable]:not([contenteditable="false"])',
  '[role="button"]', '[role="tab"]', '[role="dialog"]', '[aria-modal="true"]',
  '[role="textbox"]', '[role="combobox"]', '[role="slider"]',
  '[role="spinbutton"]', '[role="menuitem"]', '[data-global-shortcuts="off"]',
].join(',');

/** Global replay/drawing shortcuts are reserved for the non-interactive workspace. */
export function isGlobalShortcutEligible(event: Pick<KeyboardEvent, 'defaultPrevented' | 'target'>): boolean {
  if (event.defaultPrevented) return false;
  const target = event.target;
  if (target instanceof Element && target.closest(INTERACTIVE_OR_MODAL_TARGET) !== null) {
    return false;
  }
  if (typeof document !== 'undefined') {
    const active = document.activeElement;
    if (active instanceof Element && active !== document.body && active.closest(INTERACTIVE_OR_MODAL_TARGET) !== null) {
      return false;
    }
    if (document.querySelector('[role="dialog"], [aria-modal="true"]') !== null) {
      return false;
    }
  }
  return true;
}
