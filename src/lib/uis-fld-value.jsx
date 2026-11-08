// uis-fld-value.jsx — the ONE canonical read-only value row for every yaiol app: a bordered rounded row that
// shows a NAMED value with actions beside it (a URL with open / copy / remove, a result variable with copy).
// Reference: ai-music-prompt-lab's URL row and prompt-expert's result rows (the same shape hand-built in each).
// It owns the shell - surface, border, the label, the value, the action slot; the actions are the caller's
// (`UisBtnCopy` and the other `.uis-btn ico small subtle` buttons), and the value is the children.
//
// `label` = the name (accent colour), `accent` = a CSS colour that recolours it for this one row (set as the
// row's `--uis-accent`, never as inline geometry), `mono` = the value in the monospace font, `wrap` = a long
// value: the value drops under the label and wraps instead of being cut with an ellipsis. `actions` = the buttons.
//   <UisFldValue label="Spotify" accent={color} actions={<><UisBtnCopy text={href} /></>}>{href}</UisFldValue>
//
// Distributed into each app's src/lib/uis-fld-value.jsx by sync-shared — ⚠ SYNCED FILE,
// never edit the per-app copy; edit this canonical source and re-sync.
// Import: `import { UisFldValue } from './lib/uis-fld-value';`
import React from 'react';

export function UisFldValue({ label, accent, wrap = false, mono = false, actions, children }) {
  return (
    <div className={`uis-fld-value${wrap ? ' wrap' : ''}`} style={accent ? { '--uis-accent': accent } : undefined}>
      <span className="uis-fld-value-label">{label}</span>
      <span className={`uis-fld-value-text${mono ? ' mono' : ''}`}>{children}</span>
      {actions && <span className="uis-fld-value-actions">{actions}</span>}
    </div>
  );
}
