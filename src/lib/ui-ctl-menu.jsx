// ui-ctl-menu.jsx — the ONE canonical dropdown / context menu for every yaiol electron app.
// A trigger-anchored flyout: a click-away overlay + a themed panel of rows, with optional
// section labels, dividers, and one level of hover-opened submenus. Every "menu that drops
// from a toolbar button" (New-document, Export-as, …) uses this so they all look and behave
// identically. Styling lives in the shared ui-app.css (.menu / .menu-item / .menu-label /
// .menu-divider / .menu-sub / .menu-flyout).
//
// ⚠ The menu MUST render OUTSIDE any .barh-grp/.barv-grp weld — a menu placed inside a weld inherits
// `.barh-grp > * { height:100% }` and is clipped to the 30px bar (only its first row shows). Put
// the trigger button in the weld and render <Menu> as a SIBLING, inside a position:relative
// wrapper AROUND the weld:
//
//   <div style={{ position: 'relative' }}>
//     <div className="barh-grp"><button onClick={() => setOpen(o => !o)}>…</button>…</div>
//     <Menu open={open} onClose={() => setOpen(false)} title="Create a new">
//       {items.map(it => <MenuItem key={it.id} label={it.label} onClick={() => pick(it)} />)}
//     </Menu>
//   </div>
//
// Controlled: pass `anchorRef` (a ref on the trigger button) + `open` + `onClose`. The panel PORTALS
// (via `Popover`), so it is never clipped by a scrolling ancestor and needs no relative wrapper.
// `align` ('auto' | 'left' | 'right') anchors it to that edge of the trigger. A <MenuItem> runs its
// onClick then closes; a MenuItem with a `submenu` node becomes a hover-flyout parent. <MenuLabel>
// is a section sub-label; <MenuDivider> a separator line.
//
// Distributed into each app's src/lib/ by sync-shared — ⚠ SYNCED FILE, never edit the per-app
// copy; edit this canonical source and re-sync. Import:
//   `import { Menu, MenuItem, MenuLabel, MenuDivider } from './lib/ui-ctl-menu';`
import React, { createContext, useContext, useState } from 'react';
import { Popover } from './ui-ctl-popover';

const MenuCtx = createContext({ close: () => {} });

export function Menu({ anchorRef, open, onClose, title, align = 'auto', children }) {
  return (
    <Popover anchorRef={anchorRef} open={open} onClose={onClose} align={align}>
      {title && <div className="menu-title">{title}</div>}
      <MenuCtx.Provider value={{ close: onClose }}>{children}</MenuCtx.Provider>
    </Popover>
  );
}

export function MenuLabel({ children }) {
  return <div className="menu-label">{children}</div>;
}

export function MenuDivider() {
  return <div className="menu-divider" />;
}

export function MenuItem({ label, onClick, submenu, disabled = false, icon = null }) {
  const { close } = useContext(MenuCtx);
  const [hover, setHover] = useState(false);

  // A parent row: no action of its own, opens `submenu` as a flyout while hovered.
  // The flyout lives inside the .menu-sub wrapper so moving the pointer from the row
  // into the flyout stays "inside" (one hover region — no gap-close flicker).
  if (submenu) {
    return (
      <div className="menu-sub" onMouseEnter={() => setHover(true)} onMouseLeave={() => setHover(false)}>
        <div className="pop-item menu-item-parent" role="menuitem" aria-haspopup="true">
          <span>{label}</span>
          <span className="menu-caret" aria-hidden="true">▸</span>
        </div>
        {hover && <div className="pop-surface menu-flyout" role="menu">{submenu}</div>}
      </div>
    );
  }

  // A leaf row: close the menu first, then run the action (which may be async).
  return (
    <button type="button" className="pop-item" role="menuitem" disabled={disabled}
      onClick={() => { close(); onClick?.(); }}>
      {icon}<span>{label}</span>
    </button>
  );
}
