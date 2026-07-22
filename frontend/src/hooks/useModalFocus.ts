import { useEffect, useLayoutEffect, useRef } from 'react';

const FOCUSABLE = 'button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

export const useModalFocus = (active: boolean, onClose: () => void, dialog: HTMLDivElement | null) => {
  const openerRef = useRef<HTMLElement | null>(null);
  useLayoutEffect(() => {
    if (!active) {
      if (openerRef.current?.isConnected) openerRef.current.focus();
      openerRef.current = null;
      return undefined;
    }
    if (!openerRef.current && document.activeElement instanceof HTMLElement) openerRef.current = document.activeElement;
    if (!dialog) return undefined;
    dialog.querySelector<HTMLElement>(FOCUSABLE)?.focus();
    const handleKeyDown = (event: KeyboardEvent) => {
      event.stopPropagation();
      if (event.key === 'Escape') {
        event.preventDefault(); onClose(); return;
      }
      if (event.key !== 'Tab') return;
      const focusable = [...dialog.querySelectorAll<HTMLElement>(FOCUSABLE)];
      if (!focusable.length) return;
      const first = focusable[0]; const last = focusable[focusable.length - 1];
      if (event.shiftKey && document.activeElement === first) { event.preventDefault(); last.focus(); }
      else if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first.focus(); }
    };
    dialog.addEventListener('keydown', handleKeyDown);
    return () => dialog.removeEventListener('keydown', handleKeyDown);
  }, [active, dialog, onClose]);
  useEffect(() => () => { if (openerRef.current?.isConnected) openerRef.current.focus(); }, []);
};
