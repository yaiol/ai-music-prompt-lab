// parse-suno.js — self-contained reader for the .suno LP prompt format.
//
// Mirrors the canonical grammar of suno-sync's parse-prompts.mjs (the single
// reader for .suno files). ampl is a packaged Electron app and cannot import
// that module from another app's repo at runtime, so the pure text parser is
// reproduced here. Keep the grammar in step with parse-prompts.mjs:
//   "=== PROMPT NN ==="  -> track boundary, carries the number
//   "KEY:"               -> a field; value is inline after the colon, or on the
//                           following line(s) up to the next KEY:/boundary.
//   LYRICS is always the last field of a track and runs to the next boundary,
//   so its inner blank lines and [bracket] tags are preserved untouched.
//
// parseSuno(text) -> { project, tracks }
//   project: { title, date, style, theme, lang }   (header KEY: values)
//   tracks:  [{ num, title, style, lyrics }, ...]   (in file order)

const BANNER = /^===\s*PROMPT\s+(\d+)\s*===\s*$/i;
const FIELD  = /^(TITLE|STYLE|LYRICS|DATE|THEME|LANG):\s*(.*)$/;

function parseSuno(text) {
  const lines = String(text).split(/\r?\n/);
  const project = {};
  const tracks = [];

  let cur = null;     // current track, or null while in the header block
  let field = null;   // key currently being accumulated (lowercased)
  let buf = [];       // accumulated value lines for `field`

  const flush = () => {
    if (!field) { buf = []; return; }
    const target = cur ?? project;
    if (target[field] === undefined) {
      target[field] = buf.join('\n').replace(/^\n+/, '').replace(/\n+$/, '').trim();
    }
    field = null;
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
    // Once inside LYRICS, nothing but a boundary ends it — so KEY-looking
    // lyric lines are kept as content, not treated as new fields.
    if (field !== 'lyrics') {
      const f = line.match(FIELD);
      if (f) {
        flush();
        field = f[1].toLowerCase();
        buf = f[2] !== '' ? [f[2]] : [];
        continue;
      }
    }
    if (field) buf.push(line);
  }
  flush();

  // Default any unseen fields so consumers never get `undefined`.
  for (const k of ['title', 'date', 'style', 'theme', 'lang']) project[k] ??= '';
  for (const t of tracks) { t.title ??= ''; t.style ??= ''; t.lyrics ??= ''; }

  return { project, tracks };
}

module.exports = { parseSuno };
