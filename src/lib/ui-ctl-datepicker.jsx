// ui-ctl-datepicker.jsx — the ONE canonical date picker for every yaiol electron app.
// A themed calendar popup replacing the native <input type="date"> (which the browser draws
// itself and cannot be themed to match the app — same reason NumberField replaces the native
// number spinner). Trigger = a field showing the formatted date + a calendar glyph; clicking
// opens a portal popup: a month header (lucide chevron nav + a month/year jump), a weekday
// row, the day grid, and a Today / Clear footer.
//
// i18n WITHOUT app coupling: the month + weekday names and the display format come from the
// browser's built-in Intl (the `locale` prop, e.g. "fr" / "en" / "pt-BR") — the SAME locale
// data that localizes the native picker, so it covers every language for free with zero i18n
// keys. The only words passed in are `todayLabel` / `clearLabel` (default English), exactly
// like ColorPicker's `cancelLabel` / `applyLabel`.
//
// Controlled: `value` is a "YYYY-MM-DD" string ("" = empty) + `onChange(str)` ("" when cleared).
//
// Distributed into each app's src/lib by sync-shared — ⚠ SYNCED FILE, never edit the per-app
// copy; edit this canonical source and re-sync. The look is the PAIRED ui-ctl-datepicker.css
// (imported below → it rides only into apps that use this control, not into every ui-app.css).
// Buttons are the shared .btn.icon / .btn. Import: `import { DatePicker } from './lib/ui-ctl-datepicker';`
import React, { useState, useRef, useEffect } from 'react';
import { createPortal } from 'react-dom';
import { ChevronUp, ChevronDown, CalendarDays } from 'lucide-react';
import '../assets/ui-ctl-datepicker.css';

const pad = (n) => String(n).padStart(2, '0');
const toYMD = (y, m, d) => `${y}-${pad(m + 1)}-${pad(d)}`;   // m is 0-indexed
function parseYMD(s) { const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(s || ''); return m ? { y: +m[1], m: +m[2] - 1, d: +m[3] } : null; }
function todayParts() { const t = new Date(); return { y: t.getFullYear(), m: t.getMonth(), d: t.getDate() }; }

export function DatePicker({ value, onChange, locale = 'en', todayLabel = 'Today', clearLabel = 'Clear', placeholder = '', className = '', style }) {
  const loc = (locale || 'en').replace(/_/g, '-');   // app langKeys use "_" (pt_BR); Intl wants "-"
  const [open, setOpen] = useState(false);
  const [jump, setJump] = useState(false);
  const [pos, setPos] = useState({ top: 0, left: 0 });
  const sel = parseYMD(value);
  const [view, setView] = useState(() => { const p = sel || todayParts(); return { y: p.y, m: p.m }; });
  const btnRef = useRef(null);
  const popRef = useRef(null);

  // ── Intl-derived locale data (no app i18n) ─────────────────────────────────
  const monthLabel = new Intl.DateTimeFormat(loc, { month: 'long', year: 'numeric' }).format(new Date(view.y, view.m, 1));
  const displayFmt = new Intl.DateTimeFormat(loc);
  let firstDay = 1;   // 1=Mon … 7=Sun
  try { const wi = new Intl.Locale(loc).weekInfo; if (wi && wi.firstDay) firstDay = wi.firstDay; } catch { /* weekInfo unsupported → Monday */ }
  const firstDayJS = firstDay % 7;    // 0=Sun … 6=Sat
  const wdFmt = new Intl.DateTimeFormat(loc, { weekday: 'short' });
  const weekdays = [];
  for (let i = 0; i < 7; i++) weekdays.push(wdFmt.format(new Date(2023, 0, 1 + ((firstDayJS + i) % 7))));   // 2023-01-01 is a Sunday
  const monthFmt = new Intl.DateTimeFormat(loc, { month: 'short' });
  const monthNames = [];
  for (let m = 0; m < 12; m++) monthNames.push(monthFmt.format(new Date(2023, m, 1)));

  // ── Day grid: 6 weeks; JS Date normalizes over/underflow into adjacent months ─
  const leading = (new Date(view.y, view.m, 1).getDay() - firstDayJS + 7) % 7;
  const cells = [];
  for (let i = 0; i < 42; i++) cells.push(new Date(view.y, view.m, i - leading + 1));
  const now = new Date();
  const same = (a, y, m, d) => a.getFullYear() === y && a.getMonth() === m && a.getDate() === d;

  const place = () => {
    if (!btnRef.current) return;
    const r = btnRef.current.getBoundingClientRect();
    const POP_H = 340;
    const goUp = r.bottom + POP_H > window.innerHeight && r.top > POP_H;
    setPos(goUp ? { top: r.top - POP_H - 6, left: r.left } : { top: r.bottom + 6, left: r.left });
  };
  const toggle = () => {
    if (!open) { const p = sel || todayParts(); setView({ y: p.y, m: p.m }); setJump(false); place(); }
    setOpen((v) => !v);
  };
  useEffect(() => {
    if (!open) return;
    const onDown = (e) => { if (btnRef.current?.contains(e.target)) return; if (!popRef.current?.contains(e.target)) setOpen(false); };
    const onKey = (e) => { if (e.key === 'Escape') setOpen(false); };
    document.addEventListener('mousedown', onDown);
    document.addEventListener('keydown', onKey);
    return () => { document.removeEventListener('mousedown', onDown); document.removeEventListener('keydown', onKey); };
  }, [open]);

  const pick = (d) => { onChange(toYMD(d.getFullYear(), d.getMonth(), d.getDate())); setOpen(false); };
  const shiftMonth = (delta) => setView((v) => { const d = new Date(v.y, v.m + delta, 1); return { y: d.getFullYear(), m: d.getMonth() }; });

  return (
    <>
      <button ref={btnRef} type="button" onClick={toggle}
        className={`dp-field${sel ? '' : ' empty'}${open ? ' open' : ''}${className ? ' ' + className : ''}`} style={style}>
        <span>{sel ? displayFmt.format(new Date(sel.y, sel.m, sel.d)) : placeholder}</span>
        <span className="btn icon small subtle"><CalendarDays /></span>
      </button>
      {open && createPortal(
        <div ref={popRef} className="dp-pop" style={{ top: pos.top, left: pos.left }}>
          <div className="dp-head">
            <button type="button" className="dp-title" onClick={() => setJump((j) => !j)}>{monthLabel}<ChevronDown /></button>
            <div className="barh-grp">
              <button type="button" className="btn icon" onClick={() => shiftMonth(-1)}><ChevronUp /></button>
              <button type="button" className="btn icon" onClick={() => shiftMonth(1)}><ChevronDown /></button>
            </div>
          </div>
          {jump ? (
            <>
              <div className="dp-yr">
                <button type="button" className="btn icon" onClick={() => setView((v) => ({ ...v, y: v.y - 1 }))}><ChevronUp /></button>
                <span>{view.y}</span>
                <button type="button" className="btn icon" onClick={() => setView((v) => ({ ...v, y: v.y + 1 }))}><ChevronDown /></button>
              </div>
              <div className="dp-months">
                {monthNames.map((name, m) => (
                  <button type="button" key={m} className={`dp-month${m === view.m ? ' on' : ''}`}
                    onClick={() => { setView((v) => ({ ...v, m })); setJump(false); }}>{name}</button>
                ))}
              </div>
            </>
          ) : (
            <div className="dp-grid">
              {weekdays.map((w, i) => <div key={'w' + i} className="dp-wd">{w}</div>)}
              {cells.map((d, i) => {
                const other = d.getMonth() !== view.m;
                const isSel = sel && same(d, sel.y, sel.m, sel.d);
                const isToday = same(d, now.getFullYear(), now.getMonth(), now.getDate());
                return (
                  <button type="button" key={i} onClick={() => pick(d)}
                    className={`dp-day${other ? ' other' : ''}${isToday ? ' today' : ''}${isSel ? ' on' : ''}`}>{d.getDate()}</button>
                );
              })}
            </div>
          )}
          <div className="dp-foot">
            <button type="button" className="btn subtle" onClick={() => { onChange(''); setOpen(false); }}>{clearLabel}</button>
            <button type="button" className="btn" onClick={() => pick(new Date())}>{todayLabel}</button>
          </div>
        </div>,
        document.body
      )}
    </>
  );
}
