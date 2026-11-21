//! The `.amlp` file: one LP as text, a header then one block per track.
//!
//! The grammar, which must stay in step with the tool that authors these files:
//!   `=== PROMPT NN ===`   a track starts; NN is its number
//!   `KEY:`                a field; its value follows the colon, or runs over the next lines up to the next
//!                         key or track
//!   `KEY (lang):`         a TRANSLATION of that field (TITLE and LYRICS only), kept per language
//!   LYRICS is the last ORIGINAL field of a track: it runs to the next translation key or the next track,
//!   so its blank lines and `[bracket]` marks are kept untouched, and a line that merely looks like a key
//!   stays part of the lyrics.

use serde::Serialize;

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Header {
    pub title: String,
    pub date: String,
    pub style: String,
    pub theme: String,
    pub lang: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Translation {
    pub lang: String,
    pub title: String,
    pub lyrics: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub num: i64,
    pub title: String,
    pub description: String,
    pub style: String,
    pub url: String,
    pub lang: String,
    pub reference_artist: String,
    pub reference_complement: String,
    pub lyrics: String,
    pub translations: Vec<Translation>,
}

const FIELDS: [&str; 10] = ["TITLE", "DESCRIPTION", "STYLE", "URL", "LYRICS", "DATE", "THEME", "LANG", "REFERENCE ARTIST", "REFERENCE COMPLEMENT"];

/// `=== PROMPT 07 ===` gives 7.
fn banner(line: &str) -> Option<i64> {
    let rest = line.trim_end().strip_prefix("===")?.trim_start();
    let rest = if rest.len() >= 6 && rest[..6].eq_ignore_ascii_case("prompt") { &rest[6..] } else { return None };
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }
    let rest = rest.trim_start();
    let digits = rest.len() - rest.trim_start_matches(|c: char| c.is_ascii_digit()).len();
    if digits == 0 || rest[digits..].trim_start() != "===" {
        return None;
    }
    rest[..digits].parse().ok()
}

/// `KEY: value` for a known key gives (key, value).
fn field(line: &str) -> Option<(&'static str, &str)> {
    FIELDS.iter().find_map(|key| line.strip_prefix(key).and_then(|rest| rest.strip_prefix(':')).map(|value| (*key, value.trim_start())))
}

/// `TITLE (en): value` / `LYRICS (pt-BR): value` gives (key, language, value). The tag is a language code
/// (2-3 letters, an optional region): a shape no real lyric line has, which is what lets it close a LYRICS
/// block when a bare `TITLE:` deliberately cannot.
fn translation(line: &str) -> Option<(&'static str, String, &str)> {
    let (key, rest) = ["TITLE", "LYRICS"].iter().find_map(|k| line.strip_prefix(k).map(|r| (*k, r)))?;
    let rest = rest.trim_start().strip_prefix('(')?;
    let (tag, rest) = rest.split_once(')')?;
    let value = rest.trim_start().strip_prefix(':')?.trim_start();
    let tag = tag.trim();
    let (base, region) = match tag.split_once('-') {
        Some((b, r)) => (b, Some(r)),
        None => (tag, None),
    };
    let letters = |s: &str, min: usize, max: usize| (min..=max).contains(&s.len()) && s.chars().all(|c| c.is_ascii_alphabetic());
    if !letters(base, 2, 3) || region.is_some_and(|r| !letters(r, 2, 4)) {
        return None;
    }
    // "EN" -> "en", "pt-br" -> "pt-BR"
    let lang = match region {
        Some(r) => format!("{}-{}", base.to_lowercase(), r.to_uppercase()),
        None => base.to_lowercase(),
    };
    Some((key, lang, value))
}

pub fn parse(text: &str) -> (Header, Vec<Track>) {
    let mut header = Header::default();
    let mut tracks: Vec<Track> = Vec::new();
    // the field being read: its key, the language when it is a translation, its lines so far
    let mut open: Option<(&'static str, Option<String>)> = None;
    let mut buf: Vec<&str> = Vec::new();
    // the first value of a field wins: which ones this track (or the header) already has
    let mut seen: Vec<&'static str> = Vec::new();

    fn store(header: &mut Header, tracks: &mut [Track], seen: &mut Vec<&'static str>, open: &Option<(&'static str, Option<String>)>, buf: &[&str]) {
        let Some((key, lang)) = open else { return };
        let value = buf.join("\n").trim_matches('\n').trim().to_owned();
        match (tracks.last_mut(), lang) {
            (Some(track), Some(lang)) => {
                let i = match track.translations.iter().position(|t| t.lang == *lang) {
                    Some(i) => i,
                    None => {
                        track.translations.push(Translation { lang: lang.clone(), ..Default::default() });
                        track.translations.len() - 1
                    }
                };
                let tr = &mut track.translations[i];
                let slot = if *key == "TITLE" { &mut tr.title } else { &mut tr.lyrics };
                if slot.is_empty() {
                    *slot = value;
                }
            }
            (None, Some(_)) => {} // a translation in the header means nothing
            (track, None) => {
                if seen.contains(key) {
                    return;
                }
                seen.push(*key);
                match track {
                    Some(t) => match *key {
                        "TITLE" => t.title = value,
                        "DESCRIPTION" => t.description = value,
                        "STYLE" => t.style = value,
                        "URL" => t.url = value,
                        "LYRICS" => t.lyrics = value,
                        "LANG" => t.lang = value,
                        "REFERENCE ARTIST" => t.reference_artist = value,
                        "REFERENCE COMPLEMENT" => t.reference_complement = value,
                        _ => {}
                    },
                    None => match *key {
                        "TITLE" => header.title = value,
                        "DATE" => header.date = value,
                        "STYLE" => header.style = value,
                        "THEME" => header.theme = value,
                        "LANG" => header.lang = value,
                        _ => {}
                    },
                }
            }
        }
    }

    for line in text.split('\n').map(|l| l.strip_suffix('\r').unwrap_or(l)) {
        if let Some(num) = banner(line) {
            store(&mut header, &mut tracks, &mut seen, &open, &buf);
            (open, buf) = (None, Vec::new());
            seen.clear();
            tracks.push(Track { num, ..Default::default() });
            continue;
        }
        // A translation key ends whatever came before it, LYRICS included.
        if let Some((key, lang, value)) = translation(line) {
            store(&mut header, &mut tracks, &mut seen, &open, &buf);
            open = Some((key, Some(lang)));
            buf = if value.is_empty() { Vec::new() } else { vec![value] };
            continue;
        }
        // Inside the original lyrics, nothing but a track or a translation key ends them.
        let in_lyrics = matches!(&open, Some(("LYRICS", None)));
        if !in_lyrics {
            if let Some((key, value)) = field(line) {
                store(&mut header, &mut tracks, &mut seen, &open, &buf);
                open = Some((key, None));
                buf = if value.is_empty() { Vec::new() } else { vec![value] };
                continue;
            }
        }
        if open.is_some() {
            buf.push(line);
        }
    }
    store(&mut header, &mut tracks, &mut seen, &open, &buf);
    (header, tracks)
}

#[cfg(test)]
mod tests {
    use super::*;

    const LP: &str = "TITLE: My LP\r\nLANG: French (court idiom)\nDATE: 2026-03-01\nSTYLE:\nbaroque\n\n=== PROMPT 01 ===\nTITLE: Ouverture\nDESCRIPTION: the opening\nSTYLE: slow, strings\nREFERENCE ARTIST: Lully\nURL: https://suno.com/song/abc\nLYRICS:\n[Intro]\n\nTITLE: not a field here\nline two\nTITLE (EN): Overture\nLYRICS (en):\n[Intro]\nline two in English\nLYRICS (pt-br): em portugues\n=== prompt 2 ===\nTITLE: Second\nTITLE: ignored\nLANG: Latin\nLYRICS: one line\n";

    #[test]
    fn reads_the_header_and_the_tracks() {
        let (h, t) = parse(LP);
        assert_eq!(h, Header { title: "My LP".into(), date: "2026-03-01".into(), style: "baroque".into(), theme: String::new(), lang: "French (court idiom)".into() });
        assert_eq!(t.len(), 2);
        assert_eq!((t[0].num, t[0].title.as_str(), t[0].description.as_str(), t[0].style.as_str()), (1, "Ouverture", "the opening", "slow, strings"));
        assert_eq!((t[0].reference_artist.as_str(), t[0].url.as_str()), ("Lully", "https://suno.com/song/abc"));
        // a line that looks like a key stays part of the lyrics; the blank line inside is kept
        assert_eq!(t[0].lyrics, "[Intro]\n\nTITLE: not a field here\nline two");
        assert_eq!(
            t[0].translations,
            vec![
                Translation { lang: "en".into(), title: "Overture".into(), lyrics: "[Intro]\nline two in English".into() },
                Translation { lang: "pt-BR".into(), title: String::new(), lyrics: "em portugues".into() }
            ]
        );
        // the first value of a field wins
        assert_eq!((t[1].num, t[1].title.as_str(), t[1].lang.as_str(), t[1].lyrics.as_str()), (2, "Second", "Latin", "one line"));
    }

    #[test]
    fn text_with_no_track_gives_none() {
        assert!(parse("TITLE: only a header\n").1.is_empty());
        assert!(parse("").1.is_empty());
        assert_eq!(banner("=== PROMPT 12 ==="), Some(12));
        assert_eq!(banner("=== PROMPT ==="), None);
        assert_eq!(translation("LYRICS (english): x"), None);
    }
}
