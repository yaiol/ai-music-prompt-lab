// uis-ctl-menu.jsx — the ONE canonical dropdown / context menu for every yaiol electron app.
// A trigger-anchored flyout: a click-away overlay + a themed panel of rows, with optional
// section labels, dividers, and one level of hover-opened submenus. Every "menu that drops
// from a toolbar button" (New-document, Export-as, …) uses this so they all look and behave
// identically. Styling lives in the shared uis-app.css (.menu / .menu-item / .uis-men-label /
// .uis-men-divider / .uis-men-sub / .menu-flyout).
//
// ⚠ The menu MUST render OUTSIDE any .uis-barh-grp/.uis-barv-grp weld — a menu placed inside a weld inherits
// `.uis-barh-grp > * { height:100% }` and is clipped to the 30px bar (only its first row shows). Put
// the trigger button in the weld and render <UisMenu> as a SIBLING, inside a position:relative
// wrapper AROUND the weld:
//
//   <div style={{ position: 'relative' }}>
//     <div className="uis-barh-grp"><button onClick={() => setOpen(o => !o)}>…</button>…</div>
//     <UisMenu open={open} onClose={() => setOpen(false)} title="Create a new">
//       {items.map(it => <UisMenuItem key={it.id} label={it.label} onClick={() => pick(it)} />)}
//     </UisMenu>
//   </div>
//
// Controlled: pass `anchorRef` (a ref on the trigger button) + `open` + `onClose`. The panel PORTALS
// (via `UisPopover`), so it is never clipped by a scrolling ancestor and needs no relative wrapper.
// `align` ('auto' | 'left' | 'right') anchors it to that edge of the trigger. A <UisMenuItem> runs its
// onClick then closes; a UisMenuItem with a `submenu` node becomes a hover-flyout parent. <UisMenuLabel>
// is a section sub-label; <UisMenuDivider> a separator line.
//
// Distributed into each app's src/lib/ by sync-shared — ⚠ SYNCED FILE, never edit the per-app
// copy; edit this canonical source and re-sync. Import:
//   `import { UisMenu, UisMenuItem, UisMenuLabel, UisMenuDivider } from './lib/uis-ctl-menu';`
import React, { createContext, useContext, useState, useRef, useEffect, useId } from 'react';
import { UisPopover } from './uis-ctl-popover';

const NOOP = () => {};
const MenuCtx = createContext({ close: NOOP, openSub: null, setOpenSub: NOOP });

export function UisMenu({ anchorRef, open, onClose, title, align = 'auto', children }) {
  // ⚠ CLAUDE: the MENU owns which submenu is open — a single id, not a boolean per row.
  // When each row kept its own `hover` flag, sweeping down a column of parent rows left
  // every flyout on screen at once: leaving a row only SCHEDULED its close while the next
  // row opened immediately, so the delay stacked them. One id makes "only one submenu,
  // ever" structural instead of a race between timers.
  const [openSub, setOpenSub] = useState(null);

  // A reopened menu must never restore the submenu that was open when it closed.
  useEffect(() => { if (!open) setOpenSub(null); }, [open]);

  return (
    <UisPopover anchorRef={anchorRef} open={open} onClose={onClose} align={align} ignoreSelector=".menu-flyout">
      {title && <div className="uis-men-title">{title}</div>}
      <MenuCtx.Provider value={{ close: onClose, openSub, setOpenSub }}>{children}</MenuCtx.Provider>
    </UisPopover>
  );
}

export function UisMenuLabel({ children }) {
  return <div className="uis-men-label">{children}</div>;
}

export function UisMenuDivider() {
  return <div className="uis-men-divider" />;
}

export function UisMenuItem({ label, onClick, submenu, disabled = false, icon = null }) {
  const { close, openSub, setOpenSub } = useContext(MenuCtx);
  // Hooks stay unconditional — declared before the submenu branch below.
  const id     = useId();
  const rowRef = useRef(null);
  const timer  = useRef(null);
  useEffect(() => () => clearTimeout(timer.current), []);

  const hover = openSub === id;

  // Entering a row claims the submenu slot AT ONCE, so any sibling flyout disappears the
  // moment you reach the next row — no overlap, whatever the delay below is.
  const openNow = () => { clearTimeout(timer.current); setOpenSub(id); };

  // Hover intent: the flyout is a PORTAL, so leaving the row is not "leaving the menu".
  // Closing is deferred just long enough for the pointer to cross into the flyout, which
  // cancels it. The functional update is load-bearing — by the time this fires the pointer
  // may already have opened a SIBLING submenu, and a plain setOpenSub(null) would close
  // that one instead.
  const closeSoon = () => {
    clearTimeout(timer.current);
    timer.current = setTimeout(() => setOpenSub(cur => (cur === id ? null : cur)), 180);
  };

  // A parent row: no action of its own, opens `submenu` as a flyout while hovered.
  // ⚠ CLAUDE: the flyout MUST be portalled (via UisPopover), never an absolutely-positioned
  // child of the panel. `.uis-pop-surface` sets `overflow-y: auto` to cap tall panels, and an
  // `overflow` box clips on BOTH axes — so a child at `left: 100%` is invisible the moment
  // the menu is long enough to scroll. That is precisely how submenus were unreachable.
  if (submenu) {
    return (
      <div className="uis-men-sub" onMouseEnter={openNow} onMouseLeave={closeSoon}>
        <div ref={rowRef} className="uis-pop-item uis-men-item-parent" role="menuitem" aria-haspopup="true">
          <span>{label}</span>
          <span className="uis-men-caret" aria-hidden="true">▸</span>
        </div>
        <UisPopover anchorRef={rowRef} open={hover} onClose={() => setOpenSub(null)}
          placement="side" className="menu-flyout"
          onMouseEnter={openNow} onMouseLeave={closeSoon}>
          {/* ⚠ CLAUDE: the flyout's own rows share this context, so they must NOT be able to
              touch the submenu slot — a leaf's onMouseEnter would otherwise close the very
              flyout it lives in. Neutralising it here also caps nesting at one level. */}
          <MenuCtx.Provider value={{ close, openSub: null, setOpenSub: NOOP }}>
            <div role="menu">{submenu}</div>
          </MenuCtx.Provider>
        </UisPopover>
      </div>
    );
  }

  // A leaf row: close the menu first, then run the action (which may be async).
  // Hovering it also dismisses any open flyout — moving onto a plain row is a clear
  // "not going to the submenu", so it closes at once rather than on the delay.
  return (
    <button type="button" className="uis-pop-item" role="menuitem" disabled={disabled}
      onMouseEnter={() => setOpenSub(null)}
      onClick={() => { close(); onClick?.(); }}>
      {icon}<span>{label}</span>
    </button>
  );
}
