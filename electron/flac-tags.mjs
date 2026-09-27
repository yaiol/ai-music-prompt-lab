// flac-tags.mjs — read and write FLAC Vorbis-comment fields, with no external tool.
//
// WHY THIS EXISTS: the app needs to write exactly one tag (AMLP_ID) and clear
// another (COMMENT). Shelling out to ffmpeg for that means either requiring the
// user to install it, or shipping ~80 MB of GPL video machinery inside an MIT
// app. Rewriting one metadata block is a few hundred bytes of well-specified
// container work, so it is done here instead.
//
// It is also SAFER than re-muxing: the audio frames and every other metadata
// block (crucially the embedded cover art, which is its own PICTURE block) are
// copied through byte for byte. Nothing is decoded and nothing is re-encoded.
//
// FLAC layout:
//   "fLaC"
//   then a chain of metadata blocks, each:
//     1 byte  : bit 7 = "this is the last metadata block", bits 0-6 = type
//     3 bytes : data length, BIG-endian
//     N bytes : data
//   then the audio frames, to end of file.
//
// VORBIS_COMMENT (type 4) data:
//   4 bytes : vendor length, LITTLE-endian
//   N bytes : vendor string (UTF-8)
//   4 bytes : field count, LITTLE-endian
//   per field: 4 bytes length LITTLE-endian, then "KEY=value" (UTF-8)
//
// ⚠ CLAUDE: the block header length is BIG-endian and every length inside the
// comment block is LITTLE-endian. Mixing them up produces a file that still
// opens (the header parses) but whose tags are garbage - so it fails quietly,
// which is exactly the kind of bug this comment is here to stop.

import fs from "fs";
import path from "path";
import crypto from "crypto";

const BLOCK_VORBIS_COMMENT = 4;
const BLOCK_PADDING = 1;

// Split a FLAC file into its metadata blocks plus the offset where audio starts.
function parseBlocks(buf) {
  if (buf.length < 4 || buf.toString("latin1", 0, 4) !== "fLaC") return null;
  const blocks = [];
  let pos = 4;
  for (;;) {
    if (pos + 4 > buf.length) return null;
    const header = buf[pos];
    const isLast = (header & 0x80) !== 0;
    const type = header & 0x7f;
    const len = (buf[pos + 1] << 16) | (buf[pos + 2] << 8) | buf[pos + 3];
    const start = pos + 4;
    if (start + len > buf.length) return null;
    blocks.push({ type, isLast, data: buf.subarray(start, start + len) });
    pos = start + len;
    if (isLast) break;
  }
  return { blocks, audioOffset: pos };
}

function decodeComment(data) {
  let p = 0;
  const vendorLen = data.readUInt32LE(p); p += 4;
  const vendor = data.toString("utf8", p, p + vendorLen); p += vendorLen;
  const count = data.readUInt32LE(p); p += 4;
  const fields = [];
  for (let i = 0; i < count; i++) {
    const len = data.readUInt32LE(p); p += 4;
    const entry = data.toString("utf8", p, p + len); p += len;
    const eq = entry.indexOf("=");
    if (eq > 0) fields.push({ key: entry.slice(0, eq), value: entry.slice(eq + 1) });
  }
  return { vendor, fields };
}

function encodeComment({ vendor, fields }) {
  const v = Buffer.from(vendor, "utf8");
  const entries = fields.map(f => Buffer.from(`${f.key}=${f.value}`, "utf8"));
  const size = 4 + v.length + 4 + entries.reduce((n, e) => n + 4 + e.length, 0);
  const out = Buffer.alloc(size);
  let p = 0;
  out.writeUInt32LE(v.length, p); p += 4;
  v.copy(out, p); p += v.length;
  out.writeUInt32LE(entries.length, p); p += 4;
  for (const e of entries) {
    out.writeUInt32LE(e.length, p); p += 4;
    e.copy(out, p); p += e.length;
  }
  return out;
}

function blockHeader(type, len, isLast) {
  const h = Buffer.alloc(4);
  h[0] = (isLast ? 0x80 : 0) | (type & 0x7f);
  h[1] = (len >> 16) & 0xff;
  h[2] = (len >> 8) & 0xff;
  h[3] = len & 0xff;
  return h;
}

/** Read the Vorbis fields of a FLAC file. Returns null if it is not a FLAC. */
export function readFlacFields(filePath) {
  let buf;
  try { buf = fs.readFileSync(filePath); } catch { return null; }
  const parsed = parseBlocks(buf);
  if (!parsed) return null;
  const cb = parsed.blocks.find(b => b.type === BLOCK_VORBIS_COMMENT);
  if (!cb) return { vendor: "", fields: [] };
  try { return decodeComment(cb.data); } catch { return null; }
}

/** Case-insensitive lookup of one field's value; "" when absent. */
export function getField(tags, key) {
  if (!tags) return "";
  const hit = tags.fields.find(f => f.key.toLowerCase() === key.toLowerCase());
  return hit ? hit.value : "";
}

/**
 * Apply field changes to a FLAC file.
 *   changes: { KEY: "value" }  — "" or null REMOVES the field.
 * Writes a temp file beside the original and renames over it, so the
 * replacement is atomic and a crash cannot leave a half-written master.
 * Returns "ok" or a short reason.
 */
export function writeFlacFields(filePath, changes) {
  let buf;
  try { buf = fs.readFileSync(filePath); } catch { return "unreadable"; }
  const parsed = parseBlocks(buf);
  if (!parsed) return "not-flac";

  const idx = parsed.blocks.findIndex(b => b.type === BLOCK_VORBIS_COMMENT);
  let tags;
  if (idx >= 0) {
    try { tags = decodeComment(parsed.blocks[idx].data); } catch { return "bad-comment-block"; }
  } else {
    tags = { vendor: "", fields: [] };
  }

  for (const [key, value] of Object.entries(changes)) {
    const keep = tags.fields.filter(f => f.key.toLowerCase() !== key.toLowerCase());
    if (value === null || value === undefined || value === "") tags.fields = keep;
    else tags.fields = [...keep, { key, value }];
  }

  const newData = encodeComment(tags);
  const blocks = parsed.blocks.slice();
  if (idx >= 0) blocks[idx] = { ...blocks[idx], data: newData };
  else blocks.splice(1, 0, { type: BLOCK_VORBIS_COMMENT, isLast: false, data: newData });

  // ⚠ CLAUDE: exactly ONE block carries the last-block flag, and it must be the
  // one physically last. Inheriting the old flags after inserting a block gives
  // a file whose metadata chain ends early - decoders then read padding as audio.
  const parts = [Buffer.from("fLaC", "latin1")];
  blocks.forEach((b, i) => {
    parts.push(blockHeader(b.type, b.data.length, i === blocks.length - 1));
    parts.push(b.data);
  });
  parts.push(buf.subarray(parsed.audioOffset));

  const tmp = path.join(path.dirname(filePath),
    `.amlp-tag-${crypto.randomUUID().slice(0, 8)}${path.extname(filePath)}`);
  try {
    fs.writeFileSync(tmp, Buffer.concat(parts));
  } catch {
    try { fs.unlinkSync(tmp); } catch {}
    return "write-failed";
  }

  // Verify the temp parses and carries what we asked for, BEFORE replacing.
  const check = readFlacFields(tmp);
  if (!check) { try { fs.unlinkSync(tmp); } catch {} return "verify-failed"; }
  for (const [key, value] of Object.entries(changes)) {
    const got = getField(check, key);
    const want = (value === null || value === undefined) ? "" : value;
    if (got !== want) { try { fs.unlinkSync(tmp); } catch {} return "verify-failed"; }
  }
  // The audio must be untouched - this is a tag edit, not a re-encode.
  const after = fs.readFileSync(tmp);
  const afterParsed = parseBlocks(after);
  if (!afterParsed || !after.subarray(afterParsed.audioOffset).equals(buf.subarray(parsed.audioOffset))) {
    try { fs.unlinkSync(tmp); } catch {}
    return "audio-changed";
  }

  try { fs.renameSync(tmp, filePath); } catch {
    try { fs.unlinkSync(tmp); } catch {}
    return "replace-failed";
  }
  return "ok";
}

export { BLOCK_PADDING };
