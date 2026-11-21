//! The AI side: the built-in presets, templates and wizards (reconciled with their seed at every start),
//! the user's own, and the call to Gemini.
//!
//! The seed (`assets/ai-music.json`) is GENERATED from a flat text file by a compiler that lives outside
//! this app. ⚠ Never edit the JSON: it is a copy of the Electron app's `seeds/ai-music.json`, and the
//! single source is that app's `ai-music.txt`.

use crate::store::{Res, Store, err, new_id};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, sync::LazyLock, time::Duration};

static SEED: LazyLock<Value> = LazyLock::new(|| serde_json::from_str(include_str!("../assets/ai-music.json")).expect("ai-music.json is valid"));

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preset {
    pub id: String,
    pub task: String,
    pub block: String,
    pub name: String,
    pub content: String,
    /// Built in: it cannot be edited or deleted.
    pub is_default: bool,
    /// The one a block starts on.
    pub is_default_pick: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateBlock {
    pub id: String,
    pub block: String,
    pub preset_id: Option<String>,
    pub enabled: bool,
    pub name: String,
    pub prefix: String,
    pub hidden: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Template {
    pub id: String,
    pub task: String,
    pub name: String,
    pub is_default: bool,
    pub blocks: Vec<TemplateBlock>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct WizardStep {
    pub task: String,
    pub category: String,
    pub order: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Wizard {
    pub id: String,
    pub name: String,
    pub order: i64,
    pub steps: Vec<WizardStep>,
}

fn s<'a>(v: &'a Value, key: &str) -> &'a str {
    v[key].as_str().unwrap_or("")
}

fn truthy(v: &Value) -> bool {
    v.as_bool().unwrap_or(false)
}

/// The seed's block definitions: id -> (name, prefix, hidden).
fn block_defs() -> HashMap<String, (String, String, bool)> {
    SEED["blocks"].as_array().into_iter().flatten().map(|b| (s(b, "id").to_owned(), (s(b, "name").to_owned(), s(b, "prefix").to_owned(), truthy(&b["hidden"])))).collect()
}

/// The system blocks: read from the seed when a prompt is built, never stored in the database.
pub fn system_blocks() -> &'static Value {
    &SEED["system"]
}

impl Store {
    /// Bring the built-in presets, templates and wizards in line with the seed. Runs at every start.
    /// The user's own (not built in) are never touched.
    pub fn reconcile_ai_seed(&self) -> Res<()> {
        let empty = Vec::new();
        let defs = block_defs();
        let presets = SEED["presets"].as_array().unwrap_or(&empty);
        let templates = SEED["templates"].as_array().unwrap_or(&empty);
        let tx = self.db.unchecked_transaction().map_err(err)?;

        let mut preset_ids: HashMap<String, String> = HashMap::new();
        for (i, p) in presets.iter().enumerate() {
            let (task, block, name) = (s(p, "task"), s(p, "block"), s(p, "name"));
            let pick = truthy(&p["default"]) as i64;
            let id = match tx.query_row("SELECT abp_id FROM ai_block_preset WHERE abp_task=? AND abp_block=? AND abp_name=? AND abp_default=1", params![task, block, name], |r| r.get::<_, String>(0)).optional().map_err(err)? {
                Some(id) => {
                    tx.execute("UPDATE ai_block_preset SET abp_content=?, abp_order=?, abp_default_pick=? WHERE abp_id=?", params![s(p, "content"), i as i64, pick, id]).map_err(err)?;
                    id
                }
                None => {
                    let id = new_id();
                    tx.execute("INSERT INTO ai_block_preset (abp_id,abp_task,abp_block,abp_name,abp_content,abp_default,abp_default_pick,abp_order) VALUES (?,?,?,?,?,1,?,?)", params![id, task, block, name, s(p, "content"), pick, i as i64]).map_err(err)?;
                    id
                }
            };
            preset_ids.insert(format!("{task}|{block}|{name}"), id);
        }

        for (i, t) in templates.iter().enumerate() {
            let (task, name) = (s(t, "task"), s(t, "name"));
            let tid = match tx.query_row("SELECT atp_id FROM ai_template WHERE atp_task=? AND atp_name=? AND atp_default=1", params![task, name], |r| r.get::<_, String>(0)).optional().map_err(err)? {
                Some(id) => {
                    tx.execute("UPDATE ai_template SET atp_order=? WHERE atp_id=?", params![i as i64, id]).map_err(err)?;
                    id
                }
                None => {
                    let id = new_id();
                    tx.execute("INSERT INTO ai_template (atp_id,atp_task,atp_name,atp_default,atp_order) VALUES (?,?,?,1,?)", params![id, task, name, i as i64]).map_err(err)?;
                    id
                }
            };
            let blocks = t["blocks"].as_array().unwrap_or(&empty);
            for b in blocks {
                let block = s(b, "block");
                let preset = preset_ids.get(&format!("{task}|{block}|{}", s(b, "preset")));
                let (bname, prefix) = match defs.get(block) {
                    Some((n, p, _)) => (if n.is_empty() { block.to_owned() } else { n.clone() }, if p.is_empty() { block.to_uppercase() } else { p.clone() }),
                    None => (block.to_owned(), block.to_uppercase()),
                };
                let enabled = truthy(&b["enabled"]) as i64;
                match tx.query_row("SELECT atb_id FROM ai_template_block WHERE atb_atpid=? AND atb_block=?", params![tid, block], |r| r.get::<_, String>(0)).optional().map_err(err)? {
                    Some(existing) => tx.execute("UPDATE ai_template_block SET atb_abpid=?, atb_enabled=?, atb_name=?, atb_prefix=? WHERE atb_id=?", params![preset, enabled, bname, prefix, existing]),
                    None => tx.execute("INSERT INTO ai_template_block (atb_id,atb_atpid,atb_block,atb_abpid,atb_enabled,atb_name,atb_prefix) VALUES (?,?,?,?,?,?,?)", params![new_id(), tid, block, preset, enabled, bname, prefix]),
                }
                .map_err(err)?;
            }
            // blocks the built-in template no longer has
            let keep: Vec<&str> = blocks.iter().map(|b| s(b, "block")).collect();
            let existing: Vec<(String, String)> = {
                let mut st = tx.prepare("SELECT atb_id, atb_block FROM ai_template_block WHERE atb_atpid=?").map_err(err)?;
                let rows = st.query_map([&tid], |r| Ok((r.get(0)?, r.get(1)?))).map_err(err)?;
                rows.collect::<Result<_, _>>().map_err(err)?
            };
            for (id, block) in existing {
                if !keep.contains(&block.as_str()) {
                    tx.execute("DELETE FROM ai_template_block WHERE atb_id=?", [id]).map_err(err)?;
                }
            }
        }

        // built-in presets and templates the seed no longer has
        let old_presets: Vec<(String, String)> = {
            let mut st = tx.prepare("SELECT abp_id, abp_task || '|' || abp_block || '|' || abp_name FROM ai_block_preset WHERE abp_default=1").map_err(err)?;
            let rows = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?))).map_err(err)?;
            rows.collect::<Result<_, _>>().map_err(err)?
        };
        for (id, key) in old_presets {
            if !preset_ids.contains_key(&key) {
                tx.execute("UPDATE ai_template_block SET atb_abpid=NULL WHERE atb_abpid=?", [&id]).map_err(err)?;
                tx.execute("DELETE FROM ai_block_preset WHERE abp_id=?", [&id]).map_err(err)?;
            }
        }
        let keep_templates: Vec<String> = templates.iter().map(|t| format!("{}|{}", s(t, "task"), s(t, "name"))).collect();
        let old_templates: Vec<(String, String)> = {
            let mut st = tx.prepare("SELECT atp_id, atp_task || '|' || atp_name FROM ai_template WHERE atp_default=1").map_err(err)?;
            let rows = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?))).map_err(err)?;
            rows.collect::<Result<_, _>>().map_err(err)?
        };
        for (id, key) in old_templates {
            if !keep_templates.contains(&key) {
                tx.execute("DELETE FROM ai_template WHERE atp_id=?", [id]).map_err(err)?;
            }
        }

        // every template block takes its name and prefix from the seed's block definitions
        for (block, (name, prefix, _)) in &defs {
            tx.execute("UPDATE ai_template_block SET atb_name=?, atb_prefix=? WHERE atb_block=?", params![name, prefix, block]).map_err(err)?;
        }
        // system blocks are never in the database: they are resolved from the seed when a prompt is built
        for b in SEED["system"].as_array().unwrap_or(&empty) {
            let block = s(b, "block");
            tx.execute("DELETE FROM ai_template_block WHERE atb_block=?", [block]).map_err(err)?;
            tx.execute("DELETE FROM ai_block_preset WHERE abp_block=?", [block]).map_err(err)?;
        }

        let wizards = SEED["wizards"].as_array().unwrap_or(&empty);
        if !wizards.is_empty() {
            for (i, w) in wizards.iter().enumerate() {
                let id = s(w, "id");
                tx.execute("INSERT INTO ai_wizard (awz_id, awz_name, awz_order) VALUES (?1,?2,?3) ON CONFLICT(awz_id) DO UPDATE SET awz_name=?2, awz_order=?3", params![id, s(w, "name"), i as i64]).map_err(err)?;
                let steps = w["steps"].as_array().unwrap_or(&empty);
                for (si, step) in steps.iter().enumerate() {
                    tx.execute(
                        "INSERT INTO ai_wizard_task (awt_awzid, awt_task, awt_category, awt_order) VALUES (?1,?2,?3,?4) ON CONFLICT(awt_awzid, awt_task) DO UPDATE SET awt_category=?3, awt_order=?4",
                        params![id, s(step, "task"), s(step, "category"), si as i64],
                    )
                    .map_err(err)?;
                }
                let keep: Vec<&str> = steps.iter().map(|st| s(st, "task")).collect();
                let existing: Vec<String> = {
                    let mut st = tx.prepare("SELECT awt_task FROM ai_wizard_task WHERE awt_awzid=?").map_err(err)?;
                    let rows = st.query_map([id], |r| r.get(0)).map_err(err)?;
                    rows.collect::<Result<_, _>>().map_err(err)?
                };
                for task in existing {
                    if !keep.contains(&task.as_str()) {
                        tx.execute("DELETE FROM ai_wizard_task WHERE awt_awzid=? AND awt_task=?", params![id, task]).map_err(err)?;
                    }
                }
            }
            let keep: Vec<&str> = wizards.iter().map(|w| s(w, "id")).collect();
            let existing: Vec<String> = {
                let mut st = tx.prepare("SELECT awz_id FROM ai_wizard").map_err(err)?;
                let rows = st.query_map([], |r| r.get(0)).map_err(err)?;
                rows.collect::<Result<_, _>>().map_err(err)?
            };
            for id in existing {
                if !keep.contains(&id.as_str()) {
                    tx.execute("DELETE FROM ai_wizard WHERE awz_id=?", [id]).map_err(err)?;
                }
            }
        }
        tx.commit().map_err(err)
    }

    // ─── Presets ─────────────────────────────────────────────────────────────────────────────────

    pub fn ai_presets(&self, task: Option<&str>, block: Option<&str>) -> Res<Vec<Preset>> {
        let mut st = self
            .db
            .prepare("SELECT abp_id,abp_task,abp_block,abp_name,abp_content,abp_default,abp_default_pick FROM ai_block_preset WHERE (?1 IS NULL OR abp_task=?1) AND (?2 IS NULL OR abp_block=?2) ORDER BY abp_order")
            .map_err(err)?;
        let rows = st
            .query_map(params![task, block], |r| {
                Ok(Preset { id: r.get(0)?, task: r.get(1)?, block: r.get(2)?, name: r.get(3)?, content: r.get(4)?, is_default: r.get::<_, i64>(5)? == 1, is_default_pick: r.get::<_, i64>(6)? == 1 })
            })
            .map_err(err)?;
        rows.collect::<Result<_, _>>().map_err(err)
    }

    pub fn create_ai_preset(&self, task: &str, block: &str, name: &str, content: &str) -> Res<Preset> {
        if task.is_empty() || block.is_empty() || name.is_empty() {
            return Err("Missing fields".to_owned());
        }
        let id = new_id();
        let order: i64 = self.db.query_row("SELECT COUNT(*) FROM ai_block_preset WHERE abp_task=? AND abp_block=?", params![task, block], |r| r.get(0)).map_err(err)?;
        self.db.execute("INSERT INTO ai_block_preset (abp_id,abp_task,abp_block,abp_name,abp_content,abp_default,abp_order) VALUES (?,?,?,?,?,0,?)", params![id, task, block, name, content, order]).map_err(err)?;
        Ok(Preset { id, task: task.to_owned(), block: block.to_owned(), name: name.to_owned(), content: content.to_owned(), is_default: false, is_default_pick: false })
    }

    fn preset_is_default(&self, id: &str) -> Res<bool> {
        match self.db.query_row("SELECT abp_default FROM ai_block_preset WHERE abp_id=?", [id], |r| r.get::<_, i64>(0)).optional().map_err(err)? {
            Some(d) => Ok(d == 1),
            None => Err("Not found".to_owned()),
        }
    }

    pub fn update_ai_preset(&self, id: &str, name: Option<&str>, content: Option<&str>) -> Res<()> {
        if self.preset_is_default(id)? {
            return Err("Cannot edit a default preset".to_owned());
        }
        self.db.execute("UPDATE ai_block_preset SET abp_name=COALESCE(?,abp_name),abp_content=COALESCE(?,abp_content) WHERE abp_id=?", params![name, content, id]).map_err(err).map(|_| ())
    }

    /// Delete one of the user's presets; the template blocks that used it keep their place, with no preset.
    pub fn delete_ai_preset(&self, id: &str) -> Res<()> {
        if self.preset_is_default(id)? {
            return Err("Cannot delete a default preset".to_owned());
        }
        let tx = self.db.unchecked_transaction().map_err(err)?;
        tx.execute("UPDATE ai_template_block SET atb_abpid=NULL WHERE atb_abpid=?", [id]).map_err(err)?;
        tx.execute("DELETE FROM ai_block_preset WHERE abp_id=?", [id]).map_err(err)?;
        tx.commit().map_err(err)
    }

    // ─── Templates ───────────────────────────────────────────────────────────────────────────────

    fn template_blocks(&self, template: &str) -> Res<Vec<TemplateBlock>> {
        let defs = block_defs();
        let mut st = self.db.prepare("SELECT atb_id,atb_block,atb_abpid,atb_enabled,atb_name,atb_prefix FROM ai_template_block WHERE atb_atpid=?").map_err(err)?;
        let rows = st
            .query_map([template], |r| {
                let block: String = r.get(1)?;
                let name: String = r.get(4)?;
                let prefix: String = r.get(5)?;
                Ok(TemplateBlock {
                    id: r.get(0)?,
                    preset_id: r.get(2)?,
                    enabled: r.get::<_, i64>(3)? == 1,
                    name: if name.is_empty() { block.clone() } else { name },
                    prefix: if prefix.is_empty() { block.to_uppercase() } else { prefix },
                    hidden: defs.get(&block).is_some_and(|d| d.2),
                    block,
                })
            })
            .map_err(err)?;
        rows.collect::<Result<_, _>>().map_err(err)
    }

    pub fn ai_templates(&self, task: Option<&str>) -> Res<Vec<Template>> {
        let heads: Vec<(String, String, String, bool)> = {
            let mut st = self.db.prepare("SELECT atp_id,atp_task,atp_name,atp_default FROM ai_template WHERE (?1 IS NULL OR atp_task=?1) ORDER BY atp_order").map_err(err)?;
            let rows = st.query_map([task], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get::<_, i64>(3)? == 1))).map_err(err)?;
            rows.collect::<Result<_, _>>().map_err(err)?
        };
        heads.into_iter().map(|(id, task, name, is_default)| Ok(Template { blocks: self.template_blocks(&id)?, id, task, name, is_default })).collect()
    }

    pub fn ai_template(&self, id: &str) -> Res<Template> {
        self.ai_templates(None)?.into_iter().find(|t| t.id == id).ok_or_else(|| "Not found".to_owned())
    }

    /// A new template of the user's. `blocks` are (block, preset id, enabled, name, prefix).
    pub fn create_ai_template(&self, task: &str, name: &str, blocks: &[TemplateBlock]) -> Res<Template> {
        if task.is_empty() || name.is_empty() {
            return Err("Missing fields".to_owned());
        }
        let id = new_id();
        let order: i64 = self.db.query_row("SELECT COUNT(*) FROM ai_template WHERE atp_task=?", [task], |r| r.get(0)).map_err(err)?;
        let tx = self.db.unchecked_transaction().map_err(err)?;
        tx.execute("INSERT INTO ai_template (atp_id,atp_task,atp_name,atp_default,atp_order) VALUES (?,?,?,0,?)", params![id, task, name, order]).map_err(err)?;
        for b in blocks {
            tx.execute(
                "INSERT INTO ai_template_block (atb_id,atb_atpid,atb_block,atb_abpid,atb_enabled,atb_name,atb_prefix) VALUES (?,?,?,?,?,?,?)",
                params![new_id(), id, b.block, b.preset_id, b.enabled as i64, if b.name.is_empty() { &b.block } else { &b.name }, if b.prefix.is_empty() { b.block.to_uppercase() } else { b.prefix.clone() }],
            )
            .map_err(err)?;
        }
        tx.commit().map_err(err)?;
        self.ai_template(&id)
    }

    pub fn rename_ai_template(&self, id: &str, name: &str) -> Res<Template> {
        self.db.execute("UPDATE ai_template SET atp_name=? WHERE atp_id=?", params![name, id]).map_err(err)?;
        self.ai_template(id)
    }

    pub fn delete_ai_template(&self, id: &str) -> Res<()> {
        if self.ai_template(id)?.is_default {
            return Err("Cannot delete a default template".to_owned());
        }
        self.db.execute("DELETE FROM ai_template WHERE atp_id=?", [id]).map_err(err).map(|_| ())
    }

    /// Set one block of a template: its preset, whether it is on and, when given, its name and prefix.
    pub fn set_ai_template_block(&self, template: &str, block: &str, preset: Option<&str>, enabled: bool, name: Option<&str>, prefix: Option<&str>) -> Res<Template> {
        self.ai_template(template)?;
        match self.db.query_row("SELECT atb_id FROM ai_template_block WHERE atb_atpid=? AND atb_block=?", params![template, block], |r| r.get::<_, String>(0)).optional().map_err(err)? {
            Some(existing) => self.db.execute(
                "UPDATE ai_template_block SET atb_abpid=?,atb_enabled=?,atb_name=COALESCE(?,atb_name),atb_prefix=COALESCE(?,atb_prefix) WHERE atb_id=?",
                params![preset, enabled as i64, name, prefix, existing],
            ),
            None => self.db.execute(
                "INSERT INTO ai_template_block (atb_id,atb_atpid,atb_block,atb_abpid,atb_enabled,atb_name,atb_prefix) VALUES (?,?,?,?,?,?,?)",
                params![new_id(), template, block, preset, enabled as i64, name.unwrap_or(block), prefix.map(str::to_owned).unwrap_or_else(|| block.to_uppercase())],
            ),
        }
        .map_err(err)?;
        self.ai_template(template)
    }

    /// Take a block out of one of the user's templates (a built-in template keeps its blocks).
    pub fn remove_ai_template_block(&self, template: &str, block: &str) -> Res<Template> {
        if self.ai_template(template)?.is_default {
            return Err("Cannot modify a default template".to_owned());
        }
        self.db.execute("DELETE FROM ai_template_block WHERE atb_atpid=? AND atb_block=?", params![template, block]).map_err(err)?;
        self.ai_template(template)
    }

    pub fn ai_wizards(&self) -> Res<Vec<Wizard>> {
        let heads: Vec<(String, String, i64)> = {
            let mut st = self.db.prepare("SELECT awz_id, awz_name, awz_order FROM ai_wizard ORDER BY awz_order").map_err(err)?;
            let rows = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).map_err(err)?;
            rows.collect::<Result<_, _>>().map_err(err)?
        };
        heads
            .into_iter()
            .map(|(id, name, order)| {
                let mut st = self.db.prepare("SELECT awt_task, awt_category, awt_order FROM ai_wizard_task WHERE awt_awzid=? ORDER BY awt_order").map_err(err)?;
                let steps = st.query_map([&id], |r| Ok(WizardStep { task: r.get(0)?, category: r.get(1)?, order: r.get(2)? })).map_err(err)?.collect::<Result<_, _>>().map_err(err)?;
                Ok(Wizard { id, name, order, steps })
            })
            .collect()
    }
}

// ─── Gemini ─────────────────────────────────────────────────────────────────────────────────────

/// What a call asks for. `kind` shapes the limits: `music_style`, `vocal_style`, `lyrics`, `translate`.
#[derive(Clone, Debug, Default)]
pub struct Generate {
    pub request: String,
    pub kind: String,
    pub api_key: String,
    /// The longest answer wanted, in characters (0 = the kind's own limit).
    pub limit: usize,
    pub max_tokens: Option<u32>,
    /// Give the model's text back as it is: no reading of NAME / CONTENT, no cut.
    pub raw: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Generated {
    pub text: String,
    pub name_suggestions: Vec<String>,
}

/// Read a model's answer: a `NAME:` list of up to five titles (numbered or not, on the line or the lines after the key) and the
/// `CONTENT:` text (the whole answer when it has no such key), cut at `limit` characters on a word boundary.
pub fn read_answer(raw: &str, limit: usize) -> Generated {
    let bag = crate::wizard::parse_response(raw);
    let numbering = regex::Regex::new(r"^\d+\.\s*").unwrap();
    let suggestions: Vec<String> = bag
        .get("NAME")
        .map(|names| names.lines().map(|l| numbering.replace(l.trim(), "").trim().to_owned()).filter(|l| !l.is_empty()).take(5).collect())
        .unwrap_or_default();
    let mut text = bag.get("CONTENT").map(|c| c.trim()).filter(|c| !c.is_empty()).unwrap_or(raw.trim()).to_owned();
    if limit > 0 && text.chars().count() > limit {
        let cut: String = text.chars().take(limit).collect();
        // back to the end of the last whole word
        text = match cut.rfind(char::is_whitespace) {
            Some(i) => cut[..i].trim_end().to_owned(),
            None => cut,
        };
    }
    Generated { text, name_suggestions: suggestions }
}

/// Ask Gemini. Blocking: call it off the window's thread.
pub fn generate(g: &Generate) -> Res<Generated> {
    if g.api_key.is_empty() {
        return Err("No API key provided. Add your Gemini key in Settings.".to_owned());
    }
    let limit = if g.limit > 0 {
        g.limit
    } else {
        match g.kind.as_str() {
            "vocal_style" => 300,
            "music_style" => 700,
            _ => 1000,
        }
    };
    let mut config = serde_json::json!({ "temperature": 0.95, "maxOutputTokens": g.max_tokens.unwrap_or(if g.kind == "lyrics" { 4096 } else { 2048 }) });
    if g.kind == "translate" {
        config["thinkingConfig"] = serde_json::json!({ "thinkingBudget": 0 });
    }
    let body = serde_json::json!({ "contents": [{ "parts": [{ "text": g.request }] }], "generationConfig": config });
    let url = format!("https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent?key={}", g.api_key);
    let mut resp = ureq::post(&url)
        .header("Content-Type", "application/json")
        .config()
        .http_status_as_error(false)
        .timeout_global(Some(Duration::from_secs(120)))
        .build()
        .send(body.to_string())
        .map_err(err)?;
    let json: Value = serde_json::from_str(&resp.body_mut().read_to_string().map_err(err)?).map_err(|_| "Parse error".to_owned())?;
    if let Some(message) = json["error"]["message"].as_str() {
        return Err(message.to_owned());
    }
    let raw = json["candidates"][0]["content"]["parts"][0]["text"].as_str().unwrap_or("");
    if g.raw {
        return Ok(Generated { text: raw.to_owned(), name_suggestions: Vec::new() });
    }
    if g.kind == "translate" {
        return Ok(Generated { text: raw.trim().to_owned(), name_suggestions: Vec::new() });
    }
    Ok(read_answer(raw, limit))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_answer_is_read_into_names_and_text() {
        let g = read_answer("NAME:\n1. First\n2.  Second \n\nCONTENT:\n slow strings, a dark choir \n", 0);
        assert_eq!(g.name_suggestions, vec!["First", "Second"]);
        assert_eq!(g.text, "slow strings, a dark choir");
        // no markers: the whole answer is the text
        assert_eq!(read_answer("  just text ", 0), Generated { text: "just text".into(), name_suggestions: vec![] });
        // cut on a word boundary
        assert_eq!(read_answer("CONTENT:\none two three four", 12).text, "one two");
    }

    #[test]
    fn the_seed_is_read() {
        assert!(!block_defs().is_empty());
        assert!(system_blocks().is_array());
    }
}
