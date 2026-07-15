// ui-ctl-popover.jsx — the ONE floating-panel MECHANISM for the popover family.
// It portals a panel to <body> and positions it against an anchor element: below the anchor (flips
// above when there's no room), width floored at the anchor (optional) and capped, anchored to the
// left or right edge so it stays on-screen, repositioned on scroll/resize, and closed on Esc /
// click-away. It owns POSITION + PORTAL + DISMISS only — NOT the panel's contents.
//
// Every floating dropdown in the catalog is built on this so none of them re-implement (or forget,
// as `Menu` once did) the portal — the reason a dropdown inside a scrolling dialog used to clip:
//   • Menu       — a toolbar "pick one action" dropdown
//   • Combobox   — a type-to-filter picker
//   • Switcher   — a pick-one-value list whose rows carry their own actions (rename/delete)
// The shared LOOK is `.pop-surface` (this panel) + `.pop-item` (a row), in ui-app.css.
//
// Distributed by sync-shared — ⚠ SYNCED FILE, edit the canonical here and re-sync.
// Import: `import { Popover } from './lib/ui-ctl-popover';`
import React, { useState, useRef, useEffect, useCallback } from 'react';
import { createPortal } from 'react-dom';

export function Popover({
  anchorRef, open, onClose,
  align = 'auto',            // 'left' | 'right' | 'auto' (right-half anchors right, else left)
  minWidthAnchor = false,    // floor the panel width at the anchor's width (pickers do; menus don't)
  maxWidth = 400, maxHeight = 320,
  className = '', children,
}) {
  const [pos, setPos] = useState(null);
  const popRef = useRef(null);

  const place = useCallback(() => {
    const a = anchorRef?.current;
    if (!a) return;
    const r = a.getBoundingClientRect();
    const mw = Math.min(maxWidth, window.innerWidth - 16);
    const room = window.innerHeight - r.bottom;
    const goUp = room < maxHeight + 8 && r.top > room;
    const anchorRight = align === 'right' || (align === 'auto' && r.left > window.innerWidth / 2);
    setPos({
      maxWidth: mw,
      ...(minWidthAnchor ? { minWidth: r.width } : {}),
      ...(anchorRight ? { right: Math.max(8, window.innerWidth - r.right) } : { left: r.left }),
      ...(goUp ? { bottom: window.innerHeight - r.top + 4 } : { top: r.bottom + 4 }),
    });
  }, [anchorRef, align, minWidthAnchor, maxWidth, maxHeight]);

  useEffect(() => {
    if (!open) return;
    place();
    const onDown = (e) => {
      if (anchorRef?.current?.contains(e.target) || popRef.current?.contains(e.target)) return;
      onClose?.();
    };
    const onKey = (e) => { if (e.key === 'Escape') onClose?.(); };
    document.addEventListener('mousedown', onDown);
    document.addEventListener('keydown', onKey);
    window.addEventListener('scroll', place, true);
    window.addEventListener('resize', place);
    return () => {
      document.removeEventListener('mousedown', onDown);
      document.removeEventListener('keydown', onKey);
      window.removeEventListener('scroll', place, true);
      window.removeEventListener('resize', place);
    };
  }, [open, place, onClose, anchorRef]);

  if (!open || !pos) return null;
  return createPortal(
    <div ref={popRef} className={`pop-surface${className ? ' ' + className : ''}`} style={{ ...pos, maxHeight }}>
      {children}
    </div>,
    document.body
  );
}
