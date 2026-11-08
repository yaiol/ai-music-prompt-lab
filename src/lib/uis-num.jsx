// uis-num.jsx — the ONE canonical numeric input for every yaiol electron app.
// Replaces the native browser number-spinner (a platform-drawn sub-control that
// ignores the theme background) with a fully themed field + ▲▼ steppers, so the
// whole control — arrows included — uses the field colours. Styling lives in the
// shared uis-app.css (.uis-num / .uis-num-input / .uis-num-steppers / .uis-num-step);
// context (header / footer / dlg) sets the fill, exactly like the text inputs.
//
// Controlled: pass `value` (number, or '' when allowEmpty) and `onChange(next)`.
// The field keeps an internal draft so you can type freely (incl. intermediate
// below-min values); it emits a valid Number while typing and CLAMPS to [min,max]
// on blur and on the ▲▼ steppers — the caller never needs its own onBlur clamp.
// `allowEmpty` lets the field be blank (emits '' — for placeholder / "unset" fields).
// Optional: min, max, step (default 1), width (px or CSS length), disabled, title,
// className ('is-active' → accent border). Extra props (placeholder, onKeyDown, …)
// pass through to the <input>.
// `scrub` (units per pixel, default off) makes the field a drag control, as in a grading panel: the pointer
// over it is a left-right arrow, pressing and dragging sideways changes the value (snapped to `step`, clamped),
// and a press that does not move focuses the field to type. The steppers are not drawn then. Once it moves, the pointer is LOCKED (hidden,
// not stopped by the screen's edge) and the distance is summed from the movement itself, so a field at the edge of the window drags on
// without end, as in a grading panel.
//
// Distributed into each app's src/lib/uis-num.jsx by sync-shared — ⚠ SYNCED
// FILE, never edit the per-app copy; edit this canonical source and re-sync.
// Import: `import { UisNum } from './lib/uis-num';`
import React, { useState, useEffect, useRef } from 'react';

const Chevron = ({ up }) => (
  <svg width="9" height="9" viewBox="0 0 24 24" fill="none" stroke="currentColor"
    strokeWidth="3" strokeLinecap="round" strokeLinejoin="round">
    <polyline points={up ? '6 15 12 9 18 15' : '6 9 12 15 18 9'} />
  </svg>
);

export function UisNum({ value, onChange, min, max, step = 1, decimals, width, className = '', title, disabled = false, allowEmpty = false, scrub = 0, ...rest }) {
  const [draft, setDraft] = useState('');
  const held = useRef(null);
  // Mirror the controlled value into the draft (skips while the user is mid-edit,
  // because an in-range keystroke already pushed that same value up).
  // `decimals` (optional) is how many places the field shows: a number is written with exactly that many (0.6 as 0.60) once it is not being typed.
  const fmt = (n) => (n === '' || n == null ? '' : decimals == null ? String(n) : Number(n).toFixed(decimals));
  useEffect(() => { setDraft((d) => (value !== '' && value != null && parseFloat(d) === Number(value) ? d : fmt(value))); }, [value]);

  const clamp = (v) => {
    if (Number.isNaN(v)) return allowEmpty ? '' : (min != null ? min : 0);
    if (min != null) v = Math.max(min, v);
    if (max != null) v = Math.min(max, v);
    return v;
  };
  const onInput = (e) => {
    const raw = e.target.value;
    setDraft(raw);
    if (allowEmpty && raw.trim() === '') { onChange(''); return; }
    const n = parseFloat(raw);
    if (!Number.isNaN(n) && (min == null || n >= min) && (max == null || n <= max)) onChange(n);
  };
  const onBlur = (e) => {
    if (allowEmpty && e.target.value.trim() === '') { onChange(''); return; }
    const n = clamp(parseFloat(e.target.value));
    setDraft(fmt(n));   // show the clamped number even when the value did not move (it would keep the typed text otherwise)
    onChange(n);
  };
  // A step is added in binary floating point, so 0.7 + 0.05 can come out 0.7499999999999999: the result is rounded to the decimals the step
  // and the value themselves carry.
  const places = (x) => { const t = String(x); const i = t.indexOf('.'); return i < 0 || /e/i.test(t) ? 0 : t.length - i - 1; };
  const bump = (dir) => {
    if (disabled) return;
    const from = Number(value) || 0, p = Math.min(12, Math.max(places(step), places(from)));
    onChange(clamp(Number((from + dir * step).toFixed(p))));
  };

  // Scrubbing: a press on the field that is not being typed in takes the pointer; moving sideways turns the distance into a value, and a press
  // that never moved (3 px) is a click, which focuses the field.
  const onPointerDown = (e) => {
    if (!scrub || disabled || e.button !== 0 || document.activeElement === e.currentTarget) return;
    held.current = { dx: 0, v: Number(value) || 0, moved: false };
    e.currentTarget.setPointerCapture(e.pointerId);
    e.preventDefault();
  };
  const onPointerMove = (e) => {
    const h = held.current;
    if (!h) return;
    h.dx += e.movementX;
    const dx = h.dx;
    if (!h.moved && Math.abs(dx) < 3) return;
    if (!h.moved) { try { e.currentTarget.requestPointerLock(); } catch { /* the capture still drags to the edge */ } }
    h.moved = true;
    const q = Number((Math.round((h.v + dx * scrub) / step) * step).toFixed(Math.min(12, Math.max(places(step), places(h.v)))));
    const c = clamp(q) === 0 ? 0 : clamp(q); // (never -0)
    // Past an end the drag starts again from that end: moving back changes the value at once, instead of first undoing the overshoot.
    if (c !== q && c !== '') { h.v = c; h.dx = 0; }
    onChange(c);
  };
  const onPointerUp = (e) => {
    const h = held.current;
    held.current = null;
    if (document.pointerLockElement === e.currentTarget) document.exitPointerLock();
    if (h && !h.moved) { e.currentTarget.focus(); e.currentTarget.select(); }
  };

  return (
    <div className={`uis-num ${scrub ? 'scrub ' : ''}${className}`.trim()} style={width != null ? { width } : undefined} title={title}>
      <input type="number" className="uis-num-input" value={draft} min={min} max={max} step={step}
        disabled={disabled} onChange={onInput} onBlur={onBlur}
        onPointerDown={onPointerDown} onPointerMove={onPointerMove} onPointerUp={onPointerUp} {...rest} />
      <span className="uis-num-steppers" aria-hidden="true">
        <button type="button" className="uis-num-step" tabIndex={-1} disabled={disabled} onClick={() => bump(1)}><Chevron up /></button>
        <button type="button" className="uis-num-step" tabIndex={-1} disabled={disabled} onClick={() => bump(-1)}><Chevron /></button>
      </span>
    </div>
  );
}
