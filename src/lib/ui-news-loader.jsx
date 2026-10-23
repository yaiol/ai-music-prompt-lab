// ui-news-loader.jsx — the one-line wiring that makes an app show yaiol news. Once on launch it asks the
// app's own local server (the news routes of ./news.mjs) what this app should show; if there is anything it
// renders the shared NewsDialog, and when the reader closes it, tells the server which ids to remember as
// "don't show again". Silent on every failure.
//
//   <NewsLoader api={API} lang={langKey} t={t} />
//
// Always render it: the server (news.mjs) decides what is shown. A "new version" update comes first and is
// shown whatever the user's "Show news from yaiol" setting says (see ui-news-setting.jsx); that setting is per
// MACHINE, one file shared by every yaiol app, and when off the server returns no announcements and makes no
// announcement request.
//
// Distributed into each app's src/lib/ui-news-loader.jsx by sync-shared — ⚠ SYNCED FILE, never edit the
// per-app copy; edit this canonical source and re-sync.
// Import: `import { NewsLoader } from './lib/ui-news-loader';`
import React, { useEffect, useState } from 'react';
import { NewsDialog } from './ui-news-dialog';

export function NewsLoader({ api, lang, t }) {
  const [items, setItems] = useState(null);

  useEffect(() => {
    let alive = true;
    fetch(`${api}/news?lang=${encodeURIComponent(lang || 'en')}`)
      .then(r => r.json())
      .then(list => { if (alive && Array.isArray(list) && list.length) setItems(list); })
      .catch(() => {});
    return () => { alive = false; };
  }, [api]);

  if (!items) return null;
  const close = ({ hideIds }) => {
    if (hideIds.length) {
      fetch(`${api}/news-dismiss`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ ids: hideIds }) }).catch(() => {});
    }
    setItems(null);
  };
  return <NewsDialog items={items} lang={lang} t={t} onClose={close} />;
}
