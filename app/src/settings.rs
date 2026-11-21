//! Persisted user settings: language and theme, in `settings.json` under the OS app-data folder.
//!
//! The folder mirrors the electron convention exactly: `…\AppData\Roaming\yaiol\<Product Name>`,
//! with a ` (Dev)` suffix on a debug build so a dev run and an installed copy never share state.
//! The file name is plain (`settings.json`) — the folder is already per-app, so there is no
//! storage-prefix to namespace it.

use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub lang: String,
    pub theme: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self { lang: "en".to_owned(), theme: "light".to_owned(), }
    }
}

/// This app's own folder under the OS app-data folder.
pub fn data_dir() -> Option<PathBuf> {
    let folder = if cfg!(debug_assertions) {
        format!("{} (Dev)", crate::APP_NAME)
    } else {
        crate::APP_NAME.to_owned()
    };
    Some(dirs::data_dir()?.join("yaiol").join(folder))
}

// ⚠ not `settings.json`: that name is the store's own settings file, in this same folder, and this file would
// overwrite it. The first builds named it `ui-rs.json`: taken over under the real name, once.
fn path() -> Option<PathBuf> {
    let dir = data_dir()?;
    let p = dir.join("ui.json");
    if !p.exists() && dir.join("ui-rs.json").exists() {
        let _ = fs::rename(dir.join("ui-rs.json"), &p);
    }
    Some(p)
}

pub fn load() -> Settings {
    path().map(|p| load_from(&p)).unwrap_or_default()
}

pub fn save(settings: &Settings) {
    if let Some(p) = path() {
        save_to(&p, settings);
    }
}

fn load_from(path: &Path) -> Settings {
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn save_to(path: &Path, settings: &Settings) {
    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }
    if let Ok(text) = serde_json::to_string_pretty(settings) {
        let _ = fs::write(path, text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("yaiol-settings-test-{}-{name}", std::process::id()))
    }

    #[test]
    fn round_trips() {
        let p = temp("roundtrip").join("settings.json");
        let s = Settings { lang: "ar".to_owned(), theme: "dark".to_owned() };
        save_to(&p, &s);
        assert!(load_from(&p) == s);
        let _ = fs::remove_dir_all(p.parent().unwrap());
    }

    #[test]
    fn missing_or_broken_file_gives_defaults() {
        assert!(load_from(&temp("missing")) == Settings::default());
        let p = temp("broken");
        fs::write(&p, "{ not json").unwrap();
        assert!(load_from(&p) == Settings::default());
        let _ = fs::remove_file(p);
    }

    #[test]
    fn a_partial_file_keeps_the_rest_at_defaults() {
        let p = temp("partial");
        fs::write(&p, r#"{ "theme": "dark" }"#).unwrap();
        let s = load_from(&p);
        assert_eq!((s.lang.as_str(), s.theme.as_str()), ("en", "dark"));
        let _ = fs::remove_file(p);
    }
}
