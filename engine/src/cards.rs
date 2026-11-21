//! Cards and their links: create, update, delete, reorder (with the optional rename of the media files),
//! and the web links attached to a card or a project.

use crate::{
    aiid,
    store::{Card, Project, Res, Store, err, is_song, new_id, now_ms},
};
use rusqlite::{OptionalExtension, params};
use serde::Deserialize;
use serde_json::Value;
use std::{fs, path::Path};

/// A new card. Only `id`, `kind` and `name` are required.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct NewCard {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub name: String,
    pub desc: String,
    pub note: String,
    pub style: String,
    pub reference_artist: String,
    pub reference_complement: String,
    pub lyrics: String,
    pub tags: Vec<String>,
    pub favorite: bool,
    pub date: Option<i64>,
    pub linked_cards: Vec<String>,
    pub sort_number: Option<i64>,
    pub media_path: String,
    pub env: Option<String>,
}

/// A change to a card: only what is `Some` is written.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CardPatch {
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub name: Option<String>,
    pub desc: Option<String>,
    pub note: Option<String>,
    pub style: Option<String>,
    pub reference_artist: Option<String>,
    pub reference_complement: Option<String>,
    pub lyrics: Option<String>,
    pub tags: Option<Vec<String>>,
    pub favorite: Option<bool>,
    pub linked_cards: Option<Vec<String>>,
    /// `Some(None)` cannot be told from "not given" here, as in the original: a sort number is never cleared by an update.
    pub sort_number: Option<i64>,
    pub media_path: Option<String>,
    pub ai_tweaks: Option<Value>,
    pub lang: Option<String>,
    pub env: Option<String>,
    pub date: Option<i64>,
}

impl Store {
    pub fn create_card(&self, new: &NewCard) -> Res<Card> {
        if new.id.is_empty() || new.kind.is_empty() || new.name.is_empty() {
            return Err("Missing fields".to_owned());
        }
        let env = new.env.clone().filter(|e| !e.is_empty()).unwrap_or_else(|| self.default_env.clone());
        if is_song(&new.kind) && env == self.global_env {
            return Err("Songs cannot be added to Global environment".to_owned());
        }
        let tx = self.db.unchecked_transaction().map_err(err)?;
        let version = self.next_card_version(&new.name, &new.kind, None)?;
        tx.execute(
            "INSERT INTO card(cad_id,cad_name,cad_version,cad_type,cad_desc,cad_note,cad_style,cad_ref_artist,cad_ref_complement,cad_lyrics,cad_tags,cad_favorite,cad_date,cad_order,cad_media_path,cad_envid) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
            params![
                new.id, new.name, version, new.kind, new.desc, new.note, new.style, new.reference_artist, new.reference_complement, new.lyrics,
                serde_json::to_string(&new.tags).map_err(err)?, new.favorite as i64, new.date.unwrap_or_else(now_ms), new.sort_number,
                self.to_stored(&new.media_path), env
            ],
        )
        .map_err(err)?;
        for child in &new.linked_cards {
            tx.execute("INSERT OR IGNORE INTO card_link (cal_cadid_parent,cal_cadid_child) VALUES (?,?)", params![new.id, child]).map_err(err)?;
        }
        tx.commit().map_err(err)?;
        self.card(&new.id)?.ok_or_else(|| "Not found".to_owned())
    }

    pub fn update_card(&self, id: &str, patch: &CardPatch) -> Res<Card> {
        let Some((cur_name, cur_kind, cur_media)) = self
            .db
            .query_row("SELECT cad_name,cad_type,cad_media_path FROM card WHERE cad_id=?", [id], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?)))
            .optional()
            .map_err(err)?
        else {
            return Err("Not found".to_owned());
        };
        let prev_media = self.from_stored(&cur_media);
        let name_changes = patch.name.as_ref().is_some_and(|n| !n.is_empty() && *n != cur_name);
        let kind_changes = patch.kind.as_ref().is_some_and(|k| !k.is_empty() && *k != cur_kind);
        let tx = self.db.unchecked_transaction().map_err(err)?;
        let new_version = if name_changes || kind_changes {
            Some(self.next_card_version(patch.name.as_deref().unwrap_or(&cur_name), patch.kind.as_deref().unwrap_or(&cur_kind), Some(id))?)
        } else {
            None
        };
        tx.execute(
            "UPDATE card SET cad_name=COALESCE(?,cad_name),cad_type=COALESCE(?,cad_type),cad_desc=COALESCE(?,cad_desc),cad_note=COALESCE(?,cad_note),cad_style=COALESCE(?,cad_style),cad_ref_artist=COALESCE(?,cad_ref_artist),cad_ref_complement=COALESCE(?,cad_ref_complement),cad_lyrics=COALESCE(?,cad_lyrics),cad_tags=COALESCE(?,cad_tags),cad_favorite=COALESCE(?,cad_favorite),cad_order=COALESCE(?,cad_order),cad_media_path=COALESCE(?,cad_media_path),cad_ai_tweaks=COALESCE(?,cad_ai_tweaks),cad_lang=COALESCE(?,cad_lang),cad_envid=COALESCE(?,cad_envid),cad_date=COALESCE(?,cad_date),cad_version=COALESCE(?,cad_version) WHERE cad_id=?",
            params![
                patch.name, patch.kind, patch.desc, patch.note, patch.style, patch.reference_artist, patch.reference_complement, patch.lyrics,
                patch.tags.as_ref().map(|t| serde_json::to_string(t).unwrap_or_else(|_| "[]".to_owned())),
                patch.favorite.map(|f| f as i64),
                patch.sort_number,
                patch.media_path.as_ref().map(|p| self.to_stored(p)),
                patch.ai_tweaks.as_ref().map(|v| v.to_string()),
                patch.lang, patch.env, patch.date, new_version, id
            ],
        )
        .map_err(err)?;
        if let Some(links) = &patch.linked_cards {
            tx.execute("DELETE FROM card_link WHERE cal_cadid_parent=?", [id]).map_err(err)?;
            for child in links {
                tx.execute("INSERT OR IGNORE INTO card_link (cal_cadid_parent,cal_cadid_child) VALUES (?,?)", params![id, child]).map_err(err)?;
            }
        }
        tx.commit().map_err(err)?;

        // Media newly linked: a file carrying Suno's id gives the card its Suno link.
        if let Some(media) = patch.media_path.as_deref().filter(|m| !m.is_empty() && *m != prev_media) {
            self.media_was_linked(id, media)?;
        }
        self.card(id)?.ok_or_else(|| "Not found".to_owned())
    }

    /// What follows from linking a media file to a card: the id found in the file sets (or corrects) the
    /// card's Suno link; a file with no id takes the card's own link, when the tag setting is on.
    fn media_was_linked(&self, id: &str, media: &str) -> Res<()> {
        let use_tag = self.setting("use-yaiol-aiid") == "1";
        // Before the read below, so the read sees the file's final state.
        if use_tag {
            let moved = aiid::move_suno_id_to_tag(Path::new(media));
            if !["moved", "no-id", "already-tagged", "skipped-wav", "unsupported"].contains(&moved.as_str()) {
                eprintln!("YAIOL_AIID not written for {media}: {moved}");
            }
        }
        let suno: Vec<(String, String)> = {
            let mut st = self.db.prepare("SELECT url_id,url_href FROM url WHERE url_parent_id=? AND url_type='cad' AND url_href LIKE '%suno.com/song/%' ORDER BY url_order").map_err(err)?;
            let rows = st.query_map([id], |r| Ok((r.get(0)?, r.get(1)?))).map_err(err)?;
            rows.collect::<Result<_, _>>().map_err(err)?
        };
        match aiid::read_suno_id(Path::new(media)) {
            Some(suno_id) => {
                let href = format!("https://suno.com/song/{suno_id}");
                if suno.iter().any(|(_, h)| *h == href) {
                    return Ok(());
                }
                match suno.first() {
                    // ⚠ UPDATE the card's link, never add a second one. The file is the authority, so
                    // linking it CORRECTS a card that disagrees. Adding instead is what once left a card with
                    // two Suno links, one of them another track's, which then exported as the wrong song.
                    Some((url_id, _)) => {
                        self.db.execute("UPDATE url SET url_href=? WHERE url_id=?", params![href, url_id]).map_err(err)?;
                        if suno.len() > 1 {
                            eprintln!("card {id} has {} Suno links; corrected the first, left the rest", suno.len());
                        }
                    }
                    None => {
                        let max: i64 = self.db.query_row("SELECT COALESCE(MAX(url_order),0) FROM url WHERE url_parent_id=? AND url_type='cad'", [id], |r| r.get(0)).map_err(err)?;
                        self.db
                            .execute("INSERT INTO url (url_id,url_parent_id,url_type,url_label,url_href,url_order) VALUES (?,?,?,?,?,?)", params![new_id(), id, "cad", "Suno", href, max + 1])
                            .map_err(err)?;
                    }
                }
            }
            // The file carries no id: the card's own Suno link is then the only source.
            None => {
                if let Some((_, href)) = suno.first() {
                    if use_tag {
                        aiid::stamp_suno_id_from_link(Path::new(media), href);
                    }
                }
            }
        }
        Ok(())
    }

    pub fn delete_card(&self, id: &str) -> Res<bool> {
        Ok(self.db.execute("DELETE FROM card WHERE cad_id=?", [id]).map_err(err)? > 0)
    }

    /// Give the cards the order of `ids` (1, 2, 3...). With `rename_media`, each linked file's leading
    /// `NN-` follows its new place (`07-Title.flac` becomes `05-Title.flac`), its `.lrc` moves with it,
    /// and a flac's embedded track number is set too.
    pub fn reorder_cards(&self, ids: &[String], rename_media: bool) -> Res<Vec<Card>> {
        let tx = self.db.unchecked_transaction().map_err(err)?;
        for (i, id) in ids.iter().enumerate() {
            tx.execute("UPDATE card SET cad_order=? WHERE cad_id=?", params![i as i64 + 1, id]).map_err(err)?;
        }
        tx.commit().map_err(err)?;
        if rename_media {
            for (i, id) in ids.iter().enumerate() {
                let stored: Option<String> = self.db.query_row("SELECT cad_media_path FROM card WHERE cad_id=?", [id], |r| r.get(0)).optional().map_err(err)?;
                let media = self.from_stored(&stored.unwrap_or_default());
                let old = Path::new(&media);
                if media.is_empty() || !old.exists() {
                    continue;
                }
                let stem = old.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                let ext = old.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
                let new = old.with_file_name(format!("{:02}-{}{ext}", i + 1, strip_track_prefix(&stem)));
                let mut current = old.to_path_buf();
                // a file already under that name is left alone, and so is this one
                if new != old && !new.exists() && fs::rename(old, &new).is_ok() {
                    self.db.execute("UPDATE card SET cad_media_path=? WHERE cad_id=?", params![self.to_stored(&new.to_string_lossy()), id]).map_err(err)?;
                    current = new.clone();
                    // ⚠ the lyrics file is found by swapping the media's extension for .lrc, so it must
                    // move with the media or the song reads as having no lyrics file.
                    let (old_lrc, new_lrc) = (old.with_extension("lrc"), new.with_extension("lrc"));
                    if old_lrc.exists() && !new_lrc.exists() {
                        if let Err(e) = fs::rename(&old_lrc, &new_lrc) {
                            eprintln!("LRC not renamed for {}: {e}", old_lrc.display());
                        }
                    }
                }
                // The tag follows the PLACE, not the file name: a name left behind by a collision must not
                // leave the tag behind too.
                let tagged = aiid::write_track_number(&current, i + 1);
                if !["written", "unchanged", "not-flac"].contains(&tagged.as_str()) {
                    eprintln!("Track number not written for {}: {tagged}", current.display());
                }
            }
        }
        Ok(ids.iter().filter_map(|id| self.card(id).ok().flatten()).collect())
    }

    // ─── Web links of a card or a project ────────────────────────────────────────────────────────

    /// Add a link to a card.
    // ⚠ a card holds AT MOST ONE Suno link. The link IS the song's identity, so a second one is
    // never a second fact, only a contradiction: adding one REPLACES the one there. Two is what once let
    // a card carry another track's id, which then exported as the wrong song.
    pub fn add_card_url(&self, card: &str, label: &str, href: &str) -> Res<Option<Card>> {
        if href.is_empty() {
            return Err("href required".to_owned());
        }
        let existing_suno: Option<String> = if is_suno_song_link(href) {
            self.db.query_row("SELECT url_id FROM url WHERE url_parent_id=? AND url_type='cad' AND url_href LIKE '%suno.com/song/%' ORDER BY url_order", [card], |r| r.get(0)).optional().map_err(err)?
        } else {
            None
        };
        match existing_suno {
            Some(url_id) => {
                self.db.execute("UPDATE url SET url_href=?, url_label=COALESCE(NULLIF(?,''),url_label) WHERE url_id=?", params![href, label, url_id]).map_err(err)?;
            }
            None => {
                let order: i64 = self.db.query_row("SELECT COUNT(*) FROM url WHERE url_parent_id=?", [card], |r| r.get(0)).map_err(err)?;
                self.db.execute("INSERT INTO url (url_id,url_parent_id,url_type,url_label,url_href,url_order) VALUES (?,?,?,?,?,?)", params![new_id(), card, "cad", label, href, order]).map_err(err)?;
            }
        }
        // A Suno link put on a card whose media is linked gives the file its id (when the setting is on).
        // For a Suno Studio export it is the only source: Suno strips the id on export.
        let media: Option<String> = self.db.query_row("SELECT cad_media_path FROM card WHERE cad_id=?", [card], |r| r.get(0)).optional().map_err(err)?;
        let media = self.from_stored(&media.unwrap_or_default());
        if !media.is_empty() && self.setting("use-yaiol-aiid") == "1" {
            aiid::stamp_suno_id_from_link(Path::new(&media), href);
        }
        self.card(card)
    }

    pub fn delete_card_url(&self, card: &str, url_id: &str) -> Res<Option<Card>> {
        self.db.execute("DELETE FROM url WHERE url_id=? AND url_parent_id=?", params![url_id, card]).map_err(err)?;
        self.card(card)
    }

    pub fn add_project_url(&self, project: &str, label: &str, href: &str) -> Res<Option<Project>> {
        if href.is_empty() {
            return Err("href required".to_owned());
        }
        let order: i64 = self.db.query_row("SELECT COUNT(*) FROM url WHERE url_parent_id=? AND url_type='prj'", [project], |r| r.get(0)).map_err(err)?;
        self.db.execute("INSERT INTO url (url_id,url_parent_id,url_type,url_label,url_href,url_order) VALUES (?,?,?,?,?,?)", params![new_id(), project, "prj", label, href, order]).map_err(err)?;
        self.project(project)
    }

    pub fn delete_project_url(&self, project: &str, url_id: &str) -> Res<Option<Project>> {
        self.db.execute("DELETE FROM url WHERE url_id=? AND url_parent_id=? AND url_type='prj'", params![url_id, project]).map_err(err)?;
        self.project(project)
    }
}

/// An address of the form `suno.com/song/<36 characters of an id>`.
pub fn is_suno_song_link(href: &str) -> bool {
    suno_id_in_link(href).is_some()
}

/// The song id in a Suno link, lower-case. ⚠ Exactly 36 characters are taken: a share link carries a
/// `?sh=...` after the id.
pub fn suno_id_in_link(href: &str) -> Option<String> {
    let rest = &href[href.find("suno.com/song/")? + "suno.com/song/".len()..];
    let id: String = rest.chars().take(36).collect();
    (id.len() == 36 && id.chars().all(|c| c.is_ascii_hexdigit() || c == '-')).then(|| id.to_lowercase())
}

/// A file name without its leading track number: `07 - Title`, `07_Title`, `7.Title` all give `Title`.
pub fn strip_track_prefix(stem: &str) -> &str {
    let trimmed = stem.trim_start();
    let digits = trimmed.len() - trimmed.trim_start_matches(|c: char| c.is_ascii_digit()).len();
    if digits == 0 {
        return stem;
    }
    let rest = trimmed[digits..].trim_start();
    let rest = rest.strip_prefix(['-', '_', '.']).unwrap_or(rest);
    rest.trim_start()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_track_number_is_taken_off_a_file_name() {
        assert_eq!(strip_track_prefix("07-Title"), "Title");
        assert_eq!(strip_track_prefix(" 7 - My Song"), "My Song");
        assert_eq!(strip_track_prefix("12_Title"), "Title");
        assert_eq!(strip_track_prefix("3.Title"), "Title");
        assert_eq!(strip_track_prefix("Title"), "Title");
        assert_eq!(strip_track_prefix("1999"), "");
    }
}
