// Splitter - the ONE canonical resizable-panel divider for every yaiol app. Reference look:
// ai-music-prompt-lab. A 4px transparent grab strip, `col-resize` cursor, accent tint at 40%
// on hover and full accent while dragging. Distributed by sync-shared.js into each app's
// `src/lib/ui-ctl-splitter.jsx` - ⚠ SYNCED FILE, never edit the per-app copy; edit the canonical in
// `app/electron/.common/shared/lib/ui-ctl-splitter.jsx` and re-sync so every app stays byte-identical.
//
// The brand accent is the SAME in every app (only the theme differs), so both values live HERE
// (one source of truth) and the app just passes its current theme - `<Splitter theme={theme} …/>`
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
    '.app-splitter{width:4px;flex-shrink:0;cursor:col-resize;background:transparent;position:relative;z-index:10;align-self:stretch}' +
    '.app-splitter:hover{background:color-mix(in srgb,var(--app-splitter-accent) 40%,transparent)}' +
    '.app-splitter:active{background:var(--app-splitter-accent)}';
  document.head.appendChild(el);
}

export function Splitter({ theme, onResizeStart, style, title = 'Drag to resize', ...rest }) {
  useEffect(injectSplitterStyle, []);
  const accent = SPLITTER_ACCENT[theme] || SPLITTER_ACCENT.dark;
  return (
    <div
      className="app-splitter"
      title={title}
      onMouseDown={onResizeStart}
      style={{ ...style, ['--app-splitter-accent']: accent }}
      {...rest}
    />
  );
}
