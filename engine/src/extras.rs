//! Translations of a card, and the switch between absolute and relative stored paths.

use crate::store::{MediaCfg, Res, Store, err, new_id, now_ms, relative_to_root};
use rusqlite::{OptionalExtension, params};
use serde::Serialize;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Translation {
    pub id: String,
    pub name: String,
    pub lang: String,
    pub content: String,
    pub date: i64,
}

impl Store {
    /// For the search: every card's translated lyrics, joined, by card id.
    pub fn translation_index(&self) -> Res<HashMap<String, String>> {
        let mut st = self.db.prepare("SELECT cat_cadid, cat_lyrics FROM card_translation").map_err(err)?;
        let mut index: HashMap<String, String> = HashMap::new();
        for row in st.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))).map_err(err)? {
            let (card, lyrics) = row.map_err(err)?;
            let entry = index.entry(card).or_default();
            if !entry.is_empty() {
                entry.push('\n');
            }
            entry.push_str(&lyrics);
        }
        Ok(index)
    }

    pub fn translations(&self, card: &str) -> Res<Vec<Translation>> {
        let mut st = self.db.prepare("SELECT cat_id,cat_name,cat_lang,cat_lyrics,cat_date FROM card_translation WHERE cat_cadid=? ORDER BY cat_date ASC").map_err(err)?;
        let rows = st.query_map([card], |r| Ok(Translation { id: r.get(0)?, name: r.get(1)?, lang: r.get(2)?, content: r.get(3)?, date: r.get(4)? })).map_err(err)?;
        rows.collect::<Result<_, _>>().map_err(err)
    }

    /// Add a translation. A card has one per language at most.
    pub fn create_translation(&self, card: &str, lang: &str, name: &str, content: &str) -> Res<Translation> {
        if card.is_empty() || lang.is_empty() {
            return Err("Missing fields".to_owned());
        }
        if self.db.query_row("SELECT 1 FROM card_translation WHERE cat_cadid=? AND cat_lang=?", params![card, lang], |_| Ok(())).optional().map_err(err)?.is_some() {
            return Err("Translation for this language already exists".to_owned());
        }
        let tr = Translation { id: new_id(), name: name.to_owned(), lang: lang.to_owned(), content: content.to_owned(), date: now_ms() };
        self.db.execute("INSERT INTO card_translation (cat_id,cat_cadid,cat_name,cat_lang,cat_lyrics,cat_date) VALUES (?,?,?,?,?,?)", params![tr.id, card, tr.name, tr.lang, tr.content, tr.date]).map_err(err)?;
        Ok(tr)
    }

    pub fn update_translation(&self, id: &str, name: Option<&str>, content: Option<&str>) -> Res<()> {
        match self.db.execute("UPDATE card_translation SET cat_name=COALESCE(?,cat_name),cat_lyrics=COALESCE(?,cat_lyrics),cat_date=? WHERE cat_id=?", params![name, content, now_ms(), id]).map_err(err)? {
            0 => Err("Not found".to_owned()),
            _ => Ok(()),
        }
    }

    pub fn delete_translation(&self, id: &str) -> Res<()> {
        match self.db.execute("DELETE FROM card_translation WHERE cat_id=?", [id]).map_err(err)? {
            0 => Err("Not found".to_owned()),
            _ => Ok(()),
        }
    }

    /// The languages the songs have translations in: those of the given projects, or all when none is given.
    pub fn translation_languages(&self, projects: &[String]) -> Res<Vec<String>> {
        let mut st = self
            .db
            .prepare("SELECT DISTINCT pt.cat_lang, p.cad_prjid FROM card_translation pt JOIN card p ON p.cad_id = pt.cat_cadid WHERE p.cad_type = 'song' ORDER BY pt.cat_lang")
            .map_err(err)?;
        let mut out: Vec<String> = Vec::new();
        for row in st.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?))).map_err(err)? {
            let (lang, project) = row.map_err(err)?;
            if (projects.is_empty() || project.is_some_and(|p| projects.contains(&p))) && !out.contains(&lang) {
                out.push(lang);
            }
        }
        Ok(out)
    }
}

// ─── Stored paths: the audit, then the switch ───────────────────────────────────────────────────
// ⚠ the order is the contract. `audit` COUNTS what a switch would do and writes nothing; the
// window shows it; only then `apply` runs. The mode is never written anywhere else: a mode that disagrees
// with what is stored is exactly the mixed state this design exists to prevent.

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct NamedPath {
    pub name: String,
    pub path: String,
}

/// What a switch would do to the cards' files, or to the projects' folders.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PathCounts {
    pub total: usize,
    pub rewrite: usize,
    /// Paths not under the new root: they BLOCK a switch to relative.
    pub outside: usize,
    pub missing: usize,
    pub relative: usize,
    pub found_old: usize,
    pub found_new: usize,
    /// The files behind `missing` / not found under the new root (300 at most: a screenful to read).
    pub not_found: Vec<NamedPath>,
    pub outside_list: Vec<NamedPath>,
    #[serde(skip)]
    writes: Vec<(String, String)>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MediaPlan {
    /// `toRelative`, `toAbsolute`, `changeRoot` or `none`.
    pub kind: String,
    pub mode: String,
    pub root: String,
    pub cards: PathCounts,
    pub projects: PathCounts,
}

/// Why a requested mode / root is refused before anything is counted.
#[derive(Clone, Debug, PartialEq)]
pub enum MediaRefused {
    /// Relative needs a root, and an absolute one.
    NotAbsolute,
    /// Some stored paths are not under the root (how many).
    OutsideRoot(usize),
}

impl Store {
    /// Check a requested mode and root, and tidy the root.
    fn media_target(&self, relative: bool, root: &str) -> Result<MediaCfg, MediaRefused> {
        let mut root = root.trim().to_owned();
        if relative {
            if root.is_empty() || !Path::new(&root).is_absolute() {
                return Err(MediaRefused::NotAbsolute);
            }
            root = Path::new(&root).components().collect::<PathBuf>().to_string_lossy().into_owned();
            if root.len() > 3 {
                root = root.trim_end_matches(['\\', '/']).to_owned();
            }
        } else if root.is_empty() {
            // absolute keeps the remembered root, so switching back is one click
            root = self.media_cfg().root;
        }
        Ok(MediaCfg { relative, root })
    }

    fn plan_media(&self, target: &MediaCfg) -> Res<MediaPlan> {
        let cur = self.media_cfg();
        let kind = match (cur.relative, target.relative) {
            (false, true) => "toRelative",
            (true, false) => "toAbsolute",
            (true, true) if target.root != cur.root => "changeRoot",
            _ => "none",
        };
        let count = |sql: &str| -> Res<PathCounts> {
            let rows: Vec<(String, String, String)> = {
                let mut st = self.db.prepare(sql).map_err(err)?;
                let rows = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).map_err(err)?;
                rows.collect::<Result<_, _>>().map_err(err)?
            };
            let mut c = PathCounts { total: rows.len(), ..Default::default() };
            for (id, name, p) in rows {
                let is_abs = Path::new(&p).is_absolute();
                if !is_abs {
                    c.relative += 1;
                }
                let lose = |c: &mut PathCounts, path: String| {
                    if c.not_found.len() < 300 {
                        c.not_found.push(NamedPath { name: name.clone(), path });
                    }
                };
                if kind == "toRelative" && is_abs {
                    match relative_to_root(&p, &target.root) {
                        Some(rel) => {
                            c.rewrite += 1;
                            c.writes.push((rel, id.clone()));
                        }
                        None => {
                            c.outside += 1;
                            if c.outside_list.len() < 300 {
                                c.outside_list.push(NamedPath { name: name.clone(), path: p.clone() });
                            }
                        }
                    }
                }
                if kind == "toAbsolute" && !is_abs {
                    c.rewrite += 1;
                    c.writes.push((Path::new(&cur.root).join(&p).to_string_lossy().into_owned(), id.clone()));
                }
                if kind != "changeRoot" {
                    let now = self.from_stored(&p);
                    if !Path::new(&now).exists() {
                        c.missing += 1;
                        lose(&mut c, now);
                    }
                }
                if kind == "changeRoot" && !is_abs {
                    if Path::new(&cur.root).join(&p).exists() {
                        c.found_old += 1;
                    }
                    let under_new = Path::new(&target.root).join(&p);
                    if under_new.exists() {
                        c.found_new += 1;
                    } else {
                        lose(&mut c, under_new.to_string_lossy().into_owned());
                    }
                }
            }
            Ok(c)
        };
        Ok(MediaPlan {
            kind: kind.to_owned(),
            mode: if target.relative { "relative" } else { "absolute" }.to_owned(),
            root: target.root.clone(),
            cards: count("SELECT cad_id, cad_name, cad_media_path FROM card WHERE cad_media_path != ''")?,
            projects: count("SELECT prj_id, prj_name, prj_path FROM project WHERE prj_path != ''")?,
        })
    }

    /// What switching to this mode and root WOULD do. Nothing is written.
    pub fn audit_media_paths(&self, relative: bool, root: &str) -> Res<Result<MediaPlan, MediaRefused>> {
        match self.media_target(relative, root) {
            Ok(target) => self.plan_media(&target).map(Ok),
            Err(refused) => Ok(Err(refused)),
        }
    }

    /// Switch: rewrite every stored path and set the mode, in one step. Returns how many paths were rewritten.
    // ⚠ relative means EVERY stored path is relative to the root. A path outside the root cannot be,
    // so the switch is REFUSED here, not merely disabled in the dialog: applying it would leave the mixed
    // state this setting exists to prevent.
    pub fn apply_media_paths(&self, relative: bool, root: &str) -> Res<Result<usize, MediaRefused>> {
        let target = match self.media_target(relative, root) {
            Ok(t) => t,
            Err(refused) => return Ok(Err(refused)),
        };
        let plan = self.plan_media(&target)?;
        let outside = if plan.kind == "toRelative" { plan.cards.outside + plan.projects.outside } else { 0 };
        if outside > 0 {
            return Ok(Err(MediaRefused::OutsideRoot(outside)));
        }
        let tx = self.db.unchecked_transaction().map_err(err)?;
        for (value, id) in &plan.cards.writes {
            tx.execute("UPDATE card SET cad_media_path=? WHERE cad_id=?", params![value, id]).map_err(err)?;
        }
        for (value, id) in &plan.projects.writes {
            tx.execute("UPDATE project SET prj_path=? WHERE prj_id=?", params![value, id]).map_err(err)?;
        }
        tx.commit().map_err(err)?;
        self.set_media_cfg(target);
        Ok(Ok(plan.cards.rewrite + plan.projects.rewrite))
    }
}
