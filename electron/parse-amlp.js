// parse-amlp.js — self-contained reader for the .amlp LP prompt format.
//
// Mirrors the canonical grammar of suno-sync's parse-prompts.mjs (the single
// reader for .amlp files). ampl is a packaged Electron app and cannot import
// that module from another app's repo at runtime, so the pure text parser is
// reproduced here. Keep the grammar in step with parse-prompts.mjs:
//   "=== PROMPT NN ==="  -> track boundary, carries the number
//   "KEY:"               -> a field; value is inline after the colon, or on the
//                           following line(s) up to the next KEY:/boundary.
//   "KEY (lang):"        -> a TRANSLATION field (TITLE and LYRICS only), collected
//                           per language into the track's `translations` list.
//   LYRICS is the last ORIGINAL field of a track: it runs to the next translation
//   key or the next boundary, so its inner blank lines and [bracket] tags are
//   preserved untouched. "LYRICS (lang)" runs to the next translation key/boundary.
//
// parseAmlp(text) -> { project, tracks }
//   project: { title, date, style, theme, lang }   (header KEY: values)
//   tracks:  [{ num, title, description, style, url, lang, referenceArtist,
//              referenceComplement, lyrics, translations: [{ lang, title, lyrics }] }, ...]

const BANNER = /^===\s*PROMPT\s+(\d+)\s*===\s*$/i;
const FIELD  = /^(TITLE|DESCRIPTION|STYLE|URL|LYRICS|DATE|THEME|LANG|REFERENCE ARTIST|REFERENCE COMPLEMENT):\s*(.*)$/;
// "TITLE (en):" / "LYRICS (pt-BR):" — the tag is a song-language code (2-3 letters,
// optional region), a shape no real lyric line collides with. That is what lets a
// translation key close a LYRICS block when a bare "TITLE:" deliberately cannot.
const TRANS  = /^(TITLE|LYRICS)\s*\(\s*([A-Za-z]{2,3}(?:-[A-Za-z]{2,4})?)\s*\)\s*:\s*(.*)$/;

// Field key -> object property (multi-word keys become camelCase).
const KEYS = { 'reference artist': 'referenceArtist', 'reference complement': 'referenceComplement' };

// "EN" -> "en", "pt-br" -> "pt-BR": the canonical casing of a language code.
function normLang(tag) {
  const [base, region] = tag.split('-');
  return region ? `${base.toLowerCase()}-${region.toUpperCase()}` : base.toLowerCase();
}

function parseAmlp(text) {
  const lines = String(text).split(/\r?\n/);
  const project = {};
  const tracks = [];

  let cur = null;     // current track, or null while in the header block
  let field = null;   // property currently being accumulated
  let trLang = null;  // language code when `field` belongs to a translation, else null
  let buf = [];       // accumulated value lines for `field`

  const flush = () => {
    if (!field) { buf = []; return; }
    const value = buf.join('\n').replace(/^\n+/, '').replace(/\n+$/, '').trim();
    const target = cur ?? project;
    if (trLang) {
      const list = (target.translations ??= []);
      let tr = list.find(t => t.lang === trLang);
      if (!tr) { tr = { lang: trLang, title: '', lyrics: '' }; list.push(tr); }
      if (!tr[field]) tr[field] = value;
    } else if (target[field] === undefined) {
      target[field] = value;
    }
    field = null;
    trLang = null;
    buf = [];
  };

  for (const line of lines) {
    const b = line.match(BANNER);
    if (b) {
      flush();
      cur = { num: parseInt(b[1], 10) };
      tracks.push(cur);
      continue;
    }
    // A translation key ends whatever came before it, LYRICS included.
    const tr = line.match(TRANS);
    if (tr) {
      flush();
      field = tr[1].toLowerCase();
      trLang = normLang(tr[2]);
      buf = tr[3] !== '' ? [tr[3]] : [];
      continue;
    }
    // Once inside a lyrics block, nothing but a boundary or a translation key
    // ends it — so KEY-looking lyric lines are kept as content, not new fields.
    if (field !== 'lyrics') {
      const f = line.match(FIELD);
      if (f) {
        flush();
        const key = f[1].toLowerCase();
        field = KEYS[key] || key;
        buf = f[2] !== '' ? [f[2]] : [];
        continue;
      }
    }
    if (field) buf.push(line);
  }
  flush();

  // Default any unseen fields so consumers never get `undefined`.
  for (const k of ['title', 'date', 'style', 'theme', 'lang']) project[k] ??= '';
  for (const t of tracks) {
    t.title ??= ''; t.description ??= ''; t.style ??= ''; t.url ??= ''; t.lyrics ??= '';
    t.lang ??= ''; t.referenceArtist ??= ''; t.referenceComplement ??= ''; t.translations ??= [];
  }

  return { project, tracks };
}

module.exports = { parseAmlp };
