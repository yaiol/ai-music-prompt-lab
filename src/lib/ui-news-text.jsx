// ui-news-text.jsx — the mini-Markdown renderer for a yaiol news announcement's text.
// Subset, on purpose: a blank line starts a paragraph, a block whose every line starts with "- " is a
// bullet list, and inline [label](http(s) url), **bold**, *italic*. Anything else is plain text, so a
// stray symbol can never break the dialog. Built as React elements, never as HTML.
//
// Distributed into each app's src/lib/ui-news-text.jsx by sync-shared — ⚠ SYNCED FILE, never edit the
// per-app copy; edit this canonical source and re-sync. Used by ui-news-dialog.jsx.
import React from 'react';

const TOKEN = /\[([^\]\n]+)\]\((https?:\/\/[^\s)]+)\)|\*\*([^*\n]+)\*\*|\*([^*\n]+)\*/g;
const BULLET = /^- (.*)$/;

function inline(text) {
  const out = [];
  let last = 0;
  let m;
  TOKEN.lastIndex = 0;
  while ((m = TOKEN.exec(text)) !== null) {
    if (m.index > last) out.push(text.slice(last, m.index));
    const key = m.index;
    if (m[1] !== undefined) {
      out.push(<a key={key} href={m[2]} onClick={e => { e.preventDefault(); window.open(m[2], '_blank'); }}>{m[1]}</a>);
    } else if (m[3] !== undefined) {
      out.push(<b key={key}>{m[3]}</b>);
    } else {
      out.push(<em key={key}>{m[4]}</em>);
    }
    last = m.index + m[0].length;
  }
  if (last < text.length) out.push(text.slice(last));
  return out;
}

function block(raw, i) {
  const lines = raw.split('\n').map(l => l.trim()).filter(Boolean);
  if (lines.length && lines.every(l => BULLET.test(l))) {
    return <ul key={i} className="dlp-sub left">{lines.map((l, j) => <li key={j}>{inline(l.match(BULLET)[1])}</li>)}</ul>;
  }
  return <div key={i} className="dlp-sub left">{inline(lines.join(' '))}</div>;
}

export function NewsText({ text }) {
  return text.split(/\n{2,}/).filter(p => p.trim()).map(block);
}
