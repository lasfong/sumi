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
  if (!(target instanceof Element)) return true;
  return target.closest(INTERACTIVE_OR_MODAL_TARGET) === null;
}
