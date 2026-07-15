// update-check.js - shared client-side update checker for all Electron apps.
//
// Usage in App.jsx (renderer):
//
//   import { checkForUpdate } from './lib/update-check';
//
//   useEffect(() => {
//     checkForUpdate({ appId: 'markzen', alias: 'mzn', currentVersion: APP_VERSION })
//       .then(update => { if (update) setUpdateAvailable(update); });
//   }, []);
//
// Returns:
//   null          - up to date, offline, or skipped by user
//   { version }   - a newer version is available (the beacon is version-only)
//
// Endpoint: https://apps.yaiol.com/p/<appId>/latest.json (built by build-latest.js).
// The "What's new" / "Download" page URLs are built by updatePageUrl() below -
// the beacon carries no URLs and no notes; those live on the localized pages.
// Skip-this-version is persisted in localStorage as `<alias>-skipUpdate`.

const ENDPOINT = 'https://apps.yaiol.com/p';
const SITE = 'https://apps.yaiol.com';
// Languages the website is actually published in - apps localize into many
// more, but the release pages only exist for these; fall back to en otherwise.
const SITE_LANGS = ['en', 'fr', 'es', 'de'];
const SKIP_KEY = (alias) => `${alias}-skipUpdate`;

function siteLang(lang) {
  const base = String(lang || 'en').toLowerCase().split('-')[0];
  return SITE_LANGS.includes(base) ? base : 'en';
}

// Build the localized page URL a banner button opens. `page` is 'changelog'
// (What's new) or 'download'. Both pages carry identical content by design.
export function updatePageUrl(appId, lang, page) {
  return `${SITE}/${siteLang(lang)}/p/${appId}/${page}.html`;
}

// Compare semver-ish versions ("1.2.3" vs "1.2.10"). Returns >0 if a>b.
function cmpVersion(a, b) {
  const pa = String(a).split('.').map(n => parseInt(n, 10) || 0);
  const pb = String(b).split('.').map(n => parseInt(n, 10) || 0);
  const len = Math.max(pa.length, pb.length);
  for (let i = 0; i < len; i++) {
    const d = (pa[i] || 0) - (pb[i] || 0);
    if (d !== 0) return d;
  }
  return 0;
}

export async function checkForUpdate({ appId, alias, currentVersion, signal } = {}) {
  if (!appId || !alias || !currentVersion) return null;
  try {
    const res = await fetch(`${ENDPOINT}/${appId}/latest.json`, { cache: 'no-cache', signal });
    if (!res.ok) return null;
    const latest = await res.json();
    if (!latest?.version) return null;
    if (cmpVersion(latest.version, currentVersion) <= 0) return null;
    const skipped = (typeof localStorage !== 'undefined') ? localStorage.getItem(SKIP_KEY(alias)) : null;
    if (skipped && cmpVersion(skipped, latest.version) >= 0) return null;
    return latest;
  } catch {
    return null; // offline, DNS down, malformed JSON - silent
  }
}

export function skipUpdate(alias, version) {
  if (typeof localStorage !== 'undefined') {
    localStorage.setItem(SKIP_KEY(alias), version);
  }
}
