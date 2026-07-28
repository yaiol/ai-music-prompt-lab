// ui-fx-toast.jsx — the ONE canonical transient "it happened" message for every yaiol
// electron app: a centered pill that floats over the bottom of the window, self-dismisses,
// and never takes a click. Reference: ai-music-prompt-lab ("3 cards deleted"), lrc-editor
// ("File saved."). Whenever an app confirms a completed action without asking anything, it
// says it through this hook — one timer, one look, no per-app toast state.
//
// The look is the shared `.toast` CSS in ui-app.css; the behaviour (the message state, the
// self-dismiss timer, replacing an on-screen toast instead of stacking) is here.
//
// Usage — render `toast` once, at the end of the app root:
//   const { showToast, toast } = useToast();
//   …
//   showToast(t('tstAppSaved'));            // default dwell
//   showToast('❌ ' + err.message, 6000);   // longer, for something the user must read
//   return <div className="app-root"> … {toast} </div>;
//
// An app whose bottom chrome (a player bar, a statusbar) would cover the pill lifts it with
// the `bottom` option — a per-instance OFFSET, never a re-shape:
//   const { showToast, toast } = useToast({ bottom: playerOpen ? 84 : undefined });
//
// Dwell time — DEFAULT_MS is a floor, not a ceiling: a short confirmation reads in 3s, but a
// message the user must act on (an error, a name, a count) deserves an explicit longer ms.
//
// Distributed into each app's src/lib/ui-fx-toast.jsx by sync-shared — ⚠ SYNCED FILE, never
// edit the per-app copy; edit this canonical source and re-sync.
// Import: `import { useToast } from './lib/ui-fx-toast';`
import React, { useState, useRef, useEffect, useCallback } from 'react';

const DEFAULT_MS = 3000;

export function useToast({ bottom } = {}) {
  const [msg, setMsg] = useState(null);
  const timer = useRef(null);

  // Stable identity: showToast is read by effects/handlers in every consumer, so it must not
  // change every render (the useT() lesson — an unstable callback destabilizes their deps).
  const showToast = useCallback((text, ms = DEFAULT_MS) => {
    if (timer.current) clearTimeout(timer.current);
    setMsg(text);
    timer.current = setTimeout(() => setMsg(null), ms);
  }, []);

  useEffect(() => () => { if (timer.current) clearTimeout(timer.current); }, []);

  const toast = msg ? (
    <div className="toast" style={bottom != null ? { '--toast-bottom': `${bottom}px` } : undefined}>{msg}</div>
  ) : null;

  return { showToast, toast };
}
