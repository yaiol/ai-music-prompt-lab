//! The Suno song id carried by a media file: reading it, and moving it into its own tag.
//!
//! Suno writes its provenance in the file's comment (`made with suno; created=...; id=<uuid>`). The id
//! belongs in a tag of its own, `YAIOL_AIID` (`suno:<uuid>`), which can be stripped when making share
//! copies while the comment stays free for text meant to be read.
//!
//! The rules, each one load-bearing:
//! - ⚠ A `.wav` is NEVER written to: it is the untouched original, and the only reason a lost id can be
//!   recovered at all.
//! - ⚠ The id MOVES, it is never copied: a copy left in the comment would ship anyway.
//! - ⚠ The FILE outranks a link typed by hand: an id already in the file came from Suno itself, so a
//!   link that disagrees is REFUSED, never written over it.
//! - The comment is only cleared when it is Suno's own text or carries that same id; any other text there
//!   is the user's and stays.
//! - Reading looks at `YAIOL_AIID` FIRST: its value is `suno:<uuid>`, which the `id=` pattern does not
//!   match, so a file whose id was moved would otherwise read as having none.
//!
//! A FLAC (`flac.rs`) and an m4a (`m4a.rs`) are written; any other format is `unsupported`.

use crate::{cards::suno_id_in_link, flac, m4a};
use lofty::prelude::*;
use std::path::Path;

fn is_uuid(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 36 && b.iter().enumerate().all(|(i, c)| if matches!(i, 8 | 13 | 18 | 23) { *c == b'-' } else { c.is_ascii_hexdigit() })
}

/// The id in a text holding `id=<uuid>`, lower-case. The pattern is `id=` ALONE, on purpose: nothing
/// guarantees that "made with suno" shares a line with it.
pub fn suno_id_in_comment(text: &str) -> Option<String> {
    let mut rest = text;
    while let Some(at) = rest.find("id=") {
        let candidate: String = rest[at + 3..].chars().take(36).collect();
        if is_uuid(&candidate) {
            return Some(candidate.to_lowercase());
        }
        rest = &rest[at + 3..];
    }
    None
}

/// `suno:<uuid>` gives the uuid, lower-case.
fn suno_id_in_aiid(value: &str) -> Option<String> {
    value.strip_prefix("suno:").filter(|id| is_uuid(id)).map(str::to_lowercase)
}

/// One file's id, from its own tags.
fn id_in_file(file: &Path) -> Option<String> {
    if !file.is_file() {
        return None;
    }
    // a FLAC is read by this app's own reader, the one that also writes it
    if let Some(tags) = flac::read(file) {
        if let Some(id) = suno_id_in_aiid(tags.get("YAIOL_AIID")) {
            return Some(id);
        }
        return tags.fields.iter().filter(|(k, _)| k.to_lowercase().contains("comm") || k.to_lowercase().contains("desc")).find_map(|(_, v)| suno_id_in_comment(v));
    }
    // an m4a is read by this app's own reader too, the one that also writes it
    if let Some(tags) = m4a::read(file) {
        if let Some(id) = suno_id_in_aiid(m4a::get(&tags, "YAIOL_AIID")) {
            return Some(id);
        }
        return suno_id_in_comment(m4a::get(&tags, "COMMENT"));
    }
    let tagged = lofty::read_from_path(file).ok()?;
    let mut comments: Vec<String> = Vec::new();
    for tag in tagged.tags() {
        for item in tag.items() {
            let Some(text) = item.value().text() else { continue };
            // the key arrives wrapped by the container (`TXXX:YAIOL_AIID`, `----:com.apple.iTunes:YAIOL_AIID`):
            // the match is on its END
            let key = format!("{:?}", item.key()).to_lowercase();
            if key.trim_end_matches(['"', ')']).ends_with("yaiol_aiid") {
                if let Some(id) = suno_id_in_aiid(text) {
                    return Some(id);
                }
                continue;
            }
            if key.contains("comm") || key.contains("desc") {
                comments.push(text.to_owned());
            }
        }
    }
    comments.iter().find_map(|c| suno_id_in_comment(c))
}

/// The Suno id of a media file. When the file has none, the file of the same name in the other lossless
/// format is asked (the wav keeps its comment as provenance when the flac's was cleaned).
pub fn read_suno_id(media: &Path) -> Option<String> {
    if let Some(id) = id_in_file(media) {
        return Some(id);
    }
    let ext = media.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    ["wav", "flac"].iter().filter(|e| **e != ext).find_map(|e| id_in_file(&media.with_extension(e)))
}

fn ext_is(file: &Path, ext: &str) -> bool {
    file.extension().is_some_and(|e| e.to_string_lossy().eq_ignore_ascii_case(ext))
}

/// The two tags this module cares about, read from a FLAC or an m4a: `(YAIOL_AIID, COMMENT)`. `None` = not
/// readable as either (the caller answers `unsupported` before getting here).
fn aiid_and_comment(media: &Path) -> Option<(String, String)> {
    if ext_is(media, "flac") {
        let t = flac::read(media)?;
        Some((t.get("YAIOL_AIID").to_owned(), t.get("COMMENT").to_owned()))
    } else {
        let t = m4a::read(media)?;
        Some((m4a::get(&t, "YAIOL_AIID").to_owned(), m4a::get(&t, "COMMENT").to_owned()))
    }
}

/// Change fields of a FLAC or an m4a. Returns `ok` or a short reason.
fn write_fields(media: &Path, changes: &[(&str, &str)]) -> &'static str {
    if ext_is(media, "flac") { flac::write(media, changes) } else { m4a::write(media, changes) }
}

/// Move the comment's id into `YAIOL_AIID`. Returns a short reason (`moved` when done).
pub fn move_suno_id_to_tag(media: &Path) -> String {
    if ext_is(media, "wav") {
        return "skipped-wav".to_owned();
    }
    if !ext_is(media, "flac") && !ext_is(media, "m4a") {
        return "unsupported".to_owned();
    }
    if !media.is_file() {
        return "missing".to_owned();
    }
    let Some((aiid, comment)) = aiid_and_comment(media) else { return "unreadable".to_owned() };
    if !aiid.is_empty() {
        return "already-tagged".to_owned();
    }
    let Some(id) = suno_id_in_comment(&comment) else { return "no-id".to_owned() };
    match write_fields(media, &[("YAIOL_AIID", &format!("suno:{id}")), ("COMMENT", "")]) {
        "ok" => "moved".to_owned(),
        other => other.to_owned(),
    }
}

/// Write into the file an id that is nowhere in it, taken from a link on the card. This is the only way
/// a Suno Studio export can ever be identified: Suno strips the id on export.
fn write_suno_id(media: &Path, link_id: &str) -> String {
    if ext_is(media, "wav") {
        return "skipped-wav".to_owned();
    }
    if !ext_is(media, "flac") && !ext_is(media, "m4a") {
        return "unsupported".to_owned();
    }
    if !media.is_file() {
        return "missing".to_owned();
    }
    let Some((existing, comment)) = aiid_and_comment(media) else { return "unreadable".to_owned() };
    let wanted = format!("suno:{link_id}");
    if !existing.is_empty() {
        return if existing == wanted { "already-tagged" } else { "conflict-amlp" }.to_owned();
    }
    let comment_id = suno_id_in_comment(&comment);
    if comment_id.as_deref().is_some_and(|id| id != link_id) {
        return "conflict-comment".to_owned();
    }
    let clear_comment = comment_id.is_some() || comment.trim_start().to_lowercase().starts_with("made with suno");
    let mut changes: Vec<(&str, &str)> = vec![("YAIOL_AIID", &wanted)];
    if clear_comment {
        changes.push(("COMMENT", ""));
    }
    match write_fields(media, &changes) {
        "ok" => "written".to_owned(),
        other => other.to_owned(),
    }
}

/// Give a media file the id of a Suno link. The caller checks the "use the tag" setting first.
/// Shared by the two ways the pair can meet (a link added to a card with media, media linked to a card
/// with a link), so the result never depends on which came first.
pub fn stamp_suno_id_from_link(media: &Path, href: &str) {
    let Some(link_id) = suno_id_in_link(href) else { return };
    let result = write_suno_id(media, &link_id);
    if !["written", "already-tagged", "skipped-wav", "unsupported"].contains(&result.as_str()) {
        eprintln!("YAIOL_AIID not written for {}: {result}", media.display());
    }
}

/// Keep the embedded track number equal to the file's place. The value's own shape is kept: zero padding
/// (`03`) and a `/total` after it. FLAC only. Returns `written`, `unchanged`, or a short reason.
pub fn write_track_number(media: &Path, position: usize) -> String {
    if !ext_is(media, "flac") {
        return "not-flac".to_owned();
    }
    let Some(tags) = flac::read(media) else { return "unreadable".to_owned() };
    let old = tags.get("TRACKNUMBER");
    let trimmed = old.trim_start();
    let digits = trimmed.len() - trimmed.trim_start_matches(|c: char| c.is_ascii_digit()).len();
    let (number, suffix) = trimmed.split_at(digits);
    // a suffix is kept only when it is a "/total"
    let shaped = digits > 0 && (suffix.trim().is_empty() || suffix.trim_start().starts_with('/'));
    let width = if shaped && number.starts_with('0') { digits } else { 1 };
    let next = format!("{position:0width$}{}", if shaped && suffix.trim_start().starts_with('/') { suffix } else { "" });
    if next == old {
        return "unchanged".to_owned();
    }
    match flac::write(media, &[("TRACKNUMBER", &next)]) {
        "ok" => "written".to_owned(),
        other => other.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flac::{Tags, tests::sample};
    use std::fs;

    const ID: &str = "bf16a683-1234-4abc-9def-0123456789ab";

    fn file(name: &str, fields: &[(&str, &str)]) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("yaiol-amlp-aiid-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        let tags = Tags { vendor: "v".into(), fields: fields.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect() };
        fs::write(&path, sample(Some(&tags))).unwrap();
        path
    }

    #[test]
    fn the_id_is_found_in_a_comment_and_in_its_own_tag() {
        assert_eq!(suno_id_in_comment(&format!("made with suno; created=2026-08-04T09:52:08Z; id={ID}")).as_deref(), Some(ID));
        assert_eq!(suno_id_in_comment("grid=1; id=nope"), None);
        let in_comment = file("c.flac", &[("COMMENT", &format!("made with suno; id={}", ID.to_uppercase()))]);
        assert_eq!(read_suno_id(&in_comment).as_deref(), Some(ID));
        let in_tag = file("t.flac", &[("yaiol_aiid", &format!("suno:{ID}"))]);
        assert_eq!(read_suno_id(&in_tag).as_deref(), Some(ID));
        // a file with none asks the file of the same name in the other format
        let bare = file("pair.wav", &[]);
        fs::write(&bare, b"RIFF").unwrap();
        file("pair.flac", &[("COMMENT", &format!("id={ID}"))]);
        assert_eq!(read_suno_id(&bare).as_deref(), Some(ID));
    }

    #[test]
    fn the_id_moves_out_of_the_comment_and_is_not_left_behind() {
        let f = file("move.flac", &[("TITLE", "x"), ("COMMENT", &format!("made with suno; id={ID}"))]);
        assert_eq!(move_suno_id_to_tag(&f), "moved");
        let tags = flac::read(&f).unwrap();
        assert_eq!((tags.get("YAIOL_AIID"), tags.get("COMMENT"), tags.get("TITLE")), (format!("suno:{ID}").as_str(), "", "x"));
        assert_eq!(move_suno_id_to_tag(&f), "already-tagged");
        assert_eq!(move_suno_id_to_tag(&file("none.flac", &[("COMMENT", "my own note")])), "no-id");
        assert_eq!(move_suno_id_to_tag(Path::new("x.wav")), "skipped-wav");
        assert_eq!(move_suno_id_to_tag(Path::new("x.ogg")), "unsupported");
    }

    #[test]
    fn a_link_never_overwrites_what_the_file_says() {
        let other = "00000000-1111-4222-8333-444444444444";
        // an id already in its tag, a different one in the link: refused
        let tagged = file("conflict.flac", &[("YAIOL_AIID", &format!("suno:{ID}"))]);
        assert_eq!(write_suno_id(&tagged, other), "conflict-amlp");
        assert_eq!(write_suno_id(&tagged, ID), "already-tagged");
        // a different id in the comment: refused too
        let commented = file("conflict2.flac", &[("COMMENT", &format!("id={ID}"))]);
        assert_eq!(write_suno_id(&commented, other), "conflict-comment");
        // no id anywhere: written, and the user's own comment survives
        let own = file("own.flac", &[("COMMENT", "mastered by me")]);
        assert_eq!(write_suno_id(&own, ID), "written");
        let tags = flac::read(&own).unwrap();
        assert_eq!((tags.get("YAIOL_AIID"), tags.get("COMMENT")), (format!("suno:{ID}").as_str(), "mastered by me"));
        // Suno's own comment with no id is cleared
        let suno = file("suno.flac", &[("COMMENT", "Made with Suno")]);
        assert_eq!(write_suno_id(&suno, ID), "written");
        assert_eq!(flac::read(&suno).unwrap().get("COMMENT"), "");
    }

    #[test]
    fn the_track_number_keeps_its_shape() {
        let padded = file("n1.flac", &[("TRACKNUMBER", "07/12")]);
        assert_eq!(write_track_number(&padded, 5), "written");
        assert_eq!(flac::read(&padded).unwrap().get("TRACKNUMBER"), "05/12");
        assert_eq!(write_track_number(&padded, 5), "unchanged");
        let plain = file("n2.flac", &[("TRACKNUMBER", "7")]);
        assert_eq!(write_track_number(&plain, 12), "written");
        assert_eq!(flac::read(&plain).unwrap().get("TRACKNUMBER"), "12");
        let none = file("n3.flac", &[]);
        assert_eq!(write_track_number(&none, 3), "written");
        assert_eq!(flac::read(&none).unwrap().get("TRACKNUMBER"), "3");
        assert_eq!(write_track_number(Path::new("x.mp3"), 1), "not-flac");
    }

    #[test]
    fn an_m4a_is_written_the_same_way() {
        let dir = std::env::temp_dir().join(format!("yaiol-amlp-aiid-m4a-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let f = dir.join("a.m4a");
        fs::write(&f, crate::m4a::tests::sample(&[("COMMENT", "x")], true)).unwrap();
        assert_eq!(write_suno_id(&f, ID), "written");
        assert_eq!(read_suno_id(&f).as_deref(), Some(ID));
        assert_eq!(write_suno_id(&f, ID), "already-tagged");
        assert_eq!(write_suno_id(&f, "00000000-1111-4222-8333-444444444444"), "conflict-amlp");
        let g = dir.join("b.m4a");
        fs::write(&g, crate::m4a::tests::sample(&[], true)).unwrap();
        assert_eq!(move_suno_id_to_tag(&g), "no-id");
    }
}
