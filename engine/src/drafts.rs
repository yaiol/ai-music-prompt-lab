//! A song card made out of something that identifies a song: a Suno link in the clipboard, or a media file
//! whose own tag names its Suno song. The result is a DRAFT: a card the dialog opens pre-filled (or that a
//! bulk import saves as it is); nothing is stored here.
//!
//! Blocking calls (the network): run them off the window's thread.

use crate::{
    aiid,
    songpage::{self, PageError},
    store::{Card, Url},
};
use regex::Regex;
use std::{path::Path, sync::LazyLock};

/// A Suno link anywhere in a text, the share stub or the canonical song address. Matched loosely because a
/// copied link often arrives with text around it.
static SUNO_LINK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"https?://(?:www\.)?suno\.com/(?:s/[A-Za-z0-9_-]+|song/[0-9a-fA-F-]{36})").unwrap());
/// `10-Sanctus`: a track number, a dash, a title.
static NUMBERED: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*(\d{1,4})\s*[-–—]\s*(.+)$").unwrap());

/// `10-Sanctus` → `(Some(10), "Sanctus")`; a title without that prefix is the name.
pub fn split_numbered(title: &str) -> (Option<i64>, String) {
    match NUMBERED.captures(title) {
        Some(c) => (c[1].parse().ok(), c[2].trim().to_owned()),
        None => (None, title.trim().to_owned()),
    }
}

/// A Suno share stub expanded into the canonical song address. Anything else, or a Suno that cannot be
/// reached, comes back unchanged.
pub fn resolve_suno_link(href: &str) -> String {
    if !songpage::is_suno_share(href) {
        return href.to_owned();
    }
    match songpage::resolve_suno_share(href) {
        Ok(song) if !song.is_empty() => song,
        _ => href.to_owned(),
    }
}

/// A draft out of a RESOLVED Suno song address. `None` when it is not a song page, or the song is gone or
/// cannot be read.
pub fn from_suno_url(href: &str) -> Option<Card> {
    suno_draft(href).ok()
}

fn suno_draft(href: &str) -> Result<Card, PageError> {
    if !href.contains("/song/") {
        return Err(PageError::Other("Not a song address".to_owned()));
    }
    let song = songpage::fetch_suno(href)?;
    let (number, name) = split_numbered(&song.title);
    let date = chrono::DateTime::parse_from_rfc3339(&song.date).map(|d| d.timestamp_millis()).unwrap_or_else(|_| chrono::Utc::now().timestamp_millis());
    Ok(Card { kind: "song".to_owned(), name, sort_number: number, style: song.style, lyrics: song.lyrics, date, urls: vec![Url { id: String::new(), label: "suno".to_owned(), href: href.to_owned() }], ..Default::default() })
}

/// A draft out of a Suno link sitting in the clipboard. `None` whenever the text holds no Suno link or Suno
/// cannot be reached, so the caller just opens an empty card.
pub fn from_clipboard(text: &str) -> Option<Card> {
    let found = SUNO_LINK.find(text)?.as_str();
    from_suno_url(&resolve_suno_link(found))
}

/// A draft out of a media file. A file rendered by Suno carries its song id in its own tag, the same
/// provenance that stamps the song address onto a card when media is linked: the card arrives filled in
/// as a pasted link fills it. A file WITHOUT that tag still yields a card (the file's name split on the
/// same `NN-Title` pattern, already pointing at the file), never a dead end.
///
/// ⚠ A file that NAMES a song which no longer exists yields NOTHING (`Err(Gone)`), not the plain card: "no
/// id" means this is just some audio, "an id that answers 404" means the song was deleted or made private,
/// and a card for it would quietly invent an entry for something that is not there.
pub fn from_media(media: &Path) -> Result<Card, Gone> {
    let stem = media.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let (number, name) = split_numbered(&stem);
    let plain = Card { kind: "song".to_owned(), name, sort_number: number, media_path: media.to_string_lossy().into_owned(), ..Default::default() };
    let Some(id) = aiid::read_suno_id(media) else { return Ok(plain) };
    let mut draft = match suno_draft(&format!("https://suno.com/song/{id}")) {
        Ok(d) => d,
        Err(PageError::Gone(_)) => return Err(Gone),
        // Suno unreachable: the file is still worth a card
        Err(PageError::Other(_)) => return Ok(plain),
    };
    // the song's title on Suno rarely carries the track number; the file name does ("02-Title")
    if draft.sort_number.is_none() {
        draft.sort_number = number;
    }
    draft.media_path = plain.media_path;
    Ok(draft)
}

/// The song a file names is gone from Suno.
#[derive(Debug, PartialEq)]
pub struct Gone;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_numbered_title_gives_its_number_and_its_name() {
        assert_eq!(split_numbered("10-Sanctus"), (Some(10), "Sanctus".to_owned()));
        assert_eq!(split_numbered(" 02 – Two Words "), (Some(2), "Two Words".to_owned()));
        assert_eq!(split_numbered("No number"), (None, "No number".to_owned()));
    }

    #[test]
    fn a_suno_link_is_found_among_other_text() {
        let text = "look https://suno.com/s/AbC_d-1 nice";
        assert_eq!(SUNO_LINK.find(text).map(|m| m.as_str()), Some("https://suno.com/s/AbC_d-1"));
        assert!(SUNO_LINK.find("https://example.com/s/x").is_none());
        // a file with no id is just audio: a card named from the file, linked to it
        let d = from_media(Path::new("07-Some Song.flac")).unwrap();
        assert_eq!((d.name.as_str(), d.sort_number, d.media_path.as_str()), ("Some Song", Some(7), "07-Some Song.flac"));
    }
}
