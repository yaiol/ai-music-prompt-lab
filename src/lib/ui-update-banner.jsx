// ui-update-banner.jsx — the ONE canonical "a new version is available" banner for every
// yaiol electron app. Notify-only, dismissible (see app/electron/CLAUDE.md → "Update Check").
// Was copy-pasted into every App.jsx (as .update-banner* markup in the newer apps, inline
// styles in the older ones — which also passed the wrong 'latest' page and 404'd); this is
// the single source. Styling lives in the shared ui-app.css (.update-banner*). Logic (the
// page URLs + skip persistence) lives in ./update-check.
//
// Controlled by the app's update state:
//   info          — the object checkForUpdate() resolved ({ version }); null renders nothing
//   appId         — pkg.name (builds the localized What's-new / Download page URLs)
//   lang          — current UI language (updatePageUrl falls back to a site language)
//   storagePrefix — pkg.storagePrefix (the `<prefix>-skipUpdate` localStorage key)
//   t             — the app's translator; reads the five Prefix:Update keys
//   onClose       — clears the banner (setUpdateInfo(null)); fired on both Skip and Dismiss
//
// Distributed into each app's src/lib/ui-update-banner.jsx by sync-shared — ⚠ SYNCED FILE,
// never edit the per-app copy; edit this canonical source and re-sync.
// Import: `import { UpdateBanner } from './lib/ui-update-banner';`
import React from 'react';
import { updatePageUrl, skipUpdate } from './update-check';

const CloseIcon = () => (
  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor"
    strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
    <line x1="18" y1="6" x2="6" y2="18" /><line x1="6" y1="6" x2="18" y2="18" />
  </svg>
);

export function UpdateBanner({ info, appId, lang, storagePrefix, t, onClose }) {
  if (!info) return null;
  const open = (page) => window.open(updatePageUrl(appId, lang, page), '_blank');
  const skip = () => { skipUpdate(storagePrefix, info.version); onClose(); };
  return (
    <div className="update-banner">
      <div className="update-banner-row">
        <span className="update-banner-label">{t('lblUpdateAvailable')} v{info.version}</span>
        <button className="update-banner-link" onClick={() => open('changelog')}>{t('lnkUpdateWhatsNew')}</button>
        <span className="update-banner-spacer" />
        <button className="update-banner-download" onClick={() => open('download')}>{t('btnUpdateDownload')}</button>
        <button className="update-banner-link skip" onClick={skip}>{t('lnkUpdateSkip')}</button>
        <button className="update-banner-dismiss" onClick={onClose} title={t('tipUpdateDismiss')} aria-label={t('tipUpdateDismiss')}>
          <CloseIcon />
        </button>
      </div>
    </div>
  );
}
