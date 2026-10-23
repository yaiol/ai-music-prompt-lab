// ui-news-dialog.jsx — the ONE pop-up that shows yaiol news, in every app that shows news (and in the
// cockpit's news preview, so what the author sees is what the user gets).
//
// Controlled and fetch-free:
//   items   — the announcements to show. Several turn as a carousel: by itself (about reading time: 5-15 s each), until the reader
//             clicks a number / arrow (or ticks the box), and the dialog keeps one size throughout.
//             Each is { id, lang, link? }, or ONE UPDATE card { id, kind: 'update', version, text, link } (the latest version and the notes of
//             every release newer than the one running, see news.mjs): its title is t('lblUpdateAvailable') + the version, its text is
//             the release notes, its button t('btnUpdateDownload').
//             For an announcement `lang` is { <language>: { title, subtitle?, text?, image? } }, resolved HERE
//             field by field by lang → base language → en, the picture included. An `image` is an https://
//             address, a data:image URL, or a bare file name (read from apps.yaiol.com/news/); anything else is ignored
//   lang    — the app's UI language (`pt_BR` or `pt-BR`)
//   t       — the app's translator; reads lblNewsHideForever, btnNewsOpenLink, btnNewsClose, tipNewsDismiss, and for an update
//             lblUpdateAvailable, btnUpdateDownload
//   onClose — fired by the X, Esc, a click outside and Close; receives { hideIds }: the ids whose
//             "don't show again" box was ticked (the box belongs to the card being shown)
// An item whose title cannot be resolved is skipped; with none left the dialog renders nothing.
//
// Distributed into each app's src/lib/ui-news-dialog.jsx by sync-shared — ⚠ SYNCED FILE, never edit the
// per-app copy; edit this canonical source and re-sync.
// Import: `import { NewsDialog } from './lib/ui-news-dialog';`
import React, { useState, useEffect } from 'react';
import { X, ChevronLeft, ChevronRight } from 'lucide-react';
import yaiolLogo from '../assets/yaiol-logo.svg';
import { NewsText } from './ui-news-text';

// One field (title / subtitle / text / image) of an announcement in the reader's language: that language's own block, else its
// base language's, else English's - per FIELD, so a language with a title but no text shows the English text (and a language with no picture of its own shows the English one).
export function pickNewsField(langs, field, lang) {
  if (!langs || typeof langs !== 'object') return '';
  const code = String(lang || '').replace(/_/g, '-');
  for (const k of [code, code.split('-')[0], 'en']) {
    const v = langs[k]?.[field];
    if (typeof v === 'string' && v) return v;
  }
  return '';
}

const NEWS_IMAGES = 'https://apps.yaiol.com/news/';

// The address an `image` value stands for, or '' when it is not one we show.
export function newsImageUrl(raw) {
  const v = String(raw || '').trim();
  if (/^https:\/\//i.test(v) || /^data:image\//i.test(v)) return v;
  return /^[A-Za-z0-9][A-Za-z0-9._/-]*$/.test(v) && !v.includes('..') ? NEWS_IMAGES + v : '';
}

// How long an announcement stays: 4 s plus 1 s per 3 words (about reading speed), kept between 5 s and 15 s.
function readMs(read) {
  const words = ['title', 'subtitle', 'text'].map(read).join(' ').split(/\s+/).filter(Boolean).length;
  return Math.max(5000, Math.min(15000, 4000 + Math.round(words / 3) * 1000));
}

export function NewsDialog({ items, lang, t, onClose }) {
  const [index, setIndex] = useState(0);
  const [auto, setAuto] = useState(true);
  const [hidden, setHidden] = useState(() => new Set());
  const [failed, setFailed] = useState(() => new Set());   // images that did not load: the box stays, empty
  // One field of a card: an update card's title and text are built from its version and notes, an announcement's come from its languages.
  const field = (it, f) => it.kind === 'update'
    ? (f === 'title' ? `${t('lblUpdateAvailable')} v${it.version}` : f === 'text' ? it.text || '' : '')
    : pickNewsField(it.lang, f, lang);
  const list = (items || []).filter(it => it && field(it, 'title'));

  // The carousel turns by itself, each announcement staying as long as it takes to read, until the reader
  // takes over (a click on a number or an arrow, an arrow key, ticking the box) - after that it stays
  // where they put it.
  const current = Math.min(index, Math.max(list.length - 1, 0));
  useEffect(() => {
    if (!auto || list.length < 2) return undefined;
    const timer = setTimeout(() => setIndex((current + 1) % list.length), readMs(f => field(list[current], f)));
    return () => clearTimeout(timer);
  }, [auto, current, list.length, lang]); // eslint-disable-line react-hooks/exhaustive-deps

  if (!list.length) return null;
  const at = Math.min(index, list.length - 1);
  const item = list[at];
  const close = () => onClose({ hideIds: [...hidden] });
  const choose = (i) => { setAuto(false); setIndex((i + list.length) % list.length); };
  const toggleHidden = (on) => { setAuto(false); setHidden(prev => { const next = new Set(prev); if (on) next.add(item.id); else next.delete(item.id); return next; }); };
  return (
    <div className="dl-backdrop" style={{ zIndex: 900 }} onMouseDown={e => { if (e.target === e.currentTarget) close(); }}>
      <div className="dlp wide" onKeyDown={e => {
        if (e.key === 'Escape') close();
        else if (e.key === 'ArrowLeft') choose(at - 1);
        else if (e.key === 'ArrowRight') choose(at + 1);
      }}>
        <button className="dl-close" onClick={close} title={t('tipNewsDismiss')} aria-label={t('tipNewsDismiss')}><X /></button>
        <img className="dlp-icon" src={yaiolLogo} alt="yaiol" />
        {/* Every announcement is laid out in the SAME grid cell and only the current one is visible, so
            the dialog is as tall as the tallest one and never resizes when the carousel turns. */}
        <div className="dlp-pages">
          {list.map((it, i) => {
            const subtitle = field(it, 'subtitle');
            const image = newsImageUrl(field(it, 'image'));
            return (
              <div key={it.id} className={`dlp-page${i === at ? '' : ' dlp-page-off'}`}>
                <div className="dlp-title">{field(it, 'title')}</div>
                {subtitle && <div className="dlp-subtitle">{subtitle}</div>}
                {image && (
                  <div className="dlp-image">
                    {!failed.has(it.id) && <img src={image} alt="" onError={() => setFailed(prev => new Set(prev).add(it.id))} />}
                  </div>
                )}
                <div className="dlp-text"><NewsText text={field(it, 'text')} /></div>
              </div>
            );
          })}
        </div>
        {list.length > 1 && (
          <div className="dlp-nav">
            <button className="btn icon subtle" onClick={() => choose(at - 1)}><ChevronLeft /></button>
            {list.map((it, i) => (
              <button key={it.id} className={`chip${i === at ? ' on' : ''}`} onClick={() => choose(i)}>{i + 1}</button>
            ))}
            <button className="btn icon subtle" onClick={() => choose(at + 1)}><ChevronRight /></button>
          </div>
        )}
        <div className="dlp-foot">
          <label className="check-label">
            <input type="checkbox" className="check-box" checked={hidden.has(item.id)} onChange={e => toggleHidden(e.target.checked)} />
            {t('lblNewsHideForever')}
          </label>
          {item.link && <button className="btn primary" autoFocus onClick={() => window.open(item.link, '_blank')}>{item.kind === 'update' ? t('btnUpdateDownload') : t('btnNewsOpenLink')}</button>}
          <button className="btn subtle" autoFocus={!item.link} onClick={close}>{t('btnNewsClose')}</button>
        </div>
      </div>
    </div>
  );
}
