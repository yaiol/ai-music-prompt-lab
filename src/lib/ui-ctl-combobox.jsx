// ui-ctl-combobox.jsx — the ONE typeahead / autocomplete control for every yaiol electron app.
// A text field + a filtered dropdown list. It replaces the hand-rolled `open && <div style={{position:
// absolute…}}>` pattern that reinvented itself ~7 times in ampl (language pickers, tag autocompletes,
// artist / name suggestions) — each of which CLIPPED inside a scrolling dialog. The list rides the
// shared `Popover` (portal + position + flip + click-away), so it is never clipped, and looks
// identical to `Menu` / `Switcher` — they share `.pop-surface` (panel) + `.pop-item` (row).
//
// The control owns the INPUT + open/close + KEYBOARD NAV (↑/↓ move the highlight, Enter picks the
// highlighted row); the CONSUMER owns the DATA — it pre-filters `items` by `value` and says how to
// render / key / pick each one. (Content differs per use — languages vs tags vs names — but the
// control is one.) Enter with no row highlighted falls through to the consumer's `onKeyDown` (submit).
//
// Controlled input: `value` + `onChange(text)`. Optional `onOpenChange(open)` for a consumer that must
// show a different value while open (a picker showing the selected label when closed, the search text
// when open). `renderHeader({close})` adds a fixed top row (e.g. a "clear selection" row).
//
// Distributed into each app's src/lib by sync-shared — ⚠ SYNCED FILE, never edit the per-app copy;
// edit this canonical source and re-sync. Import: `import { Combobox } from './lib/ui-ctl-combobox';`
import React, { useState, useRef, useCallback } from 'react';
import { Popover } from './ui-ctl-popover';

export function Combobox({
  value, onChange, onFocus, onBlur, onOpenChange, onKeyDown,
  placeholder = '', autoFocus = false, inputClassName = 'input', inputStyle,
  items = [], itemKey = (_it, i) => i, renderItem = (it) => it, itemActive = () => false,
  onPick, renderHeader,
}) {
  const [open, setOpenRaw] = useState(false);
  const [hi, setHi] = useState(-1);   // keyboard-highlighted row index (-1 = none)
  const inputRef = useRef(null);
  const setOpen = useCallback((v) => { setOpenRaw(v); onOpenChange?.(v); if (!v) setHi(-1); }, [onOpenChange]);
  const close = useCallback(() => setOpen(false), [setOpen]);
  const showList = open && (items.length > 0 || !!renderHeader);

  const handleKeyDown = (e) => {
    if (open && items.length) {
      if (e.key === 'ArrowDown') { e.preventDefault(); setHi(h => Math.min(h + 1, items.length - 1)); return; }
      if (e.key === 'ArrowUp')   { e.preventDefault(); setHi(h => Math.max(h - 1, -1)); return; }
      if (e.key === 'Enter' && hi >= 0 && hi < items.length) { e.preventDefault(); onPick?.(items[hi]); close(); return; }
    }
    onKeyDown?.(e);   // Enter with no highlight (and every other key) is the consumer's
  };

  return (
    <>
      <input
        ref={inputRef}
        className={inputClassName}
        style={inputStyle}
        placeholder={placeholder}
        autoFocus={autoFocus}
        value={value}
        onChange={(e) => { onChange?.(e.target.value); setHi(-1); if (!open) setOpen(true); }}
        onFocus={(e) => { setOpen(true); onFocus?.(e); }}
        onBlur={(e) => { onBlur?.(e); }}   /* click-away/pick (via Popover) handle close; blur must not race the pick */
        onKeyDown={handleKeyDown}
      />
      <Popover anchorRef={inputRef} open={showList} onClose={close} minWidthAnchor maxHeight={220}>
        {renderHeader?.({ close })}
        {items.map((it, i) => (
          <button
            type="button"
            key={itemKey(it, i)}
            ref={i === hi ? (el) => el?.scrollIntoView({ block: 'nearest' }) : null}
            className={`pop-item${itemActive(it) ? ' active' : ''}${i === hi ? ' hi' : ''}`}
            onMouseEnter={() => setHi(i)}
            onMouseDown={(e) => { e.preventDefault(); onPick?.(it); close(); }}
          >
            {renderItem(it)}
          </button>
        ))}
      </Popover>
    </>
  );
}
