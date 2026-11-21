//! Taking data out as text: the `.amlp` LP file, a Markdown or AsciiDoc songbook, and the JSON file
//! (prompts, or a backup of everything). The window picks the file to write; these build its content.

use crate::{
    langs,
    store::{Card, Res, Store, err},
};
use rusqlite::OptionalExtension;
use serde_json::{Value, json};
use std::path::Path;

/// Does the song have words? Lyrics made only of structure marks (`[Intro]`, `(chorus)`) are an
/// instrumental.
pub fn has_real_lyrics(raw: &str) -> bool {
    raw.lines().map(str::trim).any(|t| !t.is_empty() && !is_marker(t))
}

fn is_marker(line: &str) -> bool {
    (line.starts_with('[') && line.ends_with(']')) || (line.starts_with('(') && line.ends_with(')'))
}

/// The lyric lines to show. `clean` drops the structure marks; runs of blank lines become one; blank
/// lines at the end are dropped. ⚠ The CONTENT is the same for every export format: it is decided here
/// once, and each format only wraps these lines in its own syntax.
pub fn extract_lyrics(raw: &str, clean: bool) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut last_blank = true;
    for line in raw.split('\n') {
        let t = line.trim();
        if t.is_empty() {
            if !last_blank {
                out.push(String::new());
                last_blank = true;
            }
            continue;
        }
        if clean && is_marker(t) {
            continue;
        }
        out.push(t.to_owned());
        last_blank = false;
    }
    while out.last().is_some_and(|l| l.is_empty()) {
        out.pop();
    }
    out
}

/// A SONG's lyric lines: one with no actual words is an instrumental in both modes.
pub fn song_lyrics_lines(raw: &str, clean: bool, instrumental_label: &str) -> Vec<String> {
    if has_real_lyrics(raw) { extract_lyrics(raw, clean) } else { vec![format!("({instrumental_label})")] }
}

/// Lyrics with the structure marks taken out: `[Verse]` and `(echo)` go, the spaces they leave at a
/// line's start go, three blank lines become two. What "Copy lyrics without comments" puts on the clipboard.
pub fn clean_lyrics(text: &str) -> String {
    use regex::Regex;
    use std::sync::OnceLock;
    static RES: OnceLock<[Regex; 4]> = OnceLock::new();
    let [square, round, indent, blanks] =
        RES.get_or_init(|| [Regex::new(r"\[.*?\]").unwrap(), Regex::new(r"\(.*?\)").unwrap(), Regex::new(r"\n +").unwrap(), Regex::new(r"\n{3,}").unwrap()]);
    let s = text.replace('\r', "");
    let s = square.replace_all(&s, "");
    let s = round.replace_all(&s, "");
    let s = indent.replace_all(&s, "\n");
    let s = blanks.replace_all(&s, "\n\n");
    s.trim_matches('\n').to_owned()
}

/// A name made safe for a file.
pub fn safe_file_name(name: &str) -> String {
    name.chars().map(|c| if matches!(c, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|') { '_' } else { c }).collect()
}

/// What a songbook export contains.
#[derive(Clone, Debug)]
pub struct DocOptions {
    pub project_ids: Vec<String>,
    /// Drop the structure marks from the lyrics.
    pub clean_lyrics: bool,
    /// The translations to include, by language code.
    pub langs: Vec<String>,
    pub images: bool,
    pub desc: bool,
    /// Only songs that have words.
    pub with_lyrics_only: bool,
    /// Only songs that have a link on a music site.
    pub published_only: bool,
    /// The interface language, for language names.
    pub ui_lang: String,
    pub translation_label: String,
    pub instrumental_label: String,
}

/// Why a songbook has nothing to write.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoSongs {
    None,
    /// There are songs, but none passes the filters.
    Filtered,
}

impl Store {
    pub(crate) fn music_sites(&self) -> Vec<String> {
        let raw = self.setting("music-sites");
        let raw = if raw.is_empty() { "soundcloud".to_owned() } else { raw };
        raw.split(',').map(|s| s.trim().to_lowercase()).filter(|s| !s.is_empty()).collect()
    }

    fn songs_of(&self, project_ids: &[String]) -> Res<Vec<Card>> {
        let mut st = self.db.prepare("SELECT * FROM card WHERE cad_type='song' ORDER BY cad_order, cad_name").map_err(err)?;
        let rows = st.query_map([], |r| self.card_from_row(r)).map_err(err)?;
        let all: Vec<Card> = rows.collect::<Result<_, _>>().map_err(err)?;
        Ok(all.into_iter().filter(|c| project_ids.is_empty() || c.project.as_ref().is_some_and(|p| project_ids.contains(p))).collect())
    }

    /// The songs a songbook holds, filtered and in order, with the file name and folder to propose.
    fn doc_songs(&self, o: &DocOptions) -> Res<Result<(Vec<Card>, String, String), NoSongs>> {
        let mut songs = self.songs_of(&o.project_ids)?;
        if songs.is_empty() {
            return Ok(Err(NoSongs::None));
        }
        let sites = self.music_sites();
        if o.with_lyrics_only {
            songs.retain(|s| has_real_lyrics(&s.lyrics));
        }
        if o.published_only {
            songs.retain(|s| s.urls.iter().any(|u| sites.contains(&u.label.trim().to_lowercase())));
        }
        if songs.is_empty() {
            return Ok(Err(NoSongs::Filtered));
        }
        // by sort number (a song with none goes last), then by name
        songs.sort_by(|a, b| {
            let key = |c: &Card| c.sort_number.filter(|n| *n != 0).map(|n| n as f64).unwrap_or(f64::INFINITY);
            key(a).total_cmp(&key(b)).then_with(|| a.name.cmp(&b.name))
        });
        let (mut name, mut dir) = ("songs".to_owned(), String::new());
        if let [only] = o.project_ids.as_slice() {
            if let Some(p) = self.project(only)? {
                name = safe_file_name(&p.name);
                dir = p.path;
            }
        }
        Ok(Ok((songs, name, dir)))
    }

    /// A songbook as Markdown (`adoc: false`) or AsciiDoc. Returns (content, proposed name, proposed folder).
    pub fn export_songbook(&self, o: &DocOptions, adoc: bool) -> Res<Result<(String, String, String), NoSongs>> {
        let (songs, name, dir) = match self.doc_songs(o)? {
            Ok(found) => found,
            Err(none) => return Ok(Err(none)),
        };
        let sites = self.music_sites();
        let mut parts: Vec<String> = Vec::new();
        // the album cover beside the first song that has a file
        if o.images {
            if let Some(media) = songs.iter().map(|s| &s.media_path).find(|m| !m.is_empty()) {
                if Path::new(media).with_file_name("folder.jpg").exists() {
                    parts.push(if adoc { "image::folder.jpg[]\n".to_owned() } else { "![](folder.jpg)\n".to_owned() });
                }
            }
        }
        let fence = if adoc { "...." } else { "```" };
        let block_of = |lines: &[String]| format!("\n{fence}\n{}\n{fence}\n", lines.join("\n"));
        for song in &songs {
            let heading = match song.sort_number.filter(|n| *n != 0) {
                Some(n) => format!("{n}. {}", song.name),
                None => song.name.clone(),
            };
            let mut block = format!("{} {heading}\n", if adoc { "==" } else { "##" });
            if !song.lang.is_empty() {
                block.push_str(&format!("\n_{}_\n", langs::lang_name(&song.lang, &o.ui_lang)));
            }
            let published: Vec<String> = song
                .urls
                .iter()
                .filter(|u| sites.contains(&u.label.trim().to_lowercase()))
                .map(|u| if adoc { format!("link:{}[{}]", u.href, u.label) } else { format!("[{}]({})", u.label, u.href) })
                .collect();
            if !published.is_empty() {
                block.push_str(&format!("\n{}\n", published.join(if adoc { " +\n" } else { "  \n" })));
            }
            if o.desc && !song.desc.is_empty() {
                let brk = if adoc { " +" } else { "  " };
                let desc = song.desc.split('\n').map(|l| format!("{}{brk}", l.trim_end())).collect::<Vec<_>>().join("\n");
                block.push_str(&format!("\n{}\n", desc.trim_end()));
            }
            let lines = song_lyrics_lines(&song.lyrics, o.clean_lyrics, &o.instrumental_label);
            if !lines.is_empty() {
                block.push_str(&block_of(&lines));
            }
            if !o.langs.is_empty() {
                for tr in self.translations(&song.id)?.into_iter().filter(|t| o.langs.contains(&t.lang)) {
                    block.push_str(&format!("\n{} {} - {}\n", if adoc { "===" } else { "###" }, o.translation_label, langs::lang_name(&tr.lang, &o.ui_lang)));
                    let lines = extract_lyrics(&tr.content, o.clean_lyrics);
                    if !lines.is_empty() {
                        block.push_str(&block_of(&lines));
                    }
                }
            }
            parts.push(block);
        }
        Ok(Ok((parts.join("\n---\n\n"), name, dir)))
    }

    /// How many songs a songbook with these options holds (the hint in the Create document dialog).
    pub fn doc_song_count(&self, o: &DocOptions) -> usize {
        match self.doc_songs(o) {
            Ok(Ok((songs, _, _))) => songs.len(),
            _ => 0,
        }
    }

    /// A songbook as a Word file: one page per song, the cover picture on top (the picture the song's file carries, else the
    /// `folder.jpg` beside it; not repeated while it is the same as on the song before). Returns (bytes, proposed name, proposed folder).
    pub fn export_songbook_docx(&self, o: &DocOptions) -> Res<Result<(Vec<u8>, String, String), NoSongs>> {
        use crate::docx::{Block, Picture, Run};
        let (songs, name, dir) = match self.doc_songs(o)? {
            Ok(found) => found,
            Err(none) => return Ok(Err(none)),
        };
        // the content is 673 px wide: A4 less the margins, at 96 per inch
        const WIDTH: u32 = 673;
        let cover_of = |media: &str| -> Option<Picture> {
            let path = Path::new(media);
            if !path.exists() {
                return None;
            }
            let from_tags = lofty::read_from_path(path).ok().and_then(|f| {
                use lofty::{file::TaggedFileExt, picture::MimeType};
                let tag = f.primary_tag().or_else(|| f.first_tag())?;
                let pic = tag.pictures().first()?;
                let ext = match pic.mime_type() {
                    Some(MimeType::Png) => "png",
                    Some(MimeType::Gif) => "gif",
                    Some(MimeType::Bmp) => "bmp",
                    Some(MimeType::Jpeg) | None => "jpg",
                    _ => return None,
                };
                Some((pic.data().to_vec(), ext))
            });
            let (data, ext) = from_tags.or_else(|| std::fs::read(path.with_file_name("folder.jpg")).ok().map(|d| (d, "jpg")))?;
            let h = crate::docx::image_size(&data, ext == "png").map(|(w, h)| (WIDTH as u64 * h as u64 / w.max(1) as u64) as u32).unwrap_or(WIDTH);
            Some(Picture { data, ext, w: WIDTH, h })
        };
        let run = |text: String, size: u32, font: &'static str, bold: bool, italic: bool, colour: &'static str| Run { text, size, font, bold, italic, colour };
        let lyric_blocks = |lines: &[String]| -> Vec<Block> {
            lines
                .iter()
                .map(|line| {
                    let t = line.trim();
                    if t.is_empty() {
                        return Block::Blank { before: 60, after: 60, page_break: false };
                    }
                    let marker = is_marker(t);
                    Block::Text {
                        runs: vec![run(t.to_owned(), if marker { 20 } else { 24 }, "Calibri", marker, marker, if marker { "999999" } else { "000000" })],
                        before: if marker { 200 } else { 0 },
                        after: if marker { 60 } else { 40 },
                        page_break: false,
                    }
                })
                .collect()
        };
        let mut blocks: Vec<Block> = Vec::new();
        let mut last_cover: Option<Vec<u8>> = None;
        for (i, song) in songs.iter().enumerate() {
            if i > 0 {
                blocks.push(Block::Blank { before: 0, after: 0, page_break: true });
            }
            if o.images && !song.media_path.is_empty() {
                if let Some(cover) = cover_of(&song.media_path) {
                    if last_cover.as_ref() != Some(&cover.data) {
                        last_cover = Some(cover.data.clone());
                        blocks.push(Block::Picture { picture: cover, before: 0, after: 360 });
                    }
                }
            }
            let title = match song.sort_number.filter(|n| *n != 0) {
                Some(n) => format!("{n}. {}", song.name),
                None => song.name.clone(),
            };
            blocks.push(Block::Text { runs: vec![run(title, 48, "Calibri Light", true, false, "7c6fff")], before: 0, after: 480, page_break: false });
            if o.desc && !song.desc.is_empty() {
                blocks.push(Block::Text { runs: vec![run(song.desc.clone(), 22, "Calibri", false, true, "555555")], before: 0, after: 240, page_break: false });
            }
            blocks.extend(lyric_blocks(&song_lyrics_lines(&song.lyrics, o.clean_lyrics, &o.instrumental_label)));
            if !o.langs.is_empty() {
                for tr in self.translations(&song.id)?.into_iter().filter(|t| o.langs.contains(&t.lang)) {
                    let label = format!("── {} ({}) ──", langs::lang_name(&tr.lang, &o.ui_lang), tr.lang);
                    blocks.push(Block::Text { runs: vec![run(label, 20, "Calibri", false, true, "888888")], before: 480, after: 200, page_break: false });
                    blocks.extend(lyric_blocks(&extract_lyrics(&tr.content, o.clean_lyrics)));
                }
            }
        }
        Ok(Ok((crate::docx::write(&blocks)?, name, dir)))
    }

    /// The `.amlp` of the given songs: the exact inverse of the import. `project` only gives the file its
    /// identity (name, folder, date), never its content: the window decides which songs go out.
    /// Returns (content, proposed name, proposed folder, how many songs); `None` when no song is given.
    // ⚠ LYRICS is the LAST original field of a track (it runs to the next track, so a field written
    // after it would be read back as lyric text). The only thing allowed after it is the translations.
    // ⚠ LANG is written with the ENGLISH language name, never the interface's: the import matches English
    // names only, so a localized name would give a file whose language fails to come back.
    pub fn export_amlp(&self, card_ids: &[String], project: Option<&str>) -> Res<Option<(String, String, String, usize)>> {
        let mut st = self.db.prepare("SELECT * FROM card WHERE cad_type='song' ORDER BY cad_order, cad_name").map_err(err)?;
        let rows = st.query_map([], |r| self.card_from_row(r)).map_err(err)?;
        let songs: Vec<Card> = rows.collect::<Result<Vec<Card>, _>>().map_err(err)?.into_iter().filter(|c| card_ids.contains(&c.id)).collect();
        if songs.is_empty() {
            return Ok(None);
        }
        let day = |ms: i64| chrono::DateTime::from_timestamp_millis(ms).map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default();
        let (mut name, mut date, mut dir) = ("songs".to_owned(), String::new(), String::new());
        if let Some(p) = project.filter(|p| !p.is_empty()).map(|p| self.project(p)).transpose()?.flatten() {
            name = p.name;
            date = day(if p.date != 0 { p.date } else { crate::store::now_ms() });
            dir = p.path;
        }
        if date.is_empty() {
            date = day(crate::store::now_ms());
        }
        let lang = songs.iter().map(|s| s.lang.as_str()).find(|l| !l.is_empty()).unwrap_or("").to_owned();
        let english = |code: &str| langs::lang_name(code, "en");
        let mut out: Vec<String> = vec![format!("TITLE: {name}"), format!("DATE: {date}"), "STYLE: ".to_owned(), "THEME: ".to_owned(), format!("LANG: {}", if lang.is_empty() { String::new() } else { english(&lang) }), String::new()];
        for (i, song) in songs.iter().enumerate() {
            // The style is the song's own plus the styles of the music / vocal cards linked to it: what the
            // card's "copy with linked" puts on the clipboard, i.e. what actually goes into Suno.
            let linked: Vec<String> = {
                let mut st = self.db.prepare("SELECT cad_style FROM card WHERE cad_id IN (SELECT cal_cadid_child FROM card_link WHERE cal_cadid_parent=?)").map_err(err)?;
                let rows = st.query_map([&song.id], |r| r.get::<_, String>(0)).map_err(err)?;
                rows.collect::<Result<_, _>>().map_err(err)?
            };
            let styles: Vec<&str> = std::iter::once(song.style.as_str()).chain(linked.iter().map(String::as_str)).map(str::trim).filter(|s| !s.is_empty()).collect();
            let ai_url = song.urls.iter().find(|u| {
                let h = u.href.to_lowercase();
                ["suno.com", "producer.ai", "tunee.ai", "mozartai.com"].iter().any(|site| h.contains(site))
            });
            let num = song.sort_number.unwrap_or(i as i64 + 1);
            out.extend([format!("=== PROMPT {num:02} ==="), "TITLE:".to_owned(), song.name.clone(), String::new(), "DESCRIPTION:".to_owned(), song.desc.clone(), String::new(), "STYLE:".to_owned(), styles.join("\n"), String::new()]);
            if !song.reference_artist.is_empty() {
                out.extend(["REFERENCE ARTIST:".to_owned(), song.reference_artist.clone(), String::new()]);
            }
            if !song.reference_complement.is_empty() {
                out.extend(["REFERENCE COMPLEMENT:".to_owned(), song.reference_complement.clone(), String::new()]);
            }
            // only when this song sings something other than the header's language
            if !song.lang.is_empty() && song.lang != lang {
                out.extend(["LANG:".to_owned(), english(&song.lang), String::new()]);
            }
            if let Some(u) = ai_url {
                out.extend(["URL:".to_owned(), u.href.clone(), String::new()]);
            }
            out.extend(["LYRICS:".to_owned(), song.lyrics.clone(), String::new()]);
            let mut translations = self.translations(&song.id)?;
            translations.sort_by(|a, b| a.lang.cmp(&b.lang));
            for tr in translations {
                if !tr.name.is_empty() {
                    out.extend([format!("TITLE ({}):", tr.lang), tr.name.clone(), String::new()]);
                }
                if !tr.content.is_empty() {
                    out.extend([format!("LYRICS ({}):", tr.lang), tr.content.clone(), String::new()]);
                }
            }
        }
        Ok(Some((out.join("\n"), safe_file_name(&name), dir, songs.len())))
    }

    // ─── JSON ────────────────────────────────────────────────────────────────────────────────────
    // ⚠ every column of `card`, `project` and `card_translation` must be written here AND read in
    // `transfer::import_json`. A field left out goes out and comes back empty, with no sign of it.

    /// A path as a JSON file carries it: in Relative mode the part below the media root, so the file
    /// moves between machines; else in full.
    fn portable(&self, path: &str) -> String {
        self.to_stored(path)
    }

    fn card_json(&self, c: &Card, backup: bool) -> Res<Value> {
        let translations: Vec<Value> = self.translations(&c.id)?.into_iter().map(|t| json!({ "lang": t.lang, "name": t.name, "content": t.content })).collect();
        let mut v = json!({
            "id": c.id, "type": c.kind, "name": c.name, "version": c.version, "order": c.sort_number, "desc": c.desc, "note": c.note, "style": c.style,
            "referenceArtist": c.reference_artist, "referenceComplement": c.reference_complement, "lyrics": c.lyrics,
            "tags": c.tags, "favorite": c.favorite, "date": c.date, "lang": c.lang, "aiTweaks": c.ai_tweaks,
            "cards": c.linked_cards, "urls": c.urls, "media": self.portable(&c.media_path),
        });
        // a backup records the environment (its projects list their cards); a prompt file records the project
        if backup {
            v["env"] = json!(c.env);
        } else {
            v["project"] = json!(c.project);
        }
        if !translations.is_empty() {
            v["translations"] = Value::Array(translations);
        }
        Ok(v)
    }

    /// The JSON of the given cards and projects (`backup: false`), or a backup: the same with the
    /// environments, where every card and project also says which environment it is in. `version` is the
    /// app's version; the file names the day it was written.
    pub fn export_json(&self, card_ids: &[String], project_ids: &[String], backup: bool, version: &str) -> Res<Value> {
        let cards: Vec<Card> = self.cards()?.into_iter().filter(|c| card_ids.contains(&c.id)).collect();
        let projects: Vec<Value> = self
            .projects()?
            .into_iter()
            .filter(|p| project_ids.contains(&p.id))
            .map(|p| {
                let mut v = json!({
                    "id": p.id, "name": p.name, "version": p.version, "color": p.color, "date": p.date, "parent": p.parent_id, "cards": p.card_ids,
                    "urls": p.urls, "path": self.portable(&p.path), "finalized": p.finalized,
                });
                if backup {
                    v["env"] = json!(p.env);
                }
                v
            })
            .collect();
        let cards_json: Vec<Value> = cards.iter().map(|c| self.card_json(c, backup)).collect::<Res<_>>()?;
        let date = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let mut out = json!({ "version": version, "date": date, "cards": cards_json, "projects": projects });
        if backup {
            out["type"] = json!("backup");
            out["environments"] = serde_json::to_value(self.environments()?).map_err(err)?;
        }
        Ok(out)
    }

    /// The user's own AI presets and templates as a JSON file (the built-in ones are never exported: they
    /// come with the app).
    pub fn export_ai_json(&self, version: &str) -> Res<Value> {
        let presets: Vec<Value> = self.ai_presets(None, None)?.into_iter().filter(|p| !p.is_default).map(|p| json!({ "id": p.id, "task": p.task, "block": p.block, "name": p.name, "content": p.content })).collect();
        let templates: Vec<Value> = self
            .ai_templates(None)?
            .into_iter()
            .filter(|t| !t.is_default)
            .map(|t| json!({ "id": t.id, "task": t.task, "name": t.name, "blocks": t.blocks.iter().map(|b| json!({ "block": b.block, "presetId": b.preset_id, "enabled": b.enabled })).collect::<Vec<_>>() }))
            .collect();
        let date = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        Ok(json!({ "version": version, "date": date, "aiPresets": presets, "aiTemplates": templates }))
    }

    /// Is there a card of this id? (For a check before an import.)
    pub fn card_exists(&self, id: &str) -> Res<bool> {
        Ok(self.db.query_row("SELECT 1 FROM card WHERE cad_id=?", [id], |_| Ok(())).optional().map_err(err)?.is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lyrics_are_cleaned_the_same_way_for_every_format() {
        let raw = "[Intro]\n\n\nfirst line  \n(softly)\n\n\nsecond line\n\n[Outro]\n\n";
        assert_eq!(extract_lyrics(raw, true), vec!["first line", "", "second line"]);
        assert_eq!(extract_lyrics(raw, false), vec!["[Intro]", "", "first line", "(softly)", "", "second line", "", "[Outro]"]);
        assert!(has_real_lyrics(raw));
        assert!(!has_real_lyrics("[Intro]\n(instrumental)\n\n[Outro]"));
        assert_eq!(song_lyrics_lines("[Intro]\n[Outro]", false, "Instrumental"), vec!["(Instrumental)"]);
        assert_eq!(safe_file_name("a/b: c?"), "a_b_ c_");
    }
}
