import { describe, expect, it } from 'vitest';
import { isGlobalShortcutEligible } from '../globalShortcutPolicy';

const eventFor = (target: EventTarget | null, defaultPrevented = false) => ({ target, defaultPrevented });

describe('global replay/drawing shortcut eligibility', () => {
  it('allows only an unhandled non-interactive workspace target', () => {
    const workspace = document.createElement('div');
    expect(isGlobalShortcutEligible(eventFor(workspace))).toBe(true);
    expect(isGlobalShortcutEligible(eventFor(workspace, true))).toBe(false);
  });

  it.each([
    ['button', '<button><span>Trade</span></button>', 'span'],
    ['link', '<a href="#"><span>Journal</span></a>', 'span'],
    ['form', '<form><div>field area</div></form>', 'div'],
    ['input', '<input>', 'input'],
    ['select', '<select><option>one</option></select>', 'select'],
    ['textarea', '<textarea></textarea>', 'textarea'],
    ['contenteditable', '<div contenteditable="true"><span>note</span></div>', 'span'],
    ['tab', '<div role="tab"><span>Trade</span></div>', 'span'],
    ['dialog', '<div role="dialog"><div>body</div></div>', 'div'],
    ['modal', '<section aria-modal="true"><div>body</div></section>', 'div'],
    ['opt-out', '<div data-global-shortcuts="off"><span>inspector</span></div>', 'span'],
  ])('rejects %s targets and descendants', (_name, html, selector) => {
    const host = document.createElement('div');
    host.innerHTML = html;
    expect(isGlobalShortcutEligible(eventFor(host.querySelector(selector)))).toBe(false);
  });
});
