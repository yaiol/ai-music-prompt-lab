//! Reading and writing the tags of a FLAC file, with no outside tool.
//!
//! Only the tag block is rewritten: the audio and every other block (the cover picture is its own block)
//! are copied through byte for byte. Nothing is decoded, nothing is encoded again.
//!
//! A FLAC file: `fLaC`, then metadata blocks, each a 1-byte head (bit 7 = "this is the last block",
//! bits 0-6 = its type), a 3-byte length, its data; then the audio to the end of the file.
//! The tag block (type 4): a vendor text, then a count and that many `KEY=value` texts, each with a length.
//!
//! ⚠ two traps, and both fail QUIETLY (the file still opens, its tags are garbage):
//!   - a block's length is BIG-endian, while every length INSIDE the tag block is LITTLE-endian;
//!   - exactly ONE block carries the "last" flag, and it must be the one physically last.

use std::{
    fs,
    path::{Path, PathBuf},
};

const TAG_BLOCK: u8 = 4;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Tags {
    pub vendor: String,
    /// (key, value), in the file's order.
    pub fields: Vec<(String, String)>,
}

impl Tags {
    /// A field's value, whatever the case of its name; empty when absent.
    pub fn get(&self, key: &str) -> &str {
        self.fields.iter().find(|(k, _)| k.eq_ignore_ascii_case(key)).map(|(_, v)| v.as_str()).unwrap_or("")
    }
}

struct Block<'a> {
    kind: u8,
    data: std::borrow::Cow<'a, [u8]>,
}

/// The metadata blocks, and where the audio starts.
fn parse_blocks(buf: &[u8]) -> Option<(Vec<Block<'_>>, usize)> {
    if buf.len() < 4 || &buf[..4] != b"fLaC" {
        return None;
    }
    let mut blocks = Vec::new();
    let mut pos = 4;
    loop {
        let head = *buf.get(pos)?;
        let len = (*buf.get(pos + 1)? as usize) << 16 | (*buf.get(pos + 2)? as usize) << 8 | *buf.get(pos + 3)? as usize;
        let start = pos + 4;
        let data = buf.get(start..start + len)?;
        blocks.push(Block { kind: head & 0x7f, data: data.into() });
        pos = start + len;
        if head & 0x80 != 0 {
            break;
        }
    }
    Some((blocks, pos))
}

fn u32le(data: &[u8], at: usize) -> Option<usize> {
    Some(u32::from_le_bytes(data.get(at..at + 4)?.try_into().ok()?) as usize)
}

fn decode(data: &[u8]) -> Option<Tags> {
    let vendor_len = u32le(data, 0)?;
    let mut p = 4;
    let vendor = String::from_utf8_lossy(data.get(p..p + vendor_len)?).into_owned();
    p += vendor_len;
    let count = u32le(data, p)?;
    p += 4;
    let mut fields = Vec::new();
    for _ in 0..count {
        let len = u32le(data, p)?;
        p += 4;
        let entry = String::from_utf8_lossy(data.get(p..p + len)?).into_owned();
        p += len;
        if let Some((key, value)) = entry.split_once('=') {
            if !key.is_empty() {
                fields.push((key.to_owned(), value.to_owned()));
            }
        }
    }
    Some(Tags { vendor, fields })
}

fn encode(tags: &Tags) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend((tags.vendor.len() as u32).to_le_bytes());
    out.extend(tags.vendor.as_bytes());
    out.extend((tags.fields.len() as u32).to_le_bytes());
    for (key, value) in &tags.fields {
        let entry = format!("{key}={value}");
        out.extend((entry.len() as u32).to_le_bytes());
        out.extend(entry.as_bytes());
    }
    out
}

fn tags_of(buf: &[u8]) -> Option<Tags> {
    let (blocks, _) = parse_blocks(buf)?;
    match blocks.iter().find(|b| b.kind == TAG_BLOCK) {
        Some(b) => decode(&b.data),
        None => Some(Tags::default()),
    }
}

/// A FLAC file's tags. `None` = not a FLAC, or unreadable.
pub fn read(file: &Path) -> Option<Tags> {
    tags_of(&fs::read(file).ok()?)
}

/// A new name in the same folder, for a file in the making or one moved aside.
pub(crate) fn sibling(file: &Path, word: &str) -> PathBuf {
    let id = uuid::Uuid::new_v4().to_string();
    let ext = file.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    file.with_file_name(format!(".amlp-{word}-{}{ext}", &id[..8]))
}

/// Put a finished, checked file in the place of the original. `true` = the original's name now holds the
/// new content.
// ⚠ do not reduce this to the one rename. On Windows a file cannot be renamed OVER while anything
// reads it, and the app's own player reads the very file a card is linked to. A file being read CAN be
// moved aside: so the original is moved, the new one takes its name, and the old one is deleted (its
// reader keeps its data until it is done). Whatever happens, one complete file is left in the folder.
pub fn replace_file(tmp: &Path, file: &Path) -> bool {
    if fs::rename(tmp, file).is_ok() {
        return true;
    }
    let aside = sibling(file, "old");
    if fs::rename(file, &aside).is_err() {
        let _ = fs::remove_file(tmp);
        return false;
    }
    if fs::rename(tmp, file).is_err() {
        // put the original back; the new file is only deleted once the original has its name again
        if fs::rename(&aside, file).is_ok() {
            let _ = fs::remove_file(tmp);
        }
        return false;
    }
    let _ = fs::remove_file(aside);
    true
}

/// Change fields of a FLAC file: `(KEY, value)`, an empty value REMOVES the field.
/// The new file is written beside the original and checked (it reads back, it carries what was asked, its
/// audio is byte for byte the original's) BEFORE it takes the original's place.
/// Returns `ok`, or a short reason.
pub fn write(file: &Path, changes: &[(&str, &str)]) -> &'static str {
    let Ok(buf) = fs::read(file) else { return "unreadable" };
    let Some((mut blocks, audio_at)) = parse_blocks(&buf) else { return "not-flac" };
    let at = blocks.iter().position(|b| b.kind == TAG_BLOCK);
    let mut tags = match at {
        Some(i) => match decode(&blocks[i].data) {
            Some(t) => t,
            None => return "bad-comment-block",
        },
        None => Tags::default(),
    };
    for (key, value) in changes {
        tags.fields.retain(|(k, _)| !k.eq_ignore_ascii_case(key));
        if !value.is_empty() {
            tags.fields.push(((*key).to_owned(), (*value).to_owned()));
        }
    }
    let data = encode(&tags);
    match at {
        Some(i) => blocks[i].data = data.into(),
        None => blocks.insert(1.min(blocks.len()), Block { kind: TAG_BLOCK, data: data.into() }),
    }
    let mut out = b"fLaC".to_vec();
    let last = blocks.len() - 1;
    for (i, b) in blocks.iter().enumerate() {
        if b.data.len() > 0xFF_FFFF {
            return "block-too-large";
        }
        out.push(if i == last { 0x80 } else { 0 } | (b.kind & 0x7f));
        out.extend(&(b.data.len() as u32).to_be_bytes()[1..]);
        out.extend(b.data.iter());
    }
    out.extend(&buf[audio_at..]);

    let tmp = sibling(file, "tag");
    if fs::write(&tmp, &out).is_err() {
        let _ = fs::remove_file(&tmp);
        return "write-failed";
    }
    // Read the new file back from disk and check it, before the original is touched.
    let verdict = (|| {
        let after = fs::read(&tmp).ok()?;
        let check = tags_of(&after)?;
        if changes.iter().any(|(key, value)| check.get(key) != *value) {
            return None;
        }
        let (_, after_audio) = parse_blocks(&after)?;
        // this is a tag edit, not a new encoding: the audio must be untouched
        Some(if after[after_audio..] == buf[audio_at..] { "ok" } else { "audio-changed" })
    })();
    match verdict {
        Some("ok") => {
            if replace_file(&tmp, file) {
                "ok"
            } else {
                "replace-failed"
            }
        }
        Some(reason) => {
            let _ = fs::remove_file(&tmp);
            reason
        }
        None => {
            let _ = fs::remove_file(&tmp);
            "verify-failed"
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A small FLAC: stream info, optionally a tag block, a "picture" block, then fake audio.
    pub(crate) fn sample(tags: Option<&Tags>) -> Vec<u8> {
        let mut f = b"fLaC".to_vec();
        let mut block = |kind: u8, last: bool, data: &[u8]| {
            f.push(if last { 0x80 } else { 0 } | kind);
            f.extend(&(data.len() as u32).to_be_bytes()[1..]);
            f.extend(data);
        };
        block(0, false, &[7u8; 34]);
        if let Some(t) = tags {
            block(4, false, &encode(t));
        }
        block(6, true, b"the cover picture, untouched");
        f.extend(b"AUDIO FRAMES \x00\x01\x02 to the end");
        f
    }

    fn temp(name: &str) -> PathBuf {
        // one folder per file, so tests running side by side never see each other's temporary files
        let dir = std::env::temp_dir().join(format!("yaiol-amlp-flac-{}-{name}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir.join(name)
    }

    #[test]
    fn a_field_is_added_changed_and_removed_and_nothing_else_moves() {
        let file = temp("a.flac");
        let before = Tags { vendor: "enc 1.0".into(), fields: vec![("TITLE".into(), "Song é".into()), ("comment".into(), "made with suno; id=x".into())] };
        fs::write(&file, sample(Some(&before))).unwrap();
        assert_eq!(read(&file).unwrap(), before);

        assert_eq!(write(&file, &[("YAIOL_AIID", "suno:abc"), ("COMMENT", "")]), "ok");
        let after = read(&file).unwrap();
        assert_eq!(after.vendor, "enc 1.0");
        assert_eq!(after.fields, vec![("TITLE".to_owned(), "Song é".to_owned()), ("YAIOL_AIID".to_owned(), "suno:abc".to_owned())]);
        // the picture block and the audio are byte for byte what they were, and exactly one block is "last"
        let bytes = fs::read(&file).unwrap();
        let (blocks, audio) = parse_blocks(&bytes).unwrap();
        assert_eq!(blocks.iter().map(|b| b.kind).collect::<Vec<_>>(), vec![0, 4, 6]);
        assert_eq!(&blocks[2].data[..], b"the cover picture, untouched");
        assert_eq!(&bytes[audio..], b"AUDIO FRAMES \x00\x01\x02 to the end");
        // no file left behind in the folder
        let left: Vec<_> = fs::read_dir(file.parent().unwrap()).unwrap().flatten().filter(|e| e.file_name().to_string_lossy().starts_with(".amlp-")).collect();
        assert!(left.is_empty());
        let _ = fs::remove_file(file);
    }

    #[test]
    fn a_file_with_no_tag_block_gets_one_after_the_stream_info() {
        let file = temp("b.flac");
        fs::write(&file, sample(None)).unwrap();
        assert_eq!(read(&file).unwrap(), Tags::default());
        assert_eq!(write(&file, &[("TRACKNUMBER", "05")]), "ok");
        let bytes = fs::read(&file).unwrap();
        let (blocks, _) = parse_blocks(&bytes).unwrap();
        assert_eq!(blocks.iter().map(|b| b.kind).collect::<Vec<_>>(), vec![0, 4, 6]);
        assert_eq!(read(&file).unwrap().get("tracknumber"), "05");
        let _ = fs::remove_file(file);
    }

    #[test]
    fn what_is_not_a_flac_is_refused_and_left_alone() {
        let file = temp("c.flac");
        fs::write(&file, b"RIFF not a flac").unwrap();
        assert!(read(&file).is_none());
        assert_eq!(write(&file, &[("A", "b")]), "not-flac");
        assert_eq!(fs::read(&file).unwrap(), b"RIFF not a flac");
        assert_eq!(write(&temp("missing.flac"), &[("A", "b")]), "unreadable");
        let _ = fs::remove_file(file);
    }
}
