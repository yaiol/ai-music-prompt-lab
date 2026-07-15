// Shared language-name resolver for the backend (CommonJS).
// Reads SONG_LANGUAGES and LANG_NAMES from src/languages.js at startup.
const fs = require('fs');
const path = require('path');

let SONG_LANGUAGES = [];
let LANG_NAMES = {};

try {
  const src = fs.readFileSync(path.join(__dirname, '..', 'src', 'languages.js'), 'utf8');
  // Strip ES module syntax and evaluate as plain JS
  const cleaned = src
    .replace(/export const /g, 'const ')
    .replace(/export function /g, 'function ');
  // Use Function constructor to evaluate in a clean scope
  const fn = new Function(cleaned + '\nreturn { SONG_LANGUAGES, LANG_NAMES };');
  const result = fn();
  SONG_LANGUAGES = result.SONG_LANGUAGES || [];
  LANG_NAMES = result.LANG_NAMES || {};
} catch (err) {
  console.warn('lang-helper: failed to load languages.js -', err.message);
}

function getLangName(code, uiLang = 'en') {
  const localized = LANG_NAMES[uiLang]?.[code];
  if (localized) return localized;
  return SONG_LANGUAGES.find(l => l.code === code)?.name || code;
}

module.exports = { getLangName, SONG_LANGUAGES, LANG_NAMES };
