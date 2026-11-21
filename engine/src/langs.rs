//! The languages a song can be in: their codes, their English names, and those names in each language of
//! the interface. The data is the file the Electron version reads, turned into JSON once.

use serde::Deserialize;
use std::{collections::HashMap, sync::LazyLock};

#[derive(Clone, Debug, Deserialize)]
pub struct SongLanguage {
    pub code: String,
    /// In English.
    pub name: String,
}

#[derive(Deserialize)]
struct Data {
    languages: Vec<SongLanguage>,
    names: HashMap<String, HashMap<String, String>>,
}

static DATA: LazyLock<Data> = LazyLock::new(|| serde_json::from_str(include_str!("../assets/song-languages.json")).expect("song-languages.json is valid"));

pub fn song_languages() -> &'static [SongLanguage] {
    &DATA.languages
}

/// A language's name in the interface language `ui`, else in English, else its code.
pub fn lang_name(code: &str, ui: &str) -> String {
    if let Some(name) = DATA.names.get(ui).and_then(|m| m.get(code)) {
        return name.clone();
    }
    english_name(code).unwrap_or(code).to_owned()
}

pub fn english_name(code: &str) -> Option<&'static str> {
    DATA.languages.iter().find(|l| l.code == code).map(|l| l.name.as_str())
}

/// The code of the language a free text STARTS with (`French (17th-century court idiom)` gives `fr`): the
/// longest known English name it begins with, ending at a word boundary. Empty when none.
pub fn code_of_language_text(raw: &str) -> String {
    let s = raw.trim().to_lowercase();
    if s.is_empty() {
        return String::new();
    }
    let mut best: Option<&SongLanguage> = None;
    for l in &DATA.languages {
        let n = l.name.to_lowercase();
        let boundary = s.len() == n.len() || s[n.len().min(s.len())..].starts_with([' ', '\t', ',', '(']);
        if s.starts_with(&n) && boundary && best.is_none_or(|b| n.len() > b.name.len()) {
            best = Some(l);
        }
    }
    best.map(|l| l.code.clone()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_codes() {
        assert!(song_languages().len() > 200);
        assert_eq!(english_name("fr"), Some("French"));
        assert_eq!(lang_name("ab", "fr"), "Abkhaze");
        assert_eq!(lang_name("ab", "zz"), "Abkhaz");
        assert_eq!(lang_name("no-such", "fr"), "no-such");
        assert_eq!(code_of_language_text("French (17th-century court idiom)"), "fr");
        assert_eq!(code_of_language_text("  latin"), "la");
        assert_eq!(code_of_language_text("Old Norse"), "");
        assert_eq!(code_of_language_text("Frenchy"), "");
    }
}
