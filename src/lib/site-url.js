// site-url.js - the localized yaiol site address of a product page, for every Electron app.
//
//   import { getUrl } from './lib/site-url';
//   getUrl(pkg.name, lang.replace(/_/g, '-'), 'help')   // https://apps.yaiol.com/<lang>/p/<id>/help/
//
// A new version is NOT checked here: the news reader (news.mjs) shows it as the first card of the news dialog.

const SITE = 'https://apps.yaiol.com';

export function getUrl(appId, lang, page = '') {
  const path = String(page).split('/').filter(Boolean).join('/');
  return `${SITE}/${lang}/p/${appId}/${path ? path + '/' : ''}`;
}
