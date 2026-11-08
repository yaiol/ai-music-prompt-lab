// UisPnlSplitter - the ONE canonical resizable-panel divider for every yaiol app. Reference look:
// ai-music-prompt-lab. A 4px transparent grab strip, `col-resize` cursor, accent tint at 40%
// on hover and full accent while dragging. Distributed by sync-shared.js into each app's
// `src/lib/uis-pnl-splitter.jsx` - ⚠ SYNCED FILE, never edit the per-app copy; edit the canonical in
// the shared catalog's `lib/uis-pnl-splitter.jsx` and re-sync so every app stays byte-identical.
//
// The brand accent is the SAME in every app (only the theme differs), so both values live HERE
// (one source of truth) and the app just passes its current theme - `<UisPnlSplitter theme={theme} …/>`
// (`'light'` | `'dark'`, default dark). The DRAG WIRING stays the caller's: pass it as
// `onResizeStart`. Only the look is shared; every app's splitter is byte-identical.
import React, { useEffect } from 'react';

// Canonical brand accent - identical across all apps, theme-keyed. Keep in lockstep with the
// GUI Standard palette (light = accentLight #5a4fff, dark = accent #7c6fff).
const SPLITTER_ACCENT = { light: '#5a4fff', dark: '#7c6fff' };

let injected = false;
function injectSplitterStyle() {
  if (injected || typeof document === 'undefined') return;
  injected = true;
  const el = document.createElement('style');
  el.textContent =
    '.uis-pnl-splitter{width:4px;flex-shrink:0;cursor:col-resize;background:transparent;position:relative;z-index:10;align-self:stretch}' +
    '.uis-pnl-splitter:hover{background:color-mix(in srgb,var(--uis-pnl-splitter-accent) 40%,transparent)}' +
    '.uis-pnl-splitter:active{background:var(--uis-pnl-splitter-accent)}';
  document.head.appendChild(el);
}

export function UisPnlSplitter({ theme, onResizeStart, style, title, ...rest }) {
  useEffect(injectSplitterStyle, []);
  const accent = SPLITTER_ACCENT[theme] || SPLITTER_ACCENT.dark;
  return (
    <div
      className="uis-pnl-splitter"
      title={title}
      onMouseDown={onResizeStart}
      style={{ ...style, ['--uis-pnl-splitter-accent']: accent }}
      {...rest}
    />
  );
}
