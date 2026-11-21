//! Media files on disk: what counts as one, listing a folder, what a drop expands to, the lyrics file
//! beside a song, and handing a song to LRC Editor.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

/// What counts as a media file, in ONE place: the folder scan and the drop must agree, or a track the
/// browse panel lists is a track a drop ignores.
pub const MEDIA_EXTS: [&str; 9] = ["mp3", "flac", "wav", "ogg", "aac", "m4a", "opus", "mp4", "webm"];

pub fn is_media(path: &Path) -> bool {
    path.extension().is_some_and(|e| MEDIA_EXTS.contains(&e.to_string_lossy().to_lowercase().as_str()))
}

/// The media files of one folder (not below it), by name.
pub fn scan_folder(folder: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut files: Vec<PathBuf> = fs::read_dir(folder)?.flatten().map(|e| e.path()).filter(|p| p.is_file() && is_media(p)).collect();
    files.sort();
    Ok(files)
}

/// The tracks of ONE folder (an album), one path per track: the same name in several formats (a flac
/// beside its wav original) is one song, and the flac is the one kept.
pub fn folder_tracks(folder: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut by_track: BTreeMap<String, PathBuf> = BTreeMap::new();
    for file in scan_folder(folder)? {
        let key = file.file_stem().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default();
        let is_flac = file.extension().is_some_and(|e| e.to_string_lossy().eq_ignore_ascii_case("flac"));
        if !by_track.contains_key(&key) || is_flac {
            by_track.insert(key, file);
        }
    }
    let mut out: Vec<PathBuf> = by_track.into_values().collect();
    out.sort();
    Ok(out)
}

/// What was dropped on the window as one flat, ordered list of media files. A dropped FOLDER becomes the
/// tracks inside it and below (dropping a folder means "everything in here"); anything that is not media
/// falls away. The depth is capped as a guard against a loop of links or a dropped drive, not as a feature.
pub fn expand_drop(paths: &[PathBuf]) -> Vec<PathBuf> {
    fn walk(path: &Path, depth: usize, out: &mut Vec<PathBuf>) {
        if path.is_dir() {
            if depth >= 8 {
                return;
            }
            let Ok(entries) = fs::read_dir(path) else { return };
            let mut names: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
            names.sort();
            for name in names {
                walk(&name, depth + 1, out);
            }
        } else if path.is_file() && is_media(path) {
            out.push(path.to_owned());
        }
    }
    let mut out = Vec::new();
    for p in paths {
        walk(p, 0, &mut out);
    }
    out
}

/// The lyrics file of a song: its own name with `.lrc`.
pub fn lrc_of(media: &Path) -> PathBuf {
    media.with_extension("lrc")
}

/// Where LRC Editor is installed, if it is.
pub fn find_lrc_editor() -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    // beside this app: both installed under the same parent folder
    if let Some(parent) = std::env::current_exe().ok().as_deref().and_then(Path::parent).and_then(Path::parent) {
        candidates.push(parent.join("LRC Editor").join("LRC Editor.exe"));
        candidates.push(parent.join("LRC Editor").join("lrc-editor.exe"));
    }
    for (var, sub) in [("LOCALAPPDATA", "Programs"), ("PROGRAMFILES", "")] {
        if let Some(base) = std::env::var_os(var).map(PathBuf::from) {
            let dir = if sub.is_empty() { base.join("yaiol").join("LRC Editor") } else { base.join(sub).join("yaiol").join("LRC Editor") };
            candidates.push(dir.join("LRC Editor.exe"));
            candidates.push(dir.join("lrc-editor.exe"));
        }
    }
    candidates.into_iter().find(|p| p.is_file())
}

/// Open a song in LRC Editor: its `.lrc` when there is one, else its plain lyrics. `false` = LRC Editor
/// is not installed.
pub fn launch_lrc_editor(media: &str, lyrics: &str) -> bool {
    let Some(editor) = find_lrc_editor() else { return false };
    let mut handoff = serde_json::json!({ "mediaPath": media });
    let lrc = if media.is_empty() { None } else { Some(lrc_of(Path::new(media))).filter(|p| p.exists()) };
    match lrc {
        Some(lrc) => handoff["lrcPath"] = serde_json::json!(lrc.to_string_lossy()),
        None if !lyrics.is_empty() => handoff["lyrics"] = serde_json::json!(lyrics),
        None => {}
    }
    let file = std::env::temp_dir().join(format!("lrc-handoff-{}.json", crate::store::now_ms()));
    if fs::write(&file, handoff.to_string()).is_err() {
        return false;
    }
    std::process::Command::new(editor).arg(format!("--handoff={}", file.display())).spawn().is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_folder_gives_one_path_per_track_and_a_drop_expands() {
        let dir = std::env::temp_dir().join(format!("yaiol-amlp-media-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("sub/deeper")).unwrap();
        for name in ["01-A.wav", "01-A.flac", "02-B.mp3", "cover.jpg", "sub/03-C.FLAC", "sub/deeper/04-D.m4a", "sub/notes.txt"] {
            fs::write(dir.join(name), b"x").unwrap();
        }
        let names = |v: Vec<PathBuf>| v.iter().map(|p| p.file_name().unwrap().to_string_lossy().into_owned()).collect::<Vec<_>>();
        assert_eq!(names(scan_folder(&dir).unwrap()), vec!["01-A.flac", "01-A.wav", "02-B.mp3"]);
        assert_eq!(names(folder_tracks(&dir).unwrap()), vec!["01-A.flac", "02-B.mp3"]);
        assert_eq!(names(expand_drop(&[dir.clone(), dir.join("cover.jpg")])), vec!["01-A.flac", "01-A.wav", "02-B.mp3", "03-C.FLAC", "04-D.m4a"]);
        assert_eq!(lrc_of(&dir.join("02-B.mp3")), dir.join("02-B.lrc"));
        let _ = fs::remove_dir_all(dir);
    }
}
