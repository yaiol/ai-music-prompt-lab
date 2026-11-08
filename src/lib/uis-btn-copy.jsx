// uis-btn-copy.jsx — the ONE canonical copy-to-clipboard icon button for every yaiol app: a `.uis-btn ico`
// that writes `text` to the clipboard and turns into a check mark for a moment, then back. Reference:
// apps-cockpit's field / traffic / diff / push / job copy buttons, ai-music-prompt-lab's URL row and
// prompt-expert's result rows (a hand-built Copy → Check flip in each before this).
//
// `text` is the string to copy, or a function returning it - sync or async, read at click time (text built on demand, or fetched first).
// `subtle` = `.uis-btn.subtle`, `small` (default true) = `.uis-btn.small`. `onCopied()` runs after a
// successful copy (an app that also shows a toast says so there); a denied clipboard does nothing and
// shows no check. Disabled with nothing to copy. `title` is the tooltip and the accessible name (already
// localized). Look lives in uis-app.css (.uis-btn).
//   <UisBtnCopy text={value} title={`Copy ${label}`} />
//
// Distributed into each app's src/lib/uis-btn-copy.jsx by sync-shared — ⚠ SYNCED FILE,
// never edit the per-app copy; edit this canonical source and re-sync.
// Import: `import { UisBtnCopy } from './lib/uis-btn-copy';`
import React, { useState, useRef, useEffect } from 'react';
import { Check, Copy } from 'lucide-react';

const FLIP_MS = 1200;

export function UisBtnCopy({ text, title, subtle = false, small = true, disabled = false, onCopied }) {
  const [done, setDone] = useState(false);
  const timer = useRef(null);
  useEffect(() => () => clearTimeout(timer.current), []);
  async function copy() {
    const v = typeof text === 'function' ? await text() : text;   // a function may be async (fetch first, then copy)
    if (v == null || v === '') return;
    try {
      await navigator.clipboard.writeText(String(v));
    } catch { return; }   // clipboard denied - nothing to recover
    setDone(true);
    clearTimeout(timer.current);
    timer.current = setTimeout(() => setDone(false), FLIP_MS);
    if (onCopied) onCopied();
  }
  return (
    <button type="button" className={`uis-btn ico${small ? ' small' : ''}${subtle ? ' subtle' : ''}`}
      onClick={copy} disabled={disabled || (typeof text !== 'function' && !text)} title={title} aria-label={title}>
      {done ? <Check /> : <Copy />}
    </button>
  );
}
