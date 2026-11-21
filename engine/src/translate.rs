//! Translating a song with the model: the request is built from the `translate` task's template, and the answer is read into
//! the translated lyrics, the translated title and the source language the model detected.

use crate::{
    store::{Res, Store},
    wizard::{parse_response, task_request},
};
use std::collections::HashMap;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Translated {
    pub content: String,
    pub name: String,
    /// The code of the language the model says the text is in (checked against the song languages by the caller).
    pub source: String,
}

/// The request for one song. `source` is empty when the song's language is not known (the model then detects it).
pub fn request(store: &Store, name: &str, lyrics: &str, source: &str, target: &str) -> Res<String> {
    let vars = HashMap::from([("SOURCE_LANGUAGE", source.to_owned()), ("TARGET_LANGUAGE", target.to_owned()), ("SONG_NAME", name.to_owned()), ("LYRICS", lyrics.to_owned())]);
    task_request(store, crate::ai::system_blocks(), "translate", &vars)
}

/// Read the model's answer. A reply with no `CONTENT:` key is the translation as it stands.
pub fn read(raw: &str) -> Translated {
    let bag = parse_response(raw);
    let content = bag.get("CONTENT").map(|c| c.trim()).filter(|c| !c.is_empty()).unwrap_or(raw.trim()).to_owned();
    Translated { content, name: bag.get("NAME").map(|n| n.trim().to_owned()).unwrap_or_default(), source: bag.get("SOURCE").map(|s| s.trim().to_owned()).unwrap_or_default() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_answer_is_read_into_text_title_and_source() {
        let t = read("SOURCE: fr\nNAME: The Night\nCONTENT:\n[Verse]\nla nuit\n");
        assert_eq!(t, Translated { content: "[Verse]\nla nuit".into(), name: "The Night".into(), source: "fr".into() });
        assert_eq!(read(" just text ").content, "just text");
    }

    #[test]
    fn the_request_asks_for_the_target_language() {
        let dir = std::env::temp_dir().join(format!("ampl-translate-{}", crate::store::new_id()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Store::open(&dir.join("t.db"), &dir.join("s.json")).unwrap();
        store.reconcile_ai_seed().unwrap();
        let r = request(&store, "Nuit", "la nuit", "French", "Spanish").unwrap();
        assert!(r.contains("Spanish") && r.contains("la nuit"), "{r}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
