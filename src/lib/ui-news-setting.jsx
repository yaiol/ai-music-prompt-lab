// ui-news-setting.jsx — the "Show news from yaiol" checkbox for a Settings page. It is the SAME per-machine
// switch in every yaiol app (one small file in the shared `yaiol` app-data folder, kept by ./news.mjs), so
// unticking it in one app silences news in all of them. It sits in SettingsView's `generalExtra` slot like
// any other General field.
//
//   generalExtra={<NewsSetting enabled={newsEnabled} onChange={toggleNews} label={t('lblStgGeneralNews')} />}
//
// Controlled: the app owns the value and persists it (POST /news-settings on the app's local server).
//
// Distributed into each app's src/lib/ui-news-setting.jsx by sync-shared — ⚠ SYNCED FILE, never edit the
// per-app copy; edit this canonical source and re-sync.
// Import: `import { NewsSetting } from './lib/ui-news-setting';`
import React from 'react';

export function NewsSetting({ enabled, onChange, label }) {
  return (
    <div className="dlg-field divider">
      <label className="check-label">
        <input type="checkbox" className="check-box" checked={enabled} onChange={e => onChange(e.target.checked)} />
        {label}
      </label>
    </div>
  );
}
