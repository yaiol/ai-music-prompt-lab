// uis-pnl-toggle.jsx — the ONE canonical collapsible-side-panel toggle for every yaiol app.
// The little button welded to a panel's open edge that shows/hides it: accent when the panel is
// open, neutral when hidden, a chevron pointing the collapse direction. Reference: markzen TOC /
// ampl projects sidebar / lrc-editor notepad (all three previously hand-rolled the same inline
// block — this replaces them). Sits between the splitter and the content in the standard pairing
// `[panel] [UisPnlSplitter] [UisPnlToggle] [content]` — it is ONLY the toggle; the UisPnlSplitter (its own
// shared component) and the panel/content layout stay the caller's.
//
// Controlled: pass `open` (bool) + `onToggle()`. `side` = which side the panel is on: 'left'
// (default — button on the panel's right edge, chevron ◀ open / ▶ hidden) or 'right'. `title` is
// the tooltip (already localized). Look lives in uis-app.css (.uis-pnl-toggle*); the component owns
// the top-aligned wrapper so a single drop-in matches every app.
//   <UisPnlToggle open={showToc} onToggle={() => setShowToc(v => !v)} title={t('tipHdrToc')} />
//
// Distributed into each app's src/lib/uis-pnl-toggle.jsx by sync-shared — ⚠ SYNCED FILE,
// never edit the per-app copy; edit this canonical source and re-sync.
// Import: `import { UisPnlToggle } from './lib/uis-pnl-toggle';`
import React from 'react';
import { ChevronLeft, ChevronRight } from 'lucide-react';

export function UisPnlToggle({ open, onToggle, side = 'left', title }) {
  // The chevron points toward the collapse action: a left panel collapses left (◀) when open and
  // expands right (▶) when hidden; a right panel is the mirror.
  const Icon = side === 'left'
    ? (open ? ChevronLeft : ChevronRight)
    : (open ? ChevronRight : ChevronLeft);
  return (
    <div className="uis-pnl-toggle-wrap">
      <button
        type="button"
        onClick={onToggle}
        title={title}
        className={`uis-pnl-toggle uis-pnl-toggle-${side}${open ? ' on' : ''}`}
      >
        <Icon />
      </button>
    </div>
  );
}
