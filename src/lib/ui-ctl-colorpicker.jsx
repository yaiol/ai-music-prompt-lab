// ui-ctl-colorpicker.jsx — the ONE canonical colour picker for every yaiol electron app.
// A swatch button that opens a draggable popup: a saturation/hue field (react-colorful),
// an eyedropper (when the platform supports it), a hex input, R/G/B NumberFields, and a
// row of quick swatches, with Cancel / Apply. Reference look: ai-music-prompt-lab (this was
// lifted from ampl's private ColorPickerPopup — the canonical). Whenever an app lets the user
// choose a colour (tag colours, card/type colours, a label), it uses THIS — never a native
// `<input type="color">` (icoc's old path) nor a re-rolled popup.
//
// Controlled: pass `color` (hex string) + `onChange(hex)` — onChange fires on Apply only, so
// the caller commits once. Labels are optional props (default English) so the component carries
// no i18n key coupling; pass your app's localized strings:
//   <ColorPicker color={c} onChange={setC}
//     cancelLabel={t('btnGlbCancel')} applyLabel={t('btnGlbApply')} pickTitle={t('tipGlbPickFromScreen')} />
//
// The popup portals to <body> (so it escapes any overflow:hidden ancestor), flips above the
// button when there's no room below, and is drag-repositionable by its grip. It borrows the
// shared field classes (.dlg-form / .input / .btn.icon / .btn) and the shared NumberField.
//
// Distributed into each app's src/lib/ui-ctl-colorpicker.jsx by sync-shared — ⚠ SYNCED FILE,
// never edit the per-app copy; edit this canonical source and re-sync. Requires `react-colorful`
// (devDependency) in the consuming app. Import: `import { ColorPicker } from './lib/ui-ctl-colorpicker';`
import React, { useState, useEffect, useRef } from 'react';
import { createPortal } from 'react-dom';
import { HexColorPicker } from 'react-colorful';
import { GripHorizontal, Pipette } from 'lucide-react';
import { NumberField } from './ui-ctl-numberfield';

const COLOR_SWATCHES = ['#ff6eb4', '#f0c040', '#60a5fa', '#7c6fff', '#4dc8c8', '#5dba6f', '#e06060'];
const hexToRgb = (h) => ({ r: parseInt(h.slice(1, 3), 16), g: parseInt(h.slice(3, 5), 16), b: parseInt(h.slice(5, 7), 16) });
const rgbToHex = (r, g, b) => '#' + [r, g, b].map((v) => Math.min(255, Math.max(0, v)).toString(16).padStart(2, '0')).join('');

export function ColorPicker({ color, onChange, cancelLabel = 'Cancel', applyLabel = 'Apply', pickTitle = 'Pick colour from screen' }) {
  const [open, setOpen] = useState(false);
  const [draft, setDraft] = useState(color);
  const [hex, setHex] = useState(color);
  const [popPos, setPopPos] = useState({ top: 0, left: 0 });
  const btnRef = useRef(null);
  const popRef = useRef(null);
  const draggingRef = useRef(false);
  const dragOffset = useRef({ x: 0, y: 0 });
  const handleToggle = () => {
    if (!open && btnRef.current) {
      const r = btnRef.current.getBoundingClientRect();
      const POPUP_H = 400;
      const goUp = r.bottom + POPUP_H > window.innerHeight;
      setPopPos(goUp
        ? { top: r.top - POPUP_H - 6, left: r.left }
        : { top: r.bottom + 6, left: r.left });
      setDraft(color); setHex(color);
    }
    setOpen((v) => !v);
  };
  const handleApply = () => { onChange(draft); setOpen(false); };
  const handleCancel = () => setOpen(false);
  const handleDragStart = (e) => {
    e.preventDefault();
    if (!popRef.current) return;
    const rect = popRef.current.getBoundingClientRect();
    setPopPos({ top: rect.top, left: rect.left });
    dragOffset.current = { x: e.clientX - rect.left, y: e.clientY - rect.top };
    draggingRef.current = true;
    document.body.style.cursor = 'grabbing';
    document.body.style.userSelect = 'none';
  };
  useEffect(() => {
    if (!open) return;
    const onMove = (e) => {
      if (!draggingRef.current) return;
      setPopPos({ top: e.clientY - dragOffset.current.y, left: e.clientX - dragOffset.current.x });
    };
    const onUp = () => {
      if (draggingRef.current) {
        draggingRef.current = false;
        document.body.style.cursor = '';
        document.body.style.userSelect = '';
      }
    };
    const onDown = (e) => {
      if (draggingRef.current) return;
      if (btnRef.current?.contains(e.target)) return;
      if (!popRef.current?.contains(e.target)) setOpen(false);
    };
    document.addEventListener('mousedown', onDown);
    document.addEventListener('mousemove', onMove);
    document.addEventListener('mouseup', onUp);
    return () => {
      document.removeEventListener('mousedown', onDown);
      document.removeEventListener('mousemove', onMove);
      document.removeEventListener('mouseup', onUp);
    };
  }, [open]);
  const updateDraft = (c) => { setDraft(c); setHex(c); };
  const rgb = /^#[0-9a-fA-F]{6}$/.test(draft) ? hexToRgb(draft) : { r: 0, g: 0, b: 0 };
  const handleRgb = (ch, val) => {
    const n = parseInt(val); if (isNaN(n)) return;
    const nc = { ...rgb, [ch]: Math.min(255, Math.max(0, n)) };
    updateDraft(rgbToHex(nc.r, nc.g, nc.b));
  };
  const handleEyedropper = async () => {
    if (!window.EyeDropper) return;
    try { const res = await new window.EyeDropper().open(); updateDraft(res.sRGBHex); } catch {}
  };
  return (
    <div style={{ display: 'inline-block', flexShrink: 0 }}>
      <button ref={btnRef} onClick={handleToggle} className={"swatch" + (open ? " on" : "")} style={{ background: color, boxShadow: open ? `0 0 0 2px ${color}` : undefined }} />
      {open && createPortal(
        <div ref={popRef} className="dlg-form" style={{ position: 'fixed', zIndex: 9999, ...popPos, background: 'var(--dlg-bgd)', border: '1px solid var(--border)', borderRadius: 10, boxShadow: '0 8px 32px #0008', display: 'flex', flexDirection: 'column', width: 220, overflow: 'hidden' }}>
          <div onMouseDown={handleDragStart} style={{ cursor: 'grab', display: 'flex', alignItems: 'center', justifyContent: 'center', padding: '6px 0 4px', borderBottom: '1px solid var(--border)', color: 'var(--text-dim)', flexShrink: 0 }}>
            <GripHorizontal className="icon-inline" />
          </div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 10, padding: 12 }}>
            <HexColorPicker color={draft} onChange={updateDraft} style={{ width: '100%' }} />
            <div style={{ display: 'flex', gap: 6, alignItems: 'center' }}>
              {window.EyeDropper && <button className="btn icon" onClick={handleEyedropper} title={pickTitle}><Pipette /></button>}
              <input value={hex} maxLength={7}
                onChange={(e) => { const v = e.target.value; setHex(v); if (/^#[0-9a-fA-F]{6}$/.test(v)) setDraft(v); }}
                onBlur={() => { if (!/^#[0-9a-fA-F]{6}$/.test(hex)) setHex(draft); }}
                className="input" style={{ flex: 1, fontFamily: 'monospace' }} />
            </div>
            <div style={{ display: 'flex', gap: 6, justifyContent: 'space-between' }}>
              {['r', 'g', 'b'].map((ch) => (
                <div key={ch} style={{ display: 'flex', flexDirection: 'column', gap: 2, alignItems: 'center' }}>
                  <NumberField min={0} max={255} value={rgb[ch]} onChange={(v) => handleRgb(ch, v)} width={56} />
                  <span style={{ fontSize: 10, color: 'var(--text-dim)', textTransform: 'uppercase' }}>{ch}</span>
                </div>
              ))}
            </div>
            <div style={{ display: 'flex', flexWrap: 'wrap', gap: 5 }}>
              {COLOR_SWATCHES.map((c) => (
                <button key={c} onClick={() => updateDraft(c)} className={"swatch small" + (draft === c ? " on" : "")} style={{ background: c, boxShadow: draft === c ? `0 0 0 2px ${c}` : undefined }} />
              ))}
            </div>
            <div style={{ display: 'flex', gap: 8 }}>
              <button className="btn subtle" onClick={handleCancel} style={{ flex: 1 }}>{cancelLabel}</button>
              <button className="btn primary" onClick={handleApply} style={{ flex: 1 }}>{applyLabel}</button>
            </div>
          </div>
        </div>,
        document.body
      )}
    </div>
  );
}
