// ui-ctl-numberfield.jsx — the ONE canonical numeric input for every yaiol electron app.
// Replaces the native browser number-spinner (a platform-drawn sub-control that
// ignores the theme background) with a fully themed field + ▲▼ steppers, so the
// whole control — arrows included — uses the field colours. Styling lives in the
// shared ui-app.css (.numfield / .numfield-input / .numfield-steppers / .numfield-step);
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
//
// Distributed into each app's src/lib/ui-ctl-numberfield.jsx by sync-shared — ⚠ SYNCED
// FILE, never edit the per-app copy; edit this canonical source and re-sync.
// Import: `import { NumberField } from './lib/ui-ctl-numberfield';`
import React, { useState, useEffect } from 'react';

const Chevron = ({ up }) => (
  <svg width="9" height="9" viewBox="0 0 24 24" fill="none" stroke="currentColor"
    strokeWidth="3" strokeLinecap="round" strokeLinejoin="round">
    <polyline points={up ? '6 15 12 9 18 15' : '6 9 12 15 18 9'} />
  </svg>
);

export function NumberField({ value, onChange, min, max, step = 1, width, className = '', title, disabled = false, allowEmpty = false, ...rest }) {
  const [draft, setDraft] = useState('');
  // Mirror the controlled value into the draft (skips while the user is mid-edit,
  // because an in-range keystroke already pushed that same value up).
  useEffect(() => { setDraft(value === '' || value == null ? '' : String(value)); }, [value]);

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
    onChange(clamp(parseFloat(e.target.value)));
  };
  const bump = (dir) => { if (!disabled) onChange(clamp((Number(value) || 0) + dir * step)); };

  return (
    <div className={`numfield ${className}`.trim()} style={width != null ? { width } : undefined} title={title}>
      <input type="number" className="numfield-input" value={draft} min={min} max={max} step={step}
        disabled={disabled} onChange={onInput} onBlur={onBlur} {...rest} />
      <span className="numfield-steppers" aria-hidden="true">
        <button type="button" className="numfield-step" tabIndex={-1} disabled={disabled} onClick={() => bump(1)}><Chevron up /></button>
        <button type="button" className="numfield-step" tabIndex={-1} disabled={disabled} onClick={() => bump(-1)}><Chevron /></button>
      </span>
    </div>
  );
}
