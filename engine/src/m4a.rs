//! The text tags of an m4a (MP4 audio) file, read and written with no external tool.
//!
//! The twin of `flac.rs`: the app writes one tag (`YAIOL_AIID`) and may clear another (the comment). The
//! audio is never decoded: every byte outside the one rewritten box is copied through.
//!
//! The tags live in `moov > udta > meta > ilst`. Each child of `ilst` is one tag: `©cmt > data` is the
//! comment, `---- > mean + name + data` a free-form tag (mean `com.apple.iTunes`). A `data` payload is
//! 4 bytes of type (1 = UTF-8), 4 bytes of locale, then the value.
//!
//! ⚠ The audio's position is stored INSIDE `moov`: each track's `stco` / `co64` holds absolute file offsets
//! of its chunks. When `moov` sits before the audio and changes size, every offset must move by the same
//! amount, or the file opens, shows its tags and plays nothing. A write is therefore checked by reading
//! audio bytes back through the new offsets before the file is replaced. A fragmented file is refused.

use crate::flac;
use std::{fs, path::Path};

const MEAN: &str = "com.apple.iTunes";
const COMMENT: [u8; 4] = [0xA9, b'c', b'm', b't'];

#[derive(Clone, Copy, Debug)]
struct Bx {
    kind: [u8; 4],
    start: usize,
    end: usize,
    body: usize,
}

/// The boxes directly inside `buf[start..end]`. `None` when the bytes are not a well-formed box list.
fn parse(buf: &[u8], start: usize, end: usize) -> Option<Vec<Bx>> {
    let mut out = Vec::new();
    let mut pos = start;
    while pos < end {
        if pos + 8 > end {
            return None;
        }
        let mut size = u32::from_be_bytes(buf[pos..pos + 4].try_into().ok()?) as usize;
        let mut header = 8;
        if size == 1 {
            if pos + 16 > end {
                return None;
            }
            size = usize::try_from(u64::from_be_bytes(buf[pos + 8..pos + 16].try_into().ok()?)).ok()?;
            header = 16;
        } else if size == 0 {
            size = end - pos;
        }
        if size < header || pos + size > end {
            return None;
        }
        out.push(Bx { kind: buf[pos + 4..pos + 8].try_into().ok()?, start: pos, end: pos + size, body: pos + header });
        pos += size;
    }
    Some(out)
}

fn make(kind: &[u8; 4], parts: &[&[u8]]) -> Vec<u8> {
    let len: usize = parts.iter().map(|p| p.len()).sum::<usize>() + 8;
    let mut v = Vec::with_capacity(len);
    v.extend_from_slice(&(len as u32).to_be_bytes());
    v.extend_from_slice(kind);
    for p in parts {
        v.extend_from_slice(p);
    }
    v
}

fn full_text(kind: &[u8; 4], text: &str) -> Vec<u8> {
    make(kind, &[&[0; 4], text.as_bytes()])
}

fn data_box(text: &str) -> Vec<u8> {
    make(b"data", &[&[0, 0, 0, 1, 0, 0, 0, 0], text.as_bytes()])
}

struct Found {
    top: Vec<Bx>,
    moov: Bx,
    moov_kids: Vec<Bx>,
    udta: Option<Bx>,
    udta_kids: Vec<Bx>,
    meta: Option<Bx>,
    meta_kids: Vec<Bx>,
    meta_prefix: usize,
    ilst: Option<Bx>,
    items: Vec<Bx>,
}

/// Where the tags are. `meta` carries 4 bytes of version/flags before its children in an m4a (QuickTime's
/// own form has none); which one this file uses is read off where `hdlr` sits.
fn locate(buf: &[u8]) -> Option<Found> {
    let top = parse(buf, 0, buf.len())?;
    if !top.iter().any(|b| &b.kind == b"ftyp") {
        return None;
    }
    let moov = *top.iter().find(|b| &b.kind == b"moov")?;
    let moov_kids = parse(buf, moov.body, moov.end)?;
    let mut f = Found { top, moov, moov_kids, udta: None, udta_kids: Vec::new(), meta: None, meta_kids: Vec::new(), meta_prefix: 4, ilst: None, items: Vec::new() };
    f.udta = f.moov_kids.iter().find(|b| &b.kind == b"udta").copied();
    let Some(udta) = f.udta else { return Some(f) };
    f.udta_kids = parse(buf, udta.body, udta.end)?;
    f.meta = f.udta_kids.iter().find(|b| &b.kind == b"meta").copied();
    let Some(meta) = f.meta else { return Some(f) };
    let at = |o: usize| buf.get(meta.body + o..meta.body + o + 4);
    f.meta_prefix = if at(8) == Some(b"hdlr") {
        4
    } else if at(4) == Some(b"hdlr") {
        0
    } else {
        return None;
    };
    f.meta_kids = parse(buf, meta.body + f.meta_prefix, meta.end)?;
    f.ilst = f.meta_kids.iter().find(|b| &b.kind == b"ilst").copied();
    if let Some(ilst) = f.ilst {
        f.items = parse(buf, ilst.body, ilst.end)?;
    }
    Some(f)
}

/// One `ilst` item as `(key, value)`: the free-form name, or the atom type for a standard tag. `None` for an
/// item that holds no text (the cover, the track number).
fn decode(buf: &[u8], item: &Bx) -> Option<(String, String)> {
    let kids = parse(buf, item.body, item.end)?;
    let data = kids.iter().find(|k| &k.kind == b"data")?;
    if data.end - data.body < 8 || buf[data.body..data.body + 4] != [0, 0, 0, 1] {
        return None;
    }
    let value = String::from_utf8_lossy(&buf[data.body + 8..data.end]).into_owned();
    if &item.kind != b"----" {
        // the comment's type starts with 0xA9, which is not UTF-8 on its own: read it as Latin-1
        return Some((item.kind.iter().map(|b| *b as char).collect(), value));
    }
    let name = kids.iter().find(|k| &k.kind == b"name")?;
    if name.end - name.body < 4 {
        return None;
    }
    Some((String::from_utf8_lossy(&buf[name.body + 4..name.end]).into_owned(), value))
}

/// The key a caller's field name is stored under: `COMMENT` is the standard comment atom, any other name is
/// a free-form tag of that name.
fn stored_key(name: &str) -> String {
    if name.eq_ignore_ascii_case("comment") { COMMENT.iter().map(|b| *b as char).collect() } else { name.to_owned() }
}

/// The text tags of an m4a, `(key, value)` in file order. `None` = not an m4a, or unreadable.
pub fn read(file: &Path) -> Option<Vec<(String, String)>> {
    let buf = fs::read(file).ok()?;
    let at = locate(&buf)?;
    Some(at.items.iter().filter_map(|i| decode(&buf, i)).collect())
}

/// One field's value, `""` when absent. `key` is the caller's name: `COMMENT`, `YAIOL_AIID`.
pub fn get<'a>(tags: &'a [(String, String)], key: &str) -> &'a str {
    let want = stored_key(key);
    tags.iter().find(|(k, _)| k.eq_ignore_ascii_case(&want)).map(|(_, v)| v.as_str()).unwrap_or("")
}

/// Every chunk-offset table inside a rebuilt `moov`: entries pointing past `after` move by `delta`.
/// `false` when a 32-bit table cannot hold the moved offset.
fn shift_offsets(moov: &mut [u8], delta: i64, after: usize) -> bool {
    fn walk(m: &mut [u8], start: usize, end: usize, delta: i64, after: usize) -> bool {
        let Some(kids) = parse(m, start, end) else { return false };
        for k in kids {
            match &k.kind {
                b"trak" | b"mdia" | b"minf" | b"stbl" => {
                    if !walk(m, k.body, k.end, delta, after) {
                        return false;
                    }
                }
                b"stco" | b"co64" => {
                    let wide = &k.kind == b"co64";
                    let Some(count) = m.get(k.body + 4..k.body + 8).map(|b| u32::from_be_bytes(b.try_into().unwrap())) else { return false };
                    let step = if wide { 8 } else { 4 };
                    let mut p = k.body + 8;
                    for _ in 0..count {
                        if p + step > k.end {
                            return false;
                        }
                        let old = if wide { u64::from_be_bytes(m[p..p + 8].try_into().unwrap()) } else { u64::from(u32::from_be_bytes(m[p..p + 4].try_into().unwrap())) };
                        if old as usize >= after {
                            let next = old as i64 + delta;
                            if next < 0 {
                                return false;
                            }
                            if wide {
                                m[p..p + 8].copy_from_slice(&(next as u64).to_be_bytes());
                            } else if next > i64::from(u32::MAX) {
                                return false;
                            } else {
                                m[p..p + 4].copy_from_slice(&(next as u32).to_be_bytes());
                            }
                        }
                        p += step;
                    }
                }
                _ => {}
            }
        }
        true
    }
    let len = moov.len();
    walk(moov, 8, len, delta, after)
}

/// The first chunk offset of every track, in file order.
fn first_offsets(buf: &[u8]) -> Vec<usize> {
    fn walk(buf: &[u8], start: usize, end: usize, out: &mut Vec<usize>) {
        for k in parse(buf, start, end).unwrap_or_default() {
            match &k.kind {
                b"moov" | b"trak" | b"mdia" | b"minf" | b"stbl" => walk(buf, k.body, k.end, out),
                b"stco" | b"co64" => {
                    let count = u32::from_be_bytes(buf[k.body + 4..k.body + 8].try_into().unwrap());
                    if count > 0 {
                        let p = k.body + 8;
                        out.push(if &k.kind == b"co64" { u64::from_be_bytes(buf[p..p + 8].try_into().unwrap()) as usize } else { u32::from_be_bytes(buf[p..p + 4].try_into().unwrap()) as usize });
                    }
                }
                _ => {}
            }
        }
    }
    let mut out = Vec::new();
    walk(buf, 0, buf.len(), &mut out);
    out
}

/// The children of a container with `old` replaced by `fresh`, or `fresh` appended when there was none.
fn replace_or_append(buf: &[u8], kids: &[Bx], old: Option<Bx>, fresh: Vec<u8>) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    for k in kids {
        match old {
            Some(o) if o.start == k.start => out.extend_from_slice(&fresh),
            _ => out.extend_from_slice(&buf[k.start..k.end]),
        }
    }
    if old.is_none() {
        out.extend_from_slice(&fresh);
    }
    out
}

/// Apply field changes to an m4a: `(KEY, value)`, an empty value REMOVES the field. `COMMENT` is the
/// standard comment, any other key a free-form tag of that name.
/// The new file is written beside the original and checked (the tags read back, every box but `moov` is the
/// original's, each track's audio is found where its offset table now says) BEFORE it takes the original's
/// place. Returns `ok`, or a short reason.
pub fn write(file: &Path, changes: &[(&str, &str)]) -> &'static str {
    let Ok(buf) = fs::read(file) else { return "unreadable" };
    let Some(at) = locate(&buf) else { return "not-m4a" };
    // a fragmented file keeps offsets in other boxes too; nothing here rewrites those
    if at.top.iter().any(|b| &b.kind == b"moof") || at.moov_kids.iter().any(|b| &b.kind == b"mvex") {
        return "fragmented";
    }

    let mut items: Vec<(Option<String>, Vec<u8>)> = at.items.iter().map(|i| (decode(&buf, i).map(|(k, _)| k), buf[i.start..i.end].to_vec())).collect();
    for (name, value) in changes {
        let key = stored_key(name);
        items.retain(|(k, _)| !k.as_deref().is_some_and(|k| k.eq_ignore_ascii_case(&key)));
        if value.is_empty() {
            continue;
        }
        let bytes = if key == *name { make(b"----", &[&full_text(b"mean", MEAN), &full_text(b"name", name), &data_box(value)]) } else { make(&COMMENT, &[&data_box(value)]) };
        items.push((Some(key), bytes));
    }

    let ilst = make(b"ilst", &items.iter().map(|(_, b)| b.as_slice()).collect::<Vec<_>>());
    let meta = match at.meta {
        Some(m) => make(b"meta", &[&buf[m.body..m.body + at.meta_prefix], &replace_or_append(&buf, &at.meta_kids, at.ilst, ilst)]),
        None => make(b"meta", &[&[0; 4], &make(b"hdlr", &[&[0; 8], b"mdirappl", &[0; 9]]), &ilst]),
    };
    let udta = make(b"udta", &[&replace_or_append(&buf, &at.udta_kids, at.meta, meta)]);
    let mut moov = make(b"moov", &[&replace_or_append(&buf, &at.moov_kids, at.udta, udta)]);

    let delta = moov.len() as i64 - (at.moov.end - at.moov.start) as i64;
    if delta != 0 && !shift_offsets(&mut moov, delta, at.moov.end) {
        return "too-large";
    }
    let mut out = Vec::with_capacity(buf.len() + moov.len());
    out.extend_from_slice(&buf[..at.moov.start]);
    out.extend_from_slice(&moov);
    out.extend_from_slice(&buf[at.moov.end..]);

    // check the result BEFORE it replaces anything
    let Some(back) = locate(&out) else { return "verify-failed" };
    let got: Vec<(String, String)> = back.items.iter().filter_map(|i| decode(&out, i)).collect();
    if changes.iter().any(|(name, value)| get(&got, name) != *value) {
        return "verify-failed";
    }
    if back.top.len() != at.top.len() {
        return "verify-failed";
    }
    for (a, b) in at.top.iter().zip(&back.top) {
        if a.kind != b.kind {
            return "verify-failed";
        }
        if &a.kind != b"moov" && buf[a.start..a.end] != out[b.start..b.end] {
            return "audio-changed";
        }
    }
    let (before, after) = (first_offsets(&buf), first_offsets(&out));
    if before.len() != after.len() {
        return "audio-changed";
    }
    for (x, y) in before.iter().zip(&after) {
        let (xe, ye) = ((x + 64).min(buf.len()), (y + 64).min(out.len()));
        if xe.checked_sub(*x) != ye.checked_sub(*y) || buf.get(*x..xe) != out.get(*y..ye) {
            return "audio-changed";
        }
    }

    let tmp = flac::sibling(file, "tag");
    if fs::write(&tmp, &out).is_err() {
        let _ = fs::remove_file(&tmp);
        return "write-failed";
    }
    if flac::replace_file(&tmp, file) { "ok" } else { "replace-failed" }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    const AUDIO: &[u8] = b"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef-the-end-of-audio";

    /// A minimal m4a: ftyp, moov(trak(stbl(stco))) [+ udta], mdat. `moov_first`: moov before mdat.
    pub(crate) fn sample(tags: &[(&str, &str)], moov_first: bool) -> Vec<u8> {
        let ftyp = make(b"ftyp", &[b"M4A ", &[0; 4], b"M4A mp42isom"]);
        let build = |offset: u32| {
            let stco = make(b"stco", &[&[0; 4], &1u32.to_be_bytes(), &offset.to_be_bytes()]);
            let trak = make(b"trak", &[&make(b"mdia", &[&make(b"minf", &[&make(b"stbl", &[&stco])])])]);
            let mut kids = vec![trak];
            if !tags.is_empty() {
                let items: Vec<Vec<u8>> = tags.iter().map(|(k, v)| make(b"----", &[&full_text(b"mean", MEAN), &full_text(b"name", k), &data_box(v)])).collect();
                let ilst = make(b"ilst", &items.iter().map(|i| i.as_slice()).collect::<Vec<_>>());
                let meta = make(b"meta", &[&[0; 4], &make(b"hdlr", &[&[0; 8], b"mdirappl", &[0; 9]]), &ilst]);
                kids.push(make(b"udta", &[&meta]));
            }
            make(b"moov", &kids.iter().map(|k| k.as_slice()).collect::<Vec<_>>())
        };
        let mdat = make(b"mdat", &[AUDIO]);
        if moov_first {
            let moov_len = build(0).len();
            let offset = (ftyp.len() + moov_len + 8) as u32;
            [ftyp, build(offset), mdat].concat()
        } else {
            let offset = (ftyp.len() + 8) as u32;
            [ftyp, mdat, build(offset)].concat()
        }
    }

    fn file(name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("yaiol-amlp-m4a-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        fs::write(&path, bytes).unwrap();
        path
    }

    fn audio_through_offsets(path: &Path) -> Vec<u8> {
        let buf = fs::read(path).unwrap();
        let at = first_offsets(&buf)[0];
        buf[at..at + AUDIO.len()].to_vec()
    }

    #[test]
    fn a_file_with_no_tag_box_gets_one() {
        let f = file("bare.m4a", &sample(&[], true));
        assert_eq!(write(&f, &[("YAIOL_AIID", "suno:abc"), ("COMMENT", "hello")]), "ok");
        let tags = read(&f).unwrap();
        assert_eq!((get(&tags, "YAIOL_AIID"), get(&tags, "COMMENT")), ("suno:abc", "hello"));
        assert_eq!(audio_through_offsets(&f), AUDIO);
    }

    #[test]
    fn the_audio_offsets_move_when_the_tags_come_before_it() {
        let f = file("before.m4a", &sample(&[("ARTIST", "x")], true));
        let first = first_offsets(&fs::read(&f).unwrap())[0];
        assert_eq!(write(&f, &[("YAIOL_AIID", "suno:a-much-longer-value-to-grow-the-box")]), "ok");
        assert_ne!(first_offsets(&fs::read(&f).unwrap())[0], first);
        assert_eq!(audio_through_offsets(&f), AUDIO);
        let tags = read(&f).unwrap();
        assert_eq!(get(&tags, "ARTIST"), "x");
        // clearing a tag removes it, the others stay
        assert_eq!(write(&f, &[("YAIOL_AIID", "")]), "ok");
        let tags = read(&f).unwrap();
        assert_eq!((get(&tags, "YAIOL_AIID"), get(&tags, "ARTIST")), ("", "x"));
        assert_eq!(audio_through_offsets(&f), AUDIO);
    }

    #[test]
    fn a_moov_after_the_audio_leaves_the_offsets_alone() {
        let f = file("after.m4a", &sample(&[], false));
        let first = first_offsets(&fs::read(&f).unwrap())[0];
        assert_eq!(write(&f, &[("YAIOL_AIID", "suno:x")]), "ok");
        assert_eq!(first_offsets(&fs::read(&f).unwrap())[0], first);
        assert_eq!(audio_through_offsets(&f), AUDIO);
    }

    #[test]
    fn a_fragmented_file_and_a_non_m4a_are_refused() {
        let mut bytes = sample(&[], true);
        bytes.extend(make(b"moof", &[&[0; 8]]));
        assert_eq!(write(&file("frag.m4a", &bytes), &[("YAIOL_AIID", "x")]), "fragmented");
        assert_eq!(write(&file("no.m4a", b"not an mp4 at all"), &[("YAIOL_AIID", "x")]), "not-m4a");
        assert!(read(&file("no2.m4a", b"nope")).is_none());
    }
}
