//! Projects (folders), environments, and moving cards or a whole folder from one environment to another.

use crate::store::{Environment, Project, Res, Store, err, is_song, new_id, now_ms, relative_to_root};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::Path,
};

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct NewProject {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
    pub date: Option<i64>,
    pub parent_id: Option<String>,
    pub env: Option<String>,
    pub path: String,
    /// (label, address)
    pub urls: Vec<(String, String)>,
    pub finalized: bool,
}

/// A change to a project: only what is `Some` is written. `parent_id: Some(None)` moves it to the top level.
#[derive(Clone, Debug, Default)]
pub struct ProjectPatch {
    pub name: Option<String>,
    pub color: Option<String>,
    pub date: Option<i64>,
    pub parent_id: Option<Option<String>>,
    pub path: Option<String>,
    pub finalized: Option<bool>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MovedCards {
    pub moved_ids: Vec<String>,
    pub duplicated_ids: Vec<String>,
    pub linked_moved_count: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MovedFolder {
    pub moved_project_ids: Vec<String>,
    pub moved_card_ids: Vec<String>,
    pub duplicated_card_ids: Vec<String>,
}

impl Store {
    pub fn create_project(&self, new: &NewProject) -> Res<Project> {
        if new.id.is_empty() || new.name.is_empty() {
            return Err("Missing fields".to_owned());
        }
        let version = self.next_project_version(&new.name, None)?;
        let env = new.env.clone().filter(|e| !e.is_empty()).unwrap_or_else(|| self.default_env.clone());
        self.db
            .execute(
                "INSERT INTO project (prj_id,prj_name,prj_color,prj_date,prj_version,prj_parent,prj_env,prj_path,prj_finalized) VALUES (?,?,?,?,?,?,?,?,?)",
                params![new.id, new.name, new.color.as_deref().unwrap_or("#7c6fff"), new.date.unwrap_or_else(now_ms), version, new.parent_id, env, self.to_stored(&new.path), new.finalized as i64],
            )
            .map_err(err)?;
        for (i, (label, href)) in new.urls.iter().enumerate() {
            self.db
                .execute("INSERT INTO url (url_id,url_parent_id,url_type,url_label,url_href,url_order) VALUES (?,?,?,?,?,?)", params![new_id(), new.id, "prj", label, href, i as i64])
                .map_err(err)?;
        }
        self.project(&new.id)?.ok_or_else(|| "Not found".to_owned())
    }

    /// Update a project. Returns it with how many cards were re-pointed at a file under the new media folder.
    pub fn update_project(&self, id: &str, patch: &ProjectPatch) -> Res<(Project, usize)> {
        let Some((cur_name, cur_path)) =
            self.db.query_row("SELECT prj_name, prj_path FROM project WHERE prj_id=?", [id], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))).optional().map_err(err)?
        else {
            return Err("Not found".to_owned());
        };
        let new_version = match &patch.name {
            Some(name) if !name.is_empty() && *name != cur_name => Some(self.next_project_version(name, Some(id))?),
            _ => None,
        };
        self.db
            .execute(
                "UPDATE project SET prj_name=COALESCE(?,prj_name),prj_color=COALESCE(?,prj_color),prj_date=COALESCE(?,prj_date),prj_version=COALESCE(?,prj_version) WHERE prj_id=?",
                params![patch.name, patch.color, patch.date, new_version, id],
            )
            .map_err(err)?;
        if let Some(parent) = &patch.parent_id {
            self.db.execute("UPDATE project SET prj_parent=? WHERE prj_id=?", params![parent, id]).map_err(err)?;
        }
        if let Some(path) = &patch.path {
            self.db.execute("UPDATE project SET prj_path=? WHERE prj_id=?", params![self.to_stored(path), id]).map_err(err)?;
        }
        if let Some(finalized) = patch.finalized {
            self.db.execute("UPDATE project SET prj_finalized=? WHERE prj_id=?", params![finalized as i64, id]).map_err(err)?;
        }

        // The media folder changed: re-point the cards whose file is GONE. A card that still works is never
        // moved by a folder edit. Two passes, the exact one first:
        //   1. the folder was renamed or moved: a file that sat under the OLD folder is looked for at the
        //      same place under the NEW one (this is what makes a folder rename one edit);
        //   2. whatever is still missing is matched by file name anywhere below the new folder.
        let mut relocated = 0;
        if let Some(new_folder) = patch.path.as_deref().filter(|p| !p.is_empty() && Path::new(p).exists()) {
            let rows: Vec<(String, String)> = {
                let mut st = self.db.prepare("SELECT cad_id, cad_media_path FROM card WHERE cad_prjid=? AND cad_media_path != ''").map_err(err)?;
                let rows = st.query_map([id], |r| Ok((r.get(0)?, r.get(1)?))).map_err(err)?;
                rows.collect::<Result<_, _>>().map_err(err)?
            };
            let mut orphans: Vec<(String, String)> = rows.into_iter().map(|(cid, p)| (cid, self.from_stored(&p))).filter(|(_, abs)| !Path::new(abs).exists()).collect();
            if !orphans.is_empty() {
                let tx = self.db.unchecked_transaction().map_err(err)?;
                let mut repoint = |card: &str, to: &Path| -> Res<()> {
                    tx.execute("UPDATE card SET cad_media_path=? WHERE cad_id=?", params![self.to_stored(&to.to_string_lossy()), card]).map_err(err)?;
                    relocated += 1;
                    Ok(())
                };
                let old_folder = self.from_stored(&cur_path);
                if !old_folder.is_empty() {
                    let mut still = Vec::new();
                    for (card, abs) in orphans {
                        match relative_to_root(&abs, &old_folder).map(|rel| Path::new(new_folder).join(rel)).filter(|to| to.exists()) {
                            Some(to) => repoint(&card, &to)?,
                            None => still.push((card, abs)),
                        }
                    }
                    orphans = still;
                }
                if !orphans.is_empty() {
                    let mut by_name: HashMap<String, std::path::PathBuf> = HashMap::new();
                    index_files(Path::new(new_folder), &mut by_name);
                    for (card, abs) in &orphans {
                        let name = Path::new(abs).file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
                        if let Some(to) = by_name.get(&name) {
                            repoint(card, to)?;
                        }
                    }
                }
                tx.commit().map_err(err)?;
            }
        }
        Ok((self.project(id)?.ok_or_else(|| "Not found".to_owned())?, relocated))
    }

    /// Delete a project; with `delete_cards` its cards go too (else they simply leave it). Returns how many cards were deleted.
    pub fn delete_project(&self, id: &str, delete_cards: bool) -> Res<usize> {
        let tx = self.db.unchecked_transaction().map_err(err)?;
        let cards = if delete_cards { tx.execute("DELETE FROM card WHERE cad_prjid=?", [id]).map_err(err)? } else { 0 };
        let gone = tx.execute("DELETE FROM project WHERE prj_id=?", [id]).map_err(err)?;
        tx.commit().map_err(err)?;
        if gone == 0 { Err("Not found".to_owned()) } else { Ok(cards) }
    }

    /// Put a card in a project (a card belongs to one project at most).
    pub fn add_card_to_project(&self, project: &str, card: &str) -> Res<Option<Project>> {
        self.db.execute("UPDATE card SET cad_prjid=? WHERE cad_id=?", params![project, card]).map_err(err)?;
        self.project(project)
    }

    pub fn remove_card_from_project(&self, project: &str, card: &str) -> Res<Option<Project>> {
        self.db.execute("UPDATE card SET cad_prjid=NULL WHERE cad_id=? AND cad_prjid=?", params![card, project]).map_err(err)?;
        self.project(project)
    }

    // ─── Environments ────────────────────────────────────────────────────────────────────────────

    pub fn create_environment(&self, name: &str) -> Res<Environment> {
        if name.is_empty() {
            return Err("Missing name".to_owned());
        }
        let id = new_id();
        let max: i64 = self.db.query_row("SELECT COALESCE(MAX(env_order),0) FROM environment WHERE env_global=0", [], |r| r.get(0)).map_err(err)?;
        self.db.execute("INSERT INTO environment (env_id,env_name,env_order,env_date,env_global) VALUES (?,?,?,?,0)", params![id, name, max + 1, now_ms()]).map_err(err)?;
        Ok(Environment { id, name: name.to_owned(), order: max + 1, is_global: false })
    }

    pub fn rename_environment(&self, id: &str, name: &str) -> Res<()> {
        if name.is_empty() {
            return Err("Missing name".to_owned());
        }
        if id == self.global_env {
            return Err("Cannot rename Global environment".to_owned());
        }
        match self.db.execute("UPDATE environment SET env_name=? WHERE env_id=?", params![name, id]).map_err(err)? {
            0 => Err("Not found".to_owned()),
            _ => Ok(()),
        }
    }

    /// Delete an environment. Refused for Global and for one that still holds a card or a project.
    pub fn delete_environment(&self, id: &str) -> Res<()> {
        if id == self.global_env {
            return Err("Cannot delete Global environment".to_owned());
        }
        let cards: i64 = self.db.query_row("SELECT COUNT(*) FROM card WHERE cad_envid=?", [id], |r| r.get(0)).map_err(err)?;
        let projects: i64 = self.db.query_row("SELECT COUNT(*) FROM project WHERE prj_env=?", [id], |r| r.get(0)).map_err(err)?;
        if cards > 0 || projects > 0 {
            return Err("Environment is not empty".to_owned());
        }
        match self.db.execute("DELETE FROM environment WHERE env_id=?", [id]).map_err(err)? {
            0 => Err("Not found".to_owned()),
            _ => Ok(()),
        }
    }

    // ─── Moving between environments ─────────────────────────────────────────────────────────────

    /// A music / vocal card leaves its environment: its style is first written into the songs of that
    /// environment that used it (so they lose nothing), then those links are cut.
    fn detach_and_move_style_card(&self, card: &str, target_env: &str, target_project: Option<&str>) -> Res<bool> {
        let Some((style, env)) = self.db.query_row("SELECT cad_style, cad_envid FROM card WHERE cad_id=?", [card], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))).optional().map_err(err)? else {
            return Ok(false);
        };
        let songs: Vec<(String, String)> = {
            let mut st = self.db.prepare("SELECT cad_id, cad_style FROM card WHERE cad_id IN (SELECT cal_cadid_parent FROM card_link WHERE cal_cadid_child=?) AND cad_envid=?").map_err(err)?;
            let rows = st.query_map(params![card, env], |r| Ok((r.get(0)?, r.get(1)?))).map_err(err)?;
            rows.collect::<Result<_, _>>().map_err(err)?
        };
        for (song, song_style) in songs {
            let merged = [song_style.trim(), style.trim()].iter().filter(|s| !s.is_empty()).copied().collect::<Vec<_>>().join("\n");
            if merged != song_style {
                self.db.execute("UPDATE card SET cad_style=? WHERE cad_id=?", params![merged, song]).map_err(err)?;
            }
        }
        self.db.execute("DELETE FROM card_link WHERE cal_cadid_child=? AND cal_cadid_parent IN (SELECT cad_id FROM card WHERE cad_envid=?)", params![card, env]).map_err(err)?;
        self.db.execute("UPDATE card SET cad_prjid=NULL WHERE cad_id=? AND cad_prjid IN (SELECT prj_id FROM project WHERE prj_env=?)", params![card, env]).map_err(err)?;
        self.db.execute("UPDATE card SET cad_envid=? WHERE cad_id=?", params![target_env, card]).map_err(err)?;
        if let Some(project) = target_project {
            self.db.execute("UPDATE card SET cad_prjid=? WHERE cad_id=?", params![project, card]).map_err(err)?;
        }
        Ok(true)
    }

    /// A copy of a card (all of it, its translations too) in another environment. Returns the copy's id.
    // ⚠ the WHOLE card is copied, not a chosen subset: a field left out here is data the user loses
    // without a sound the moment a shared style card is duplicated into another environment.
    fn duplicate_card_into(&self, card: &str, target_env: &str) -> Res<String> {
        let (name, kind): (String, String) = self.db.query_row("SELECT cad_name, cad_type FROM card WHERE cad_id=?", [card], |r| Ok((r.get(0)?, r.get(1)?))).map_err(err)?;
        let new = new_id();
        let version = self.next_card_version(&name, &kind, None)?;
        self.db
            .execute(
                "INSERT INTO card(cad_id,cad_name,cad_version,cad_type,cad_desc,cad_note,cad_style,cad_ref_artist,cad_ref_complement,cad_lyrics,cad_tags,cad_favorite,cad_date,cad_order,cad_media_path,cad_ai_tweaks,cad_lang,cad_envid) \
                 SELECT ?,cad_name,?,cad_type,cad_desc,cad_note,cad_style,cad_ref_artist,cad_ref_complement,cad_lyrics,cad_tags,cad_favorite,?,cad_order,cad_media_path,cad_ai_tweaks,cad_lang,? FROM card WHERE cad_id=?",
                params![new, version, now_ms(), target_env, card],
            )
            .map_err(err)?;
        let translations: Vec<(String, String, String, i64)> = {
            let mut st = self.db.prepare("SELECT cat_name,cat_lang,cat_lyrics,cat_date FROM card_translation WHERE cat_cadid=?").map_err(err)?;
            let rows = st.query_map([card], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))).map_err(err)?;
            rows.collect::<Result<_, _>>().map_err(err)?
        };
        for (tr_name, lang, lyrics, date) in translations {
            self.db.execute("INSERT INTO card_translation (cat_id,cat_cadid,cat_name,cat_lang,cat_lyrics,cat_date) VALUES (?,?,?,?,?,?)", params![new_id(), new, tr_name, lang, lyrics, date]).map_err(err)?;
        }
        Ok(new)
    }

    fn children_of(&self, song: &str) -> Res<Vec<String>> {
        let mut st = self.db.prepare("SELECT cal_cadid_child FROM card_link WHERE cal_cadid_parent=?").map_err(err)?;
        let rows = st.query_map([song], |r| r.get(0)).map_err(err)?;
        rows.collect::<Result<_, _>>().map_err(err)
    }

    /// Move cards to another environment (and, optionally, into a project there).
    ///
    /// A song takes the music / vocal cards it uses with it: one used ONLY by songs that are moving moves;
    /// one that a song staying behind still uses is duplicated, and the moving song points at the copy.
    /// A card of the Global environment is never moved (every environment sees it).
    pub fn move_cards(&self, ids: &[String], target_env: &str, target_project: Option<&str>) -> Res<MovedCards> {
        if target_env.is_empty() {
            return Err("Missing targetEnvId".to_owned());
        }
        if self.db.query_row("SELECT 1 FROM environment WHERE env_id=?", [target_env], |_| Ok(())).optional().map_err(err)?.is_none() {
            return Err("Target environment not found".to_owned());
        }
        let mut out = MovedCards::default();
        let tx = self.db.unchecked_transaction().map_err(err)?;
        let kind_env = |id: &str| -> Res<Option<(String, String)>> { self.db.query_row("SELECT cad_type, cad_envid FROM card WHERE cad_id=?", [id], |r| Ok((r.get(0)?, r.get(1)?))).optional().map_err(err) };

        let mut moved_children: HashSet<String> = HashSet::new();
        let mut others: Vec<&String> = Vec::new();
        for id in ids {
            let Some((kind, source_env)) = kind_env(id)? else { continue };
            if !is_song(&kind) {
                others.push(id);
                continue;
            }
            for child in self.children_of(id)? {
                if moved_children.contains(&child) {
                    continue;
                }
                let Some((_, child_env)) = kind_env(&child)? else { continue };
                if child_env == self.global_env || child_env == target_env {
                    continue;
                }
                // the songs of the source environment that use this child and are NOT moving
                let users: Vec<String> = {
                    let mut st = self.db.prepare("SELECT pl.cal_cadid_parent FROM card_link pl JOIN card p ON p.cad_id=pl.cal_cadid_parent WHERE pl.cal_cadid_child=? AND p.cad_envid=?").map_err(err)?;
                    let rows = st.query_map(params![child, source_env], |r| r.get(0)).map_err(err)?;
                    rows.collect::<Result<_, _>>().map_err(err)?
                };
                if users.iter().all(|u| ids.contains(u)) {
                    self.db.execute("UPDATE card SET cad_prjid=NULL WHERE cad_id=?", [&child]).map_err(err)?;
                    self.db.execute("UPDATE card SET cad_envid=? WHERE cad_id=?", params![target_env, child]).map_err(err)?;
                    moved_children.insert(child);
                    out.linked_moved_count += 1;
                } else {
                    let copy = self.duplicate_card_into(&child, target_env)?;
                    self.db.execute("UPDATE card_link SET cal_cadid_child=? WHERE cal_cadid_parent=? AND cal_cadid_child=?", params![copy, id, child]).map_err(err)?;
                    out.duplicated_ids.push(copy);
                }
            }
            self.db.execute("UPDATE card SET cad_prjid=NULL WHERE cad_id=? AND cad_prjid IN (SELECT prj_id FROM project WHERE prj_env=?)", params![id, source_env]).map_err(err)?;
            self.db.execute("UPDATE card SET cad_envid=? WHERE cad_id=?", params![target_env, id]).map_err(err)?;
            if let Some(project) = target_project {
                self.db.execute("UPDATE card SET cad_prjid=? WHERE cad_id=?", params![project, id]).map_err(err)?;
            }
            out.moved_ids.push(id.clone());
        }
        // the music / vocal cards of the selection that did not already move with a song
        for id in others {
            if !moved_children.contains(id) && self.detach_and_move_style_card(id, target_env, target_project)? {
                out.moved_ids.push(id.clone());
            }
        }
        tx.commit().map_err(err)?;
        Ok(out)
    }

    /// Move a project, its sub-projects and their cards to another environment. A project whose name
    /// already exists there is MERGED into that one.
    pub fn move_folder(&self, project: &str, target_env: &str) -> Res<MovedFolder> {
        if project.is_empty() || target_env.is_empty() {
            return Err("Missing fields".to_owned());
        }
        let mut out = MovedFolder::default();
        let tx = self.db.unchecked_transaction().map_err(err)?;
        // the project and everything below it, the root first
        let mut subtree: Vec<String> = Vec::new();
        let mut queue = vec![project.to_owned()];
        while !queue.is_empty() {
            let id = queue.remove(0);
            let mut st = self.db.prepare("SELECT prj_id FROM project WHERE prj_parent=?").map_err(err)?;
            let children: Vec<String> = st.query_map([&id], |r| r.get(0)).map_err(err)?.collect::<Result<_, _>>().map_err(err)?;
            queue.extend(children);
            subtree.push(id);
        }
        let source_env: String = self.db.query_row("SELECT prj_env FROM project WHERE prj_id=?", [project], |r| r.get(0)).optional().map_err(err)?.unwrap_or_default();
        let mut cards: Vec<String> = Vec::new();
        for id in &subtree {
            let mut st = self.db.prepare("SELECT cad_id FROM card WHERE cad_prjid=?").map_err(err)?;
            for card in st.query_map([id], |r| r.get::<_, String>(0)).map_err(err)? {
                let card = card.map_err(err)?;
                if !cards.contains(&card) {
                    cards.push(card);
                }
            }
        }
        for id in &cards {
            let Some(kind) = self.db.query_row("SELECT cad_type FROM card WHERE cad_id=?", [id], |r| r.get::<_, String>(0)).optional().map_err(err)? else { continue };
            if !is_song(&kind) {
                continue;
            }
            for child in self.children_of(id)? {
                let child_env: Option<String> = self.db.query_row("SELECT cad_envid FROM card WHERE cad_id=?", [&child], |r| r.get(0)).optional().map_err(err)?;
                if child_env.is_none_or(|e| e == self.global_env) {
                    continue;
                }
                let users: i64 = self
                    .db
                    .query_row("SELECT COUNT(*) FROM card_link pl JOIN card p ON p.cad_id=pl.cal_cadid_parent WHERE pl.cal_cadid_child=? AND p.cad_envid=?", params![child, source_env], |r| r.get(0))
                    .map_err(err)?;
                if users == 1 {
                    self.db.execute("UPDATE card SET cad_envid=? WHERE cad_id=?", params![target_env, child]).map_err(err)?;
                    out.moved_card_ids.push(child);
                } else {
                    let copy = self.duplicate_card_into(&child, target_env)?;
                    self.db.execute("UPDATE card_link SET cal_cadid_child=? WHERE cal_cadid_parent=? AND cal_cadid_child=?", params![copy, id, child]).map_err(err)?;
                    out.duplicated_card_ids.push(copy);
                }
            }
            self.db.execute("UPDATE card SET cad_envid=? WHERE cad_id=?", params![target_env, id]).map_err(err)?;
            out.moved_card_ids.push(id.clone());
        }
        // the projects, the root first, their parents re-pointed where a merge replaced one
        let mut mapped: HashMap<String, String> = HashMap::new();
        for id in &subtree {
            let (name, parent): (String, Option<String>) = self.db.query_row("SELECT prj_name, prj_parent FROM project WHERE prj_id=?", [id], |r| Ok((r.get(0)?, r.get(1)?))).map_err(err)?;
            let parent = parent.filter(|p| !p.is_empty()).map(|p| mapped.get(&p).cloned().unwrap_or(p));
            let clash: Option<String> = self.db.query_row("SELECT prj_id FROM project WHERE prj_name=? AND prj_env=? AND prj_id!=?", params![name, target_env, id], |r| r.get(0)).optional().map_err(err)?;
            match clash {
                Some(existing) => {
                    self.db.execute("UPDATE card SET cad_prjid=? WHERE cad_prjid=?", params![existing, id]).map_err(err)?;
                    self.db.execute("DELETE FROM project WHERE prj_id=?", [id]).map_err(err)?;
                    mapped.insert(id.clone(), existing);
                }
                None => {
                    self.db.execute("UPDATE project SET prj_env=?, prj_parent=? WHERE prj_id=?", params![target_env, parent, id]).map_err(err)?;
                    mapped.insert(id.clone(), id.clone());
                    out.moved_project_ids.push(id.clone());
                }
            }
        }
        tx.commit().map_err(err)?;
        Ok(out)
    }
}

/// Every file below `dir`, by lower-case name (the first one found wins).
fn index_files(dir: &Path, into: &mut HashMap<String, std::path::PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            index_files(&path, into);
        } else {
            into.entry(entry.file_name().to_string_lossy().to_lowercase()).or_insert(path);
        }
    }
}
