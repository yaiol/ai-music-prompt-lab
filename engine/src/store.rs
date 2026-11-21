//! The database and the settings file: opening them, the schema, how a stored path is written and read,
//! and the two row shapes everything else uses (`Card`, `Project`).
//!
//! ⚠ there is NO migration code here, on purpose (the app's own rule): the schema below is
//! created when missing and is otherwise taken as it is.

use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    cell::RefCell,
    fs,
    path::{Path, PathBuf},
};

/// The Global environment's id: the same on every machine, so a backup made on one opens on another.
pub const GLOBAL_ENV_ID: &str = "00000000-0000-4000-8000-000000000000";
/// The prefix of every settings key.
pub const PREFIX: &str = "ampl";

pub type Res<T> = Result<T, String>;

pub(crate) fn err<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

pub fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Url {
    pub id: String,
    pub label: String,
    pub href: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    pub id: String,
    /// `music`, `vocal`, `song` or `note`.
    #[serde(rename = "type")]
    pub kind: String,
    pub name: String,
    /// Shown on the card, searched, exported to a document.
    pub desc: String,
    /// Private: never searched, never exported to a document.
    pub note: String,
    pub style: String,
    /// What the style is aiming at: the act it evokes. Never merged into `style`, never sent to an AI.
    pub reference_artist: String,
    /// Whatever narrows the artist: an LP, a song, an era, a genre.
    pub reference_complement: String,
    pub lyrics: String,
    pub tags: Vec<String>,
    pub favorite: bool,
    pub date: i64,
    pub version: i64,
    pub linked_cards: Vec<String>,
    pub urls: Vec<Url>,
    pub sort_number: Option<i64>,
    /// The absolute path, whatever way it is stored.
    pub media_path: String,
    pub ai_tweaks: Value,
    pub lang: String,
    pub env: String,
    pub project: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub color: String,
    pub date: i64,
    pub version: i64,
    pub parent_id: Option<String>,
    pub card_ids: Vec<String>,
    pub urls: Vec<Url>,
    pub env: String,
    /// The media folder, absolute, whatever way it is stored.
    pub path: String,
    pub finalized: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Environment {
    pub id: String,
    pub name: String,
    pub order: i64,
    pub is_global: bool,
}

/// How every stored path is written: in full, or as the part below the media root.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct MediaCfg {
    pub relative: bool,
    /// Remembered even in absolute mode, so switching back is one click.
    pub root: String,
}

pub struct Store {
    pub(crate) db: Connection,
    settings_path: PathBuf,
    /// The settings file as last read or written: a card list asks for a setting dozens of times per paint, and the file is ours alone.
    settings: RefCell<Option<serde_json::Map<String, Value>>>,
    media: RefCell<Option<MediaCfg>>,
    pub default_env: String,
    pub global_env: String,
}

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS card (
  cad_id TEXT PRIMARY KEY, cad_envid TEXT NOT NULL DEFAULT '',
  cad_prjid TEXT REFERENCES project(prj_id) ON DELETE SET NULL,
  cad_name TEXT NOT NULL, cad_version INTEGER NOT NULL DEFAULT 1, cad_type TEXT NOT NULL, cad_order INTEGER,
  cad_desc TEXT NOT NULL DEFAULT '', cad_style TEXT NOT NULL DEFAULT '',
  cad_ref_artist TEXT NOT NULL DEFAULT '', cad_ref_complement TEXT NOT NULL DEFAULT '',
  cad_lang TEXT NOT NULL DEFAULT '', cad_lyrics TEXT NOT NULL DEFAULT '', cad_ai_tweaks TEXT NOT NULL DEFAULT '{}',
  cad_media_path TEXT NOT NULL DEFAULT '', cad_note TEXT NOT NULL DEFAULT '', cad_tags TEXT NOT NULL DEFAULT '[]',
  cad_favorite INTEGER NOT NULL DEFAULT 0, cad_date INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS card_link (
  cal_cadid_parent TEXT NOT NULL, cal_cadid_child TEXT NOT NULL,
  PRIMARY KEY (cal_cadid_parent, cal_cadid_child),
  FOREIGN KEY (cal_cadid_parent) REFERENCES card(cad_id) ON DELETE CASCADE,
  FOREIGN KEY (cal_cadid_child) REFERENCES card(cad_id) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS project (
  prj_id TEXT PRIMARY KEY, prj_name TEXT NOT NULL, prj_version INTEGER NOT NULL DEFAULT 1,
  prj_color TEXT NOT NULL DEFAULT '#7c6fff', prj_date INTEGER NOT NULL, prj_parent TEXT DEFAULT NULL,
  prj_path TEXT NOT NULL DEFAULT '', prj_env TEXT NOT NULL DEFAULT '', prj_finalized INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS url (
  url_id TEXT PRIMARY KEY, url_parent_id TEXT NOT NULL, url_type TEXT NOT NULL DEFAULT 'cad',
  url_label TEXT NOT NULL DEFAULT '', url_href TEXT NOT NULL, url_order INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS card_translation (
  cat_id TEXT PRIMARY KEY, cat_cadid TEXT NOT NULL REFERENCES card(cad_id) ON DELETE CASCADE,
  cat_lang TEXT NOT NULL, cat_name TEXT NOT NULL DEFAULT '', cat_lyrics TEXT NOT NULL DEFAULT '',
  cat_date INTEGER NOT NULL, UNIQUE(cat_cadid, cat_lang)
);
CREATE TABLE IF NOT EXISTS environment (
  env_id TEXT PRIMARY KEY, env_name TEXT NOT NULL, env_order INTEGER NOT NULL DEFAULT 0,
  env_date INTEGER NOT NULL, env_global INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS ai_block_preset (
  abp_id TEXT PRIMARY KEY, abp_task TEXT NOT NULL, abp_block TEXT NOT NULL, abp_name TEXT NOT NULL,
  abp_content TEXT NOT NULL, abp_default INTEGER NOT NULL DEFAULT 0, abp_order INTEGER NOT NULL DEFAULT 0,
  abp_default_pick INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS ai_template (
  atp_id TEXT PRIMARY KEY, atp_task TEXT NOT NULL, atp_name TEXT NOT NULL,
  atp_default INTEGER NOT NULL DEFAULT 0, atp_order INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS ai_template_block (
  atb_id TEXT PRIMARY KEY, atb_atpid TEXT NOT NULL, atb_block TEXT NOT NULL, atb_abpid TEXT,
  atb_enabled INTEGER NOT NULL DEFAULT 1, atb_name TEXT NOT NULL DEFAULT '', atb_prefix TEXT NOT NULL DEFAULT '',
  FOREIGN KEY (atb_atpid) REFERENCES ai_template(atp_id) ON DELETE CASCADE,
  FOREIGN KEY (atb_abpid) REFERENCES ai_block_preset(abp_id) ON DELETE SET NULL
);
CREATE TABLE IF NOT EXISTS ai_wizard (awz_id TEXT PRIMARY KEY, awz_name TEXT NOT NULL, awz_order INTEGER NOT NULL DEFAULT 0);
CREATE TABLE IF NOT EXISTS ai_wizard_task (
  awt_awzid TEXT NOT NULL, awt_task TEXT NOT NULL, awt_category TEXT NOT NULL, awt_order INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (awt_awzid, awt_task),
  FOREIGN KEY (awt_awzid) REFERENCES ai_wizard(awz_id) ON DELETE CASCADE
);
";

impl Store {
    /// Open (or create) the database and the settings file beside it.
    pub fn open(db_path: &Path, settings_path: &Path) -> Res<Store> {
        if let Some(dir) = db_path.parent() {
            let _ = fs::create_dir_all(dir);
        }
        let db = Connection::open(db_path).map_err(err)?;
        db.pragma_update(None, "foreign_keys", "ON").map_err(err)?;
        db.execute_batch(SCHEMA).map_err(err)?;

        // The two environments every database has: Global, and one ordinary one.
        let global_env: String = match db.query_row("SELECT env_id FROM environment WHERE env_global=1", [], |r| r.get(0)).optional().map_err(err)? {
            Some(id) => id,
            None => {
                db.execute("INSERT INTO environment (env_id,env_name,env_order,env_date,env_global) VALUES (?,?,?,?,1)", params![GLOBAL_ENV_ID, "(Global)", 0, now_ms()]).map_err(err)?;
                GLOBAL_ENV_ID.to_owned()
            }
        };
        let default_env: String = match db.query_row("SELECT env_id FROM environment WHERE env_global=0 ORDER BY env_order ASC", [], |r| r.get(0)).optional().map_err(err)? {
            Some(id) => id,
            None => {
                let id = new_id();
                db.execute("INSERT INTO environment (env_id,env_name,env_order,env_date,env_global) VALUES (?,?,?,?,0)", params![id, "Default", 1, now_ms()]).map_err(err)?;
                id
            }
        };
        db.execute("UPDATE card SET cad_envid=? WHERE cad_envid=''", [&default_env]).map_err(err)?;
        db.execute("UPDATE project SET prj_env=? WHERE prj_env=''", [&default_env]).map_err(err)?;

        Ok(Store { db, settings_path: settings_path.to_owned(), settings: RefCell::new(None), media: RefCell::new(None), default_env, global_env })
    }

    // ─── Settings: one flat file of `ampl-…` keys ────────────────────────────────────────────────

    pub fn read_settings(&self) -> serde_json::Map<String, Value> {
        if let Some(cached) = self.settings.borrow().as_ref() {
            return cached.clone();
        }
        let read: serde_json::Map<String, Value> = fs::read_to_string(&self.settings_path).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
        *self.settings.borrow_mut() = Some(read.clone());
        read
    }

    pub fn write_settings(&self, data: &serde_json::Map<String, Value>) {
        *self.settings.borrow_mut() = Some(data.clone());
        if let Ok(text) = serde_json::to_string_pretty(data) {
            let _ = fs::write(&self.settings_path, text);
        }
    }

    /// A setting as text ("" when absent). `key` is given without the prefix.
    pub fn setting(&self, key: &str) -> String {
        if self.settings.borrow().is_none() {
            self.read_settings();
        }
        match self.settings.borrow().as_ref().and_then(|m| m.get(&format!("{PREFIX}-{key}"))) {
            Some(Value::String(s)) => s.clone(),
            Some(Value::Null) | None => String::new(),
            Some(other) => other.to_string(),
        }
    }

    /// ⚠ The media mode and root are NOT set through here: see `media_paths::apply`.
    pub fn set_setting(&self, key: &str, value: &str) {
        let mut all = self.read_settings();
        all.insert(format!("{PREFIX}-{key}"), Value::String(value.to_owned()));
        self.write_settings(&all);
    }

    // ─── Stored paths: Absolute | Relative ───────────────────────────────────────────────────────
    // ONE setting decides how EVERY stored path is written: each card's media file and each project's
    // media folder. ⚠ the conversion lives at this boundary ONLY. Rows are resolved on the way
    // out (`card_from_row`, `project_from_row`) and every write of those two columns goes through
    // `to_stored` on the way in, so the rest of the app only ever sees absolute paths.

    pub fn media_cfg(&self) -> MediaCfg {
        if let Some(cfg) = self.media.borrow().clone() {
            return cfg;
        }
        let cfg = MediaCfg { relative: self.setting("media-mode") == "relative", root: self.setting("media-root").trim().to_owned() };
        *self.media.borrow_mut() = Some(cfg.clone());
        cfg
    }

    pub(crate) fn set_media_cfg(&self, cfg: MediaCfg) {
        let mut all = self.read_settings();
        all.insert(format!("{PREFIX}-media-mode"), Value::String(if cfg.relative { "relative" } else { "absolute" }.to_owned()));
        if cfg.root.is_empty() {
            all.remove(&format!("{PREFIX}-media-root"));
        } else {
            all.insert(format!("{PREFIX}-media-root"), Value::String(cfg.root.clone()));
        }
        self.write_settings(&all);
        *self.media.borrow_mut() = Some(cfg);
    }

    /// The root IN FORCE: empty in absolute mode, even when one is remembered for next time.
    pub(crate) fn media_root(&self) -> String {
        let cfg = self.media_cfg();
        if cfg.relative { cfg.root } else { String::new() }
    }

    pub(crate) fn to_stored(&self, path: &str) -> String {
        if path.is_empty() || !Path::new(path).is_absolute() {
            return path.to_owned();
        }
        relative_to_root(path, &self.media_root()).unwrap_or_else(|| path.to_owned())
    }

    pub(crate) fn from_stored(&self, stored: &str) -> String {
        if stored.is_empty() || Path::new(stored).is_absolute() {
            return stored.to_owned();
        }
        let root = self.media_root();
        if root.is_empty() { stored.to_owned() } else { Path::new(&root).join(stored).to_string_lossy().into_owned() }
    }

    // ─── Rows ────────────────────────────────────────────────────────────────────────────────────

    pub(crate) fn urls_of(&self, parent: &str, kind: &str) -> Res<Vec<Url>> {
        let mut st = self.db.prepare_cached("SELECT url_id,url_label,url_href FROM url WHERE url_parent_id=? AND url_type=? ORDER BY url_order").map_err(err)?;
        let rows = st.query_map(params![parent, kind], |r| Ok(Url { id: r.get(0)?, label: r.get(1)?, href: r.get(2)? })).map_err(err)?;
        rows.collect::<Result<_, _>>().map_err(err)
    }

    pub(crate) fn card_from_row(&self, row: &Row) -> rusqlite::Result<Card> {
        let id: String = row.get("cad_id")?;
        let text = |name: &str| -> rusqlite::Result<String> { Ok(row.get::<_, Option<String>>(name)?.unwrap_or_default()) };
        let mut links = self.db.prepare_cached("SELECT cal_cadid_child FROM card_link WHERE cal_cadid_parent=?")?;
        let linked_cards = links.query_map([&id], |r| r.get(0))?.collect::<Result<Vec<String>, _>>()?;
        let tags = text("cad_tags")?;
        let tweaks = text("cad_ai_tweaks")?;
        Ok(Card {
            kind: row.get("cad_type")?,
            name: row.get("cad_name")?,
            desc: text("cad_desc")?,
            note: text("cad_note")?,
            style: text("cad_style")?,
            reference_artist: text("cad_ref_artist")?,
            reference_complement: text("cad_ref_complement")?,
            lyrics: text("cad_lyrics")?,
            tags: serde_json::from_str(&tags).unwrap_or_default(),
            favorite: row.get::<_, i64>("cad_favorite")? == 1,
            date: row.get("cad_date")?,
            version: row.get::<_, Option<i64>>("cad_version")?.filter(|v| *v != 0).unwrap_or(1),
            linked_cards,
            urls: self.urls_of(&id, "cad").unwrap_or_default(),
            sort_number: row.get("cad_order")?,
            media_path: self.from_stored(&text("cad_media_path")?),
            ai_tweaks: serde_json::from_str(&tweaks).unwrap_or_else(|_| Value::Object(Default::default())),
            lang: text("cad_lang")?,
            env: text("cad_envid")?,
            project: row.get("cad_prjid")?,
            id,
        })
    }

    pub(crate) fn project_from_row(&self, row: &Row) -> rusqlite::Result<Project> {
        let id: String = row.get("prj_id")?;
        let mut items = self.db.prepare_cached("SELECT cad_id FROM card WHERE cad_prjid=? ORDER BY cad_order, cad_name")?;
        let card_ids = items.query_map([&id], |r| r.get(0))?.collect::<Result<Vec<String>, _>>()?;
        Ok(Project {
            name: row.get("prj_name")?,
            color: row.get("prj_color")?,
            date: row.get("prj_date")?,
            version: row.get::<_, Option<i64>>("prj_version")?.filter(|v| *v != 0).unwrap_or(1),
            parent_id: row.get::<_, Option<String>>("prj_parent")?.filter(|p| !p.is_empty()),
            card_ids,
            urls: self.urls_of(&id, "prj").unwrap_or_default(),
            env: row.get::<_, Option<String>>("prj_env")?.unwrap_or_default(),
            path: self.from_stored(&row.get::<_, Option<String>>("prj_path")?.unwrap_or_default()),
            finalized: row.get::<_, i64>("prj_finalized")? != 0,
            id,
        })
    }

    pub fn card(&self, id: &str) -> Res<Option<Card>> {
        self.db.query_row("SELECT * FROM card WHERE cad_id=?", [id], |r| self.card_from_row(r)).optional().map_err(err)
    }

    pub fn cards(&self) -> Res<Vec<Card>> {
        let mut st = self.db.prepare("SELECT * FROM card ORDER BY cad_date DESC").map_err(err)?;
        let rows = st.query_map([], |r| self.card_from_row(r)).map_err(err)?;
        rows.collect::<Result<_, _>>().map_err(err)
    }

    pub fn project(&self, id: &str) -> Res<Option<Project>> {
        self.db.query_row("SELECT * FROM project WHERE prj_id=?", [id], |r| self.project_from_row(r)).optional().map_err(err)
    }

    pub fn projects(&self) -> Res<Vec<Project>> {
        let mut st = self.db.prepare("SELECT * FROM project ORDER BY prj_date DESC").map_err(err)?;
        let rows = st.query_map([], |r| self.project_from_row(r)).map_err(err)?;
        rows.collect::<Result<_, _>>().map_err(err)
    }

    pub fn environments(&self) -> Res<Vec<Environment>> {
        let mut st = self.db.prepare("SELECT env_id,env_name,env_order,env_global FROM environment ORDER BY env_global DESC, env_order ASC").map_err(err)?;
        let rows = st.query_map([], |r| Ok(Environment { id: r.get(0)?, name: r.get(1)?, order: r.get(2)?, is_global: r.get::<_, i64>(3)? == 1 })).map_err(err)?;
        rows.collect::<Result<_, _>>().map_err(err)
    }

    /// The lowest version number not used by another card of this name and type.
    pub(crate) fn next_card_version(&self, name: &str, kind: &str, exclude: Option<&str>) -> Res<i64> {
        let mut st = self.db.prepare_cached("SELECT cad_version FROM card WHERE cad_name=? AND cad_type=? AND cad_id!=?").map_err(err)?;
        let used: Vec<i64> = st.query_map(params![name, kind, exclude.unwrap_or("")], |r| r.get(0)).map_err(err)?.collect::<Result<_, _>>().map_err(err)?;
        Ok((1..).find(|v| !used.contains(v)).unwrap_or(1))
    }

    pub(crate) fn next_project_version(&self, name: &str, exclude: Option<&str>) -> Res<i64> {
        let mut st = self.db.prepare_cached("SELECT prj_version FROM project WHERE prj_name=? AND prj_id!=?").map_err(err)?;
        let used: Vec<i64> = st.query_map(params![name, exclude.unwrap_or("")], |r| r.get(0)).map_err(err)?.collect::<Result<_, _>>().map_err(err)?;
        Ok((1..).find(|v| !used.contains(v)).unwrap_or(1))
    }
}

/// The part of `abs` below `root`, or `None` when it is not under it (another drive included).
pub fn relative_to_root(abs: &str, root: &str) -> Option<String> {
    if root.is_empty() || abs.is_empty() || !Path::new(abs).is_absolute() {
        return None;
    }
    let rel = Path::new(abs).strip_prefix(Path::new(root)).ok()?;
    let rel = rel.to_string_lossy().into_owned();
    (!rel.is_empty()).then_some(rel)
}

pub fn is_song(kind: &str) -> bool {
    matches!(kind, "song" | "song_l" | "song_e")
}
