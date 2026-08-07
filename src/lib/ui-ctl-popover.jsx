// ui-ctl-popover.jsx — the ONE floating-panel MECHANISM for the popover family.
// It portals a panel to <body> and positions it against an anchor element: below the anchor (flips
// above when there's no room), width floored at the anchor (optional) and capped, anchored to the
// left or right edge so it stays on-screen, repositioned on scroll/resize, and closed on Esc /
// click-away. It owns POSITION + PORTAL + DISMISS only — NOT the panel's contents.
//
// `placement="side"` puts the panel BESIDE the anchor instead of under it (flipping to the other
// side when there is no room) — what a submenu flyout needs. `ignoreSelector` exempts a selector
// from click-away, so a portalled child panel does not dismiss its parent out from under a click.
// `onMouseEnter`/`onMouseLeave` pass through to the panel, so a hover-opened flyout can stay open
// while the pointer is over itself.
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

const MARGIN     = 8;    // breathing room kept between the panel and the window edge
const MIN_PANEL  = 160;  // never squeeze a panel below this, even in a cramped window
const FLIP_BELOW = 240;  // less room than this under the anchor → consider opening upwards

export function Popover({
  anchorRef, open, onClose,
  align = 'auto',            // 'left' | 'right' | 'auto' (right-half anchors right, else left)
  placement = 'below',       // 'below' (a dropdown) | 'side' (a submenu flyout, beside its row)
  minWidthAnchor = false,    // floor the panel width at the anchor's width (pickers do; menus don't)
  maxWidth = 400,
  maxHeight = null,          // null = fit the viewport (cap at the room on the chosen side)
  ignoreSelector = null,     // click-away exception: a mousedown inside this selector does NOT dismiss
  onMouseEnter, onMouseLeave, // hover passthrough — a hover-opened flyout must stay open over itself
  className = '', children,
}) {
  const [pos, setPos] = useState(null);
  const popRef = useRef(null);

  const place = useCallback(() => {
    const a = anchorRef?.current;
    if (!a) return;
    const r = a.getBoundingClientRect();
    const mw = Math.min(maxWidth, window.innerWidth - 16);

    // ⚠ CLAUDE: the height cap is the ROOM ON SCREEN, not a constant. It used to be a
    // flat `maxHeight = 320`, which made every long menu scroll on a 1000px-tall window
    // for no reason — the panel was capped at a third of the screen and the rest was
    // hidden behind a scrollbar. Only pass an explicit `maxHeight` to cap a panel BELOW
    // the available room on purpose (Combobox does, so a long list stays a short list).
    const roomBelow = window.innerHeight - r.bottom - MARGIN * 2;
    const roomAbove = r.top - MARGIN * 2;
    const fit = (room) => Math.max(MIN_PANEL, maxHeight != null ? Math.min(maxHeight, room) : room);

    // 'side' — a submenu opens BESIDE its parent row, not under it, and flips to the
    // other side when there is no room. Edges touch deliberately (no gap), so the
    // pointer can travel from the row into the flyout without crossing dead space.
    if (placement === 'side') {
      const flipLeft = r.right + mw + MARGIN > window.innerWidth && r.left > mw;
      // A flyout hangs DOWN from its row, so its room is everything below that row's top.
      const cap = fit(window.innerHeight - r.top - MARGIN * 2);
      setPos({
        maxWidth: mw, maxHeight: cap,
        ...(flipLeft ? { right: Math.max(MARGIN, window.innerWidth - r.left) } : { left: r.right }),
        // -4 cancels .pop-surface's padding so the first row lines up with the parent row.
        top: Math.max(MARGIN, Math.min(r.top - 4, window.innerHeight - cap - MARGIN)),
      });
      return;
    }

    // Flip up only when below is genuinely cramped AND above is roomier — not merely
    // because the panel could not have its full height below.
    const goUp = roomBelow < Math.min(maxHeight ?? FLIP_BELOW, FLIP_BELOW) && roomAbove > roomBelow;
    const anchorRight = align === 'right' || (align === 'auto' && r.left > window.innerWidth / 2);
    setPos({
      maxWidth: mw, maxHeight: fit(goUp ? roomAbove : roomBelow),
      ...(minWidthAnchor ? { minWidth: r.width } : {}),
      ...(anchorRight ? { right: Math.max(MARGIN, window.innerWidth - r.right) } : { left: r.left }),
      ...(goUp ? { bottom: window.innerHeight - r.top + 4 } : { top: r.bottom + 4 }),
    });
  }, [anchorRef, align, placement, minWidthAnchor, maxWidth, maxHeight]);

  useEffect(() => {
    if (!open) return;
    place();
    const onDown = (e) => {
      if (anchorRef?.current?.contains(e.target) || popRef.current?.contains(e.target)) return;
      // ⚠ CLAUDE: a portalled CHILD panel (a submenu flyout) is not inside popRef, so
      // without this exception the parent dismisses on mousedown — unmounting the child
      // before its click ever lands, and the menu item silently does nothing.
      if (ignoreSelector && e.target?.closest?.(ignoreSelector)) return;
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
  }, [open, place, onClose, anchorRef, ignoreSelector]);

  if (!open || !pos) return null;
  return createPortal(
    // `pos` already carries the computed maxHeight — do NOT re-apply the raw prop here,
    // which is null by default and would drop the cap entirely.
    <div ref={popRef} className={`pop-surface${className ? ' ' + className : ''}`} style={pos}
      onMouseEnter={onMouseEnter} onMouseLeave={onMouseLeave}>
      {children}
    </div>,
    document.body
  );
}
