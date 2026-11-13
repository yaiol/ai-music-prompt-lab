// uis-news-loader.jsx — the one-line wiring that makes an app show yaiol news. Once on launch it asks the
// app's own local server (the news routes of ./news.mjs) what this app should show; if there is anything it
// renders the shared UisDlgNews, and when the reader closes it, tells the server which ids to remember as
// "don't show again". Silent on every failure.
//
//   <UisNewsLoader api={API} lang={langKey} t={t} />
//
// Always render it: the server (news.mjs) decides what is shown. A "new version" update comes first and is
// shown whatever the user's "Show news from yaiol" setting says (see uis-news-setting.jsx); that setting is per
// MACHINE, one file shared by every yaiol app, and when off the server returns no announcements and makes no
// announcement request.
//
// TWO REMINDER BUTTONS for the window bar, the last children of UisBarWindow (after settings · help · source),
// in this order. Each draws nothing until the loader's one launch request has found something for it, then
// stays for the whole session - also after the dialog was closed or a card silenced with "don't show again",
// which is their point: a way back. Neither makes a request of its own.
//
// UisBtnUpdate — a newer version is out. A click reopens that update card alone; its tooltip is the card's
// title (t('lblUpdateAvailable') + the version). No newer version known (up to date, offline, no launch
// request) = no button.
//
// UisBtnNews — yaiol has news for this app: at least one announcement that is live today (started, not ended)
// and targeted at it, silenced ones included. A click reopens the announcements; tooltip t('tipNewsShow').
// News switched off (no announcement is requested) or every announcement ended = no button.
//
//   <UisBarWindow …> …settings · help · source… <UisBtnUpdate t={t} /> <UisBtnNews t={t} /> </UisBarWindow>
//
// A card reopened from a button shows its "don't show again" box as it stands (ticked once silenced);
// unticking it brings the launch notice back.
//
// Distributed into each app's src/lib/uis-news-loader.jsx by sync-shared — ⚠ SYNCED FILE, never edit the
// per-app copy; edit this canonical source and re-sync.
// Import: `import { UisNewsLoader, UisBtnUpdate, UisBtnNews } from './lib/uis-news-loader';`
import React, { useEffect, useState, useSyncExternalStore } from 'react';
import { Download, Newspaper } from 'lucide-react';
import { UisDlgNews } from './uis-dlg-news';

// What the launch request learned, kept for the session and shared by the loader (which writes it) and the
// two reminder buttons (which read it): the update card or null, and the live announcements. `reopen` is the
// mounted loader's "show those cards again" ('update' or 'news').
let known = { update: null, news: [] };
let reopen = null;
const watchers = new Set();
const watch = fn => { watchers.add(fn); return () => watchers.delete(fn); };
const setKnown = next => { known = next; watchers.forEach(fn => fn()); };

export function UisNewsLoader({ api, lang, t }) {
  const [items, setItems] = useState(null);

  useEffect(() => {
    let alive = true;
    fetch(`${api}/news?lang=${encodeURIComponent(lang || 'en')}`)
      .then(r => r.json())
      .then(list => {
        if (!alive || !Array.isArray(list)) return;
        setKnown({ update: list.find(it => it?.kind === 'update') || null, news: list.filter(it => it && it.kind !== 'update') });
        // A silenced card is not shown on launch: it is only kept, for its reminder button.
        const show = list.filter(it => !it?.dismissed);
        if (show.length) setItems(show);
      })
      .catch(() => {});
    reopen = what => {
      const cards = what === 'news' ? known.news : (known.update ? [known.update] : []);
      if (cards.length) setItems(cards);
    };
    return () => { alive = false; reopen = null; };
  }, [api]);

  if (!items) return null;
  const close = ({ hideIds }) => {
    // Every card that was shown: its box may have been ticked, or (reopened already silenced) unticked, which
    // brings its launch notice back.
    const restore = items.filter(it => it.dismissed && !hideIds.includes(it.id)).map(it => it.id);
    if (hideIds.length || restore.length) {
      fetch(`${api}/news-dismiss`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ ids: hideIds, restore }) }).catch(() => {});
    }
    const shown = new Set(items.map(it => it.id));
    const flag = it => (shown.has(it.id) && !!it.dismissed !== hideIds.includes(it.id) ? { ...it, dismissed: hideIds.includes(it.id) } : it);
    setKnown({ update: known.update && flag(known.update), news: known.news.map(flag) });
    setItems(null);
  };
  return <UisDlgNews items={items} lang={lang} t={t} onClose={close} />;
}

export function UisBtnUpdate({ t }) {
  const update = useSyncExternalStore(watch, () => known.update);
  if (!update) return null;
  const tip = `${t('lblUpdateAvailable')} v${update.version}`;
  return (
    <button className="uis-barh-window-btn update" title={tip} aria-label={tip} onClick={() => reopen?.('update')}>
      <Download />
    </button>
  );
}

export function UisBtnNews({ t }) {
  const count = useSyncExternalStore(watch, () => known.news.length);
  if (!count) return null;
  const tip = t('tipNewsShow');
  return (
    <button className="uis-barh-window-btn" title={tip} aria-label={tip} onClick={() => reopen?.('news')}>
      <Newspaper />
    </button>
  );
}
