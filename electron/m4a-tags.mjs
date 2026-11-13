// m4a-tags.mjs — read and write the text tags of an m4a (MP4 audio) file, with no external tool.
//
// The m4a twin of flac-tags.mjs, for the same reason: the app writes one tag (YAIOL_AIID) and may
// clear another (the comment), and that does not justify shipping or requiring ffmpeg. The audio
// is never decoded or re-encoded: every byte outside the one rewritten box is copied through.
//
// MP4 layout: a tree of boxes, each
//   4 bytes : size of the whole box, BIG-endian (1 = a 64-bit size follows the type; 0 = to end of file)
//   4 bytes : type
//   N bytes : payload (child boxes, for a container)
// The tags live in  moov > udta > meta > ilst.  Each child of ilst is one tag:
//   "©cmt" > "data"                      the comment (0xA9 "cmt": the type is NOT ASCII)
//   "----" > "mean" + "name" + "data"    a free-form tag; mean = "com.apple.iTunes", name = the tag's name
// A "data" payload is: 4 bytes type (1 = UTF-8 text), 4 bytes locale, then the value.
//
// ⚠ CLAUDE: the audio's position is written INSIDE moov. Each track's "stco" (or "co64") table
// holds absolute file offsets of its audio chunks. When moov sits before the audio and changes
// size, every one of those offsets must move by the same amount, or the file still opens, shows
// its tags, and plays noise or nothing. That is what shiftChunkOffsets() is for, and why a write
// is checked by reading audio bytes back through the new offsets before the file is replaced.

import fs from "fs";
import path from "path";
import crypto from "crypto";

const MEAN = "com.apple.iTunes";
const COMMENT_TYPE = "©cmt";
// Field names the callers use that are a standard atom rather than a free-form tag.
const STANDARD = { comment: COMMENT_TYPE };

// The boxes directly inside buf[start, end). Null when the bytes are not a well-formed box list.
function parseBoxes(buf, start, end) {
  const boxes = [];
  let pos = start;
  while (pos < end) {
    if (pos + 8 > end) return null;
    let size = buf.readUInt32BE(pos);
    let header = 8;
    if (size === 1) {
      if (pos + 16 > end) return null;
      const big = buf.readBigUInt64BE(pos + 8);
      if (big > BigInt(Number.MAX_SAFE_INTEGER)) return null;
      size = Number(big);
      header = 16;
    } else if (size === 0) {
      size = end - pos;
    }
    if (size < header || pos + size > end) return null;
    boxes.push({ type: buf.toString("latin1", pos + 4, pos + 8), start: pos, end: pos + size, body: pos + header });
    pos += size;
  }
  return boxes;
}

function box(type, ...parts) {
  const payload = Buffer.concat(parts);
  const head = Buffer.alloc(8);
  head.writeUInt32BE(payload.length + 8, 0);
  head.write(type, 4, "latin1");
  return Buffer.concat([head, payload]);
}

const fullText = (type, text) => box(type, Buffer.alloc(4), Buffer.from(text, "utf8"));
const dataBox = (text) => box("data", Buffer.from([0, 0, 0, 1, 0, 0, 0, 0]), Buffer.from(text, "utf8"));

// Where the tags are. `meta` carries 4 bytes of version/flags before its children in an m4a
// (QuickTime's own form has none); which one this file uses is read off where "hdlr" sits.
function locate(buf) {
  const top = parseBoxes(buf, 0, buf.length);
  if (!top || !top.some(b => b.type === "ftyp")) return null;
  const moov = top.find(b => b.type === "moov");
  if (!moov) return null;
  const moovKids = parseBoxes(buf, moov.body, moov.end);
  if (!moovKids) return null;
  const found = { top, moov, moovKids, udta: null, udtaKids: [], meta: null, metaKids: [], metaPrefix: 4, ilst: null, items: [] };
  found.udta = moovKids.find(b => b.type === "udta") || null;
  if (!found.udta) return found;
  found.udtaKids = parseBoxes(buf, found.udta.body, found.udta.end);
  if (!found.udtaKids) return null;
  found.meta = found.udtaKids.find(b => b.type === "meta") || null;
  if (!found.meta) return found;
  const m = found.meta;
  if (buf.toString("latin1", m.body + 8, m.body + 12) === "hdlr") found.metaPrefix = 4;
  else if (buf.toString("latin1", m.body + 4, m.body + 8) === "hdlr") found.metaPrefix = 0;
  else return null;
  found.metaKids = parseBoxes(buf, m.body + found.metaPrefix, m.end);
  if (!found.metaKids) return null;
  found.ilst = found.metaKids.find(b => b.type === "ilst") || null;
  if (!found.ilst) return found;
  found.items = parseBoxes(buf, found.ilst.body, found.ilst.end);
  return found.items ? found : null;
}

// One ilst item as { key, value } — key is the free-form name, or the atom type for a standard
// tag. Null for an item that holds no text (the cover, the track number).
function decodeItem(buf, item) {
  const kids = parseBoxes(buf, item.body, item.end);
  if (!kids) return null;
  const data = kids.find(k => k.type === "data");
  if (!data || data.end - data.body < 8 || buf.readUInt32BE(data.body) !== 1) return null;
  const value = buf.toString("utf8", data.body + 8, data.end);
  if (item.type !== "----") return { key: item.type, value };
  const name = kids.find(k => k.type === "name");
  if (!name || name.end - name.body < 4) return null;
  return { key: buf.toString("utf8", name.body + 4, name.end), value };
}

// The key a caller's field name is stored under in this container.
const storedKey = (name) => STANDARD[name.toLowerCase()] || name;
const sameKey = (a, b) => a.toLowerCase() === b.toLowerCase();

/** Read the text tags of an m4a. Returns null if it is not one. Same shape as readFlacFields. */
export function readM4aFields(filePath) {
  let buf;
  try { buf = fs.readFileSync(filePath); } catch { return null; }
  let at;
  try { at = locate(buf); } catch { return null; }
  if (!at) return null;
  const fields = [];
  for (const item of at.items) {
    const f = decodeItem(buf, item);
    if (f) fields.push(f);
  }
  return { fields };
}

/** One field's value, "" when absent. `key` is the caller's name: "COMMENT", "YAIOL_AIID". */
export function getM4aField(tags, key) {
  if (!tags) return "";
  const hit = tags.fields.find(f => sameKey(f.key, storedKey(key)));
  return hit ? hit.value : "";
}

// Every chunk-offset table inside a rebuilt moov: entries pointing past `after` move by `delta`.
// Returns false when a 32-bit table cannot hold the moved offset.
function shiftChunkOffsets(moov, delta, after) {
  const walk = (start, end) => {
    const kids = parseBoxes(moov, start, end);
    if (!kids) return false;
    for (const k of kids) {
      if (["trak", "mdia", "minf", "stbl"].includes(k.type)) {
        if (!walk(k.body, k.end)) return false;
      } else if (k.type === "stco" || k.type === "co64") {
        const count = moov.readUInt32BE(k.body + 4);
        const wide = k.type === "co64";
        for (let i = 0, p = k.body + 8; i < count; i++, p += wide ? 8 : 4) {
          const old = wide ? Number(moov.readBigUInt64BE(p)) : moov.readUInt32BE(p);
          if (old < after) continue;
          const next = old + delta;
          if (wide) moov.writeBigUInt64BE(BigInt(next), p);
          else if (next > 0xffffffff) return false;
          else moov.writeUInt32BE(next, p);
        }
      }
    }
    return true;
  };
  return walk(8, moov.length);
}

// The first chunk offset of every track, in file order of the tracks.
function firstChunkOffsets(buf) {
  const out = [];
  const walk = (start, end) => {
    for (const k of parseBoxes(buf, start, end) || []) {
      if (["moov", "trak", "mdia", "minf", "stbl"].includes(k.type)) walk(k.body, k.end);
      else if ((k.type === "stco" || k.type === "co64") && buf.readUInt32BE(k.body + 4) > 0) {
        out.push(k.type === "co64" ? Number(buf.readBigUInt64BE(k.body + 8)) : buf.readUInt32BE(k.body + 8));
      }
    }
  };
  walk(0, buf.length);
  return out;
}

const replaceOrAppend = (buf, kids, old, fresh) =>
  old ? kids.map(k => (k === old ? fresh : buf.subarray(k.start, k.end))) : [...kids.map(k => buf.subarray(k.start, k.end)), fresh];

/**
 * Apply field changes to an m4a file.
 *   changes: { KEY: "value" }  — "" or null REMOVES the field.
 * "COMMENT" is the standard comment; any other key is a free-form tag of that name.
 * Writes a temp file beside the original and renames over it, after checking the temp carries
 * the tags asked for and that its audio is the original's, byte for byte.
 * Returns "ok" or a short reason.
 */
export function writeM4aFields(filePath, changes) {
  let buf;
  try { buf = fs.readFileSync(filePath); } catch { return "unreadable"; }
  let at;
  try { at = locate(buf); } catch { at = null; }
  if (!at) return "not-m4a";
  // A fragmented file keeps offsets in other boxes too; nothing here rewrites those.
  if (at.top.some(b => b.type === "moof") || at.moovKids.some(b => b.type === "mvex")) return "fragmented";

  let items = at.items.map(item => ({ field: decodeItem(buf, item), bytes: buf.subarray(item.start, item.end) }));
  for (const [name, value] of Object.entries(changes)) {
    const key = storedKey(name);
    items = items.filter(i => !(i.field && sameKey(i.field.key, key)));
    if (value === null || value === undefined || value === "") continue;
    const bytes = key === name
      ? box("----", fullText("mean", MEAN), fullText("name", name), dataBox(value))
      : box(key, dataBox(value));
    items.push({ field: { key, value }, bytes });
  }

  const ilst = box("ilst", ...items.map(i => i.bytes));
  const meta = at.meta
    ? box("meta", buf.subarray(at.meta.body, at.meta.body + at.metaPrefix), ...replaceOrAppend(buf, at.metaKids, at.ilst, ilst))
    : box("meta", Buffer.alloc(4),
        box("hdlr", Buffer.alloc(8), Buffer.from("mdirappl", "latin1"), Buffer.alloc(9)), ilst);
  const udta = box("udta", ...replaceOrAppend(buf, at.udtaKids, at.meta, meta));
  // Buffer.concat copies, so the offsets patched below never touch the original bytes.
  const moov = box("moov", ...replaceOrAppend(buf, at.moovKids, at.udta, udta));

  const delta = moov.length - (at.moov.end - at.moov.start);
  if (delta !== 0 && !shiftChunkOffsets(moov, delta, at.moov.end)) return "too-large";
  const out = Buffer.concat([buf.subarray(0, at.moov.start), moov, buf.subarray(at.moov.end)]);

  // Check the result BEFORE it replaces anything: the tags read back, every box but moov is the
  // original's, and each track's audio is found where its offset table now says it is.
  const back = (() => { try { return locate(out); } catch { return null; } })();
  if (!back) return "verify-failed";
  const got = { fields: back.items.map(i => decodeItem(out, i)).filter(Boolean) };
  for (const [name, value] of Object.entries(changes)) {
    if (getM4aField(got, name) !== ((value === null || value === undefined) ? "" : value)) return "verify-failed";
  }
  if (back.top.length !== at.top.length) return "verify-failed";
  for (let i = 0; i < at.top.length; i++) {
    const a = at.top[i], b = back.top[i];
    if (a.type !== b.type) return "verify-failed";
    if (a.type !== "moov" && !buf.subarray(a.start, a.end).equals(out.subarray(b.start, b.end))) return "audio-changed";
  }
  const before = firstChunkOffsets(buf), after = firstChunkOffsets(out);
  if (before.length !== after.length) return "audio-changed";
  for (let i = 0; i < before.length; i++) {
    if (!buf.subarray(before[i], before[i] + 64).equals(out.subarray(after[i], after[i] + 64))) return "audio-changed";
  }

  const tmp = path.join(path.dirname(filePath),
    `.amlp-tag-${crypto.randomUUID().slice(0, 8)}${path.extname(filePath)}`);
  try {
    fs.writeFileSync(tmp, out);
    fs.renameSync(tmp, filePath);
  } catch {
    try { fs.unlinkSync(tmp); } catch {}
    return "replace-failed";
  }
  return "ok";
}
