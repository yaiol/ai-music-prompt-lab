//! Bringing data in: a JSON file (a backup of everything, or a set of prompts and projects) and an `.amlp`
//! LP file.
//!
//! ⚠ any new column of `card`, `project` or `card_translation` must be added to BOTH statements
//! of `import_json` (the update and the insert) and to the exports in the app. Leaving one out loses the
//! field without a sound: it goes out and comes back empty.

use crate::{
    amlp, langs,
    store::{Res, Store, err, new_id, now_ms},
};
use rusqlite::{OptionalExtension, params};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResults {
    pub cards_created: usize,
    pub cards_updated: usize,
    pub cards_skipped: usize,
    pub projects_created: usize,
    pub projects_updated: usize,
    pub song_links_added: usize,
    pub song_urls_added: usize,
    pub validation_errors: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AmlpResults {
    pub project_name: String,
    pub cards_created: usize,
    pub translations_created: usize,
    pub tracks: usize,
}

/// Why an `.amlp` was not imported. The window shows a message for each.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AmlpRefused {
    Empty,
    /// Songs cannot live in the Global environment.
    GlobalEnv,
    NoTracks,
}

fn is_uuid(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 36 && b.iter().enumerate().all(|(i, c)| if matches!(i, 8 | 13 | 18 | 23) { *c == b'-' } else { c.is_ascii_hexdigit() })
}

fn is_hex_colour(s: &str) -> bool {
    s.len() == 7 && s.starts_with('#') && s[1..].bytes().all(|c| c.is_ascii_hexdigit())
}

fn text<'a>(v: &'a Value, key: &str) -> &'a str {
    v[key].as_str().unwrap_or("")
}

/// A non-empty text, or NULL: what `value || null` gives.
fn some_text(v: &Value, key: &str) -> Option<String> {
    v[key].as_str().filter(|s| !s.is_empty()).map(str::to_owned)
}

fn truthy(v: &Value) -> bool {
    match v {
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0),
        Value::String(s) => !s.is_empty(),
        Value::Null => false,
        _ => true,
    }
}

/// A sort number from whatever the file holds: a number, or text that starts with one.
fn order_int(v: &Value) -> Option<i64> {
    match v {
        Value::Number(n) => n.as_f64().map(|f| f.trunc() as i64),
        Value::String(s) => {
            let t = s.trim();
            let end = t.char_indices().take_while(|(i, c)| c.is_ascii_digit() || (*i == 0 && (*c == '-' || *c == '+'))).map(|(i, c)| i + c.len_utf8()).last().unwrap_or(0);
            t[..end].parse().ok()
        }
        _ => None,
    }
}

/// The label a song link gets from its address. ⚠ The LABEL, not the address, is what tells whether a
/// song is "created" (an AI site) or "published" (a music site), so a link labelled wrong leaves an
/// imported song looking unmade with a working link on it. The same mapping is used when a link is
/// typed in the card dialog.
pub fn song_url_label(href: &str) -> String {
    for (needle, label) in [("mozartai.com", "mozartai"), ("producer.ai", "producer"), ("suno.com", "suno"), ("tunee.ai", "tunee"), ("musicfy.club", "musicfy"), ("wikipedia", "wikipedia")] {
        if href.contains(needle) {
            return label.to_owned();
        }
    }
    let host = href.split_once("://").map(|(_, rest)| rest.split(['/', '?', '#']).next().unwrap_or("")).unwrap_or("");
    let host = host.rsplit('@').next().unwrap_or(host).split(':').next().unwrap_or(host);
    if host.is_empty() {
        return "Link".to_owned();
    }
    let host = host.replacen("www.", "", 1);
    host.strip_suffix(".com").or_else(|| host.strip_suffix(".ai")).unwrap_or(&host).to_owned()
}

impl Store {
    /// Import a JSON file's content. `backup` = the file is a backup of everything: ids are kept and the
    /// environments come with it. Otherwise the cards and projects go into `target_env`, under NEW ids
    /// (an id from another environment must never be matched).
    pub fn import_json(&self, body: &Value, target_env: Option<&str>) -> Res<ImportResults> {
        let backup = body["type"] == "backup";
        let target_env = target_env.filter(|e| !e.is_empty()).unwrap_or(&self.default_env).to_owned();
        let empty = Vec::new();
        let cards = body["cards"].as_array().unwrap_or(&empty);
        let projects = body["projects"].as_array().unwrap_or(&empty);
        let mut out = ImportResults::default();

        // A backup's environments first. The map re-points the backup's ids at this database's, so the
        // cards and projects land right even when the backup was made on another machine.
        let mut env_map: HashMap<String, String> = HashMap::new();
        if backup {
            for e in body["environments"].as_array().unwrap_or(&empty) {
                let (id, name) = (text(e, "id"), text(e, "name"));
                if id.is_empty() || name.is_empty() {
                    continue;
                }
                let order = e["order"].as_i64().unwrap_or(0);
                let is_global = truthy(&e["isGlobal"]);
                if self.db.query_row("SELECT 1 FROM environment WHERE env_id=?", [id], |_| Ok(())).optional().map_err(err)?.is_some() {
                    env_map.insert(id.to_owned(), id.to_owned());
                    if !is_global {
                        self.db.execute("UPDATE environment SET env_name=?,env_order=? WHERE env_id=?", params![name, order, id]).map_err(err)?;
                    }
                } else if is_global {
                    // ⚠ Global is unique: always re-point at the one that exists, never add a second.
                    env_map.insert(id.to_owned(), self.global_env.clone());
                } else {
                    self.db.execute("INSERT INTO environment (env_id,env_name,env_order,env_date,env_global) VALUES (?,?,?,?,0)", params![id, format!("{name} (Restored)"), order, now_ms()]).map_err(err)?;
                    env_map.insert(id.to_owned(), id.to_owned());
                }
            }
        }
        let env_of = |v: &Value| -> String {
            if !backup {
                return target_env.clone();
            }
            match v["env"].as_str().filter(|e| !e.is_empty()) {
                Some(e) => env_map.get(e).cloned().unwrap_or_else(|| e.to_owned()),
                None => self.default_env.clone(),
            }
        };

        let tx = self.db.unchecked_transaction().map_err(err)?;
        for p in cards {
            let id = text(p, "id");
            let kind = match text(p, "type") {
                "song_l" | "song_e" => "song", // files written by older versions
                other => other,
            };
            if !is_uuid(id) {
                out.validation_errors.push(format!("Card skipped: invalid id \"{id}\""));
                continue;
            }
            let name = text(p, "name");
            if name.is_empty() {
                out.validation_errors.push(format!("Card skipped: name must be a non-empty string (id: {id})"));
                continue;
            }
            let Some(style) = p["style"].as_str() else {
                out.validation_errors.push(format!("Card skipped: style must be a string (id: {id})"));
                continue;
            };
            if !["music", "vocal", "song", "note"].contains(&kind) {
                out.validation_errors.push(format!("Card skipped: invalid type \"{kind}\" (id: {id})"));
                continue;
            }
            let tags = serde_json::to_string(p["tags"].as_array().unwrap_or(&empty)).map_err(err)?;
            let fav = truthy(&p["favorite"]) as i64;
            let lyrics = text(p, "lyrics");
            let order = order_int(&p["order"]);
            let media = self.to_stored(text(p, "media"));
            let tweaks = if truthy(&p["aiTweaks"]) { Some(p["aiTweaks"].to_string()) } else { None };
            let card_env = env_of(p);
            type Existing = (String, String, String, String, String, String, String, i64, Option<i64>);
            let existing: Option<Existing> = self
                .db
                .query_row(
                    if backup {
                        "SELECT cad_type,cad_name,cad_style,cad_lyrics,cad_ref_artist,cad_ref_complement,cad_tags,cad_favorite,cad_order FROM card WHERE cad_id=?1"
                    } else {
                        "SELECT cad_type,cad_name,cad_style,cad_lyrics,cad_ref_artist,cad_ref_complement,cad_tags,cad_favorite,cad_order FROM card WHERE cad_id=?1 AND cad_envid=?2"
                    },
                    rusqlite::params_from_iter(if backup { vec![id] } else { vec![id, target_env.as_str()] }),
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?, r.get(8)?)),
                )
                .optional()
                .map_err(err)?;
            let mut effective = id.to_owned();
            match existing {
                Some((e_kind, e_name, e_style, e_lyrics, e_artist, e_complement, e_tags, e_fav, e_order)) => {
                    let changed = kind != e_kind
                        || name != e_name
                        || style != e_style
                        || lyrics != e_lyrics
                        || text(p, "referenceArtist") != e_artist
                        || text(p, "referenceComplement") != e_complement
                        || tags != e_tags
                        || fav != e_fav
                        || order != e_order;
                    if changed {
                        tx.execute(
                            "UPDATE card SET cad_type=?,cad_name=?,cad_desc=COALESCE(?,cad_desc),cad_note=COALESCE(?,cad_note),cad_style=?,cad_ref_artist=COALESCE(?,cad_ref_artist),cad_ref_complement=COALESCE(?,cad_ref_complement),cad_lyrics=?,cad_tags=?,cad_favorite=?,cad_media_path=?,cad_order=?,cad_lang=COALESCE(?,cad_lang),cad_ai_tweaks=COALESCE(?,cad_ai_tweaks),cad_prjid=COALESCE(?,cad_prjid),cad_envid=? WHERE cad_id=?",
                            params![kind, name, some_text(p, "desc"), some_text(p, "note"), style, some_text(p, "referenceArtist"), some_text(p, "referenceComplement"), lyrics, tags, fav, media, order, some_text(p, "lang"), tweaks, some_text(p, "project"), card_env, id],
                        )
                        .map_err(err)?;
                        out.cards_updated += 1;
                    } else {
                        out.cards_skipped += 1;
                    }
                }
                None => {
                    if !backup {
                        effective = new_id();
                    }
                    tx.execute(
                        "INSERT INTO card(cad_id,cad_name,cad_version,cad_type,cad_desc,cad_note,cad_style,cad_ref_artist,cad_ref_complement,cad_lyrics,cad_tags,cad_favorite,cad_date,cad_media_path,cad_order,cad_lang,cad_ai_tweaks,cad_prjid,cad_envid) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
                        params![
                            effective, name, p["version"].as_i64().filter(|v| *v != 0).unwrap_or(1), kind, text(p, "desc"), text(p, "note"), style, text(p, "referenceArtist"), text(p, "referenceComplement"),
                            lyrics, tags, fav, p["date"].as_i64().filter(|d| *d != 0).unwrap_or_else(now_ms), media, order, text(p, "lang"), tweaks.unwrap_or_else(|| "{}".to_owned()), some_text(p, "project"), card_env
                        ],
                    )
                    .map_err(err)?;
                    out.cards_created += 1;
                }
            }
            if let Some(translations) = p["translations"].as_array() {
                for tr in translations {
                    let lang = text(tr, "lang");
                    if lang.is_empty() {
                        continue;
                    }
                    tx.execute(
                        "INSERT OR REPLACE INTO card_translation (cat_id,cat_cadid,cat_name,cat_lang,cat_lyrics,cat_date) VALUES (COALESCE((SELECT cat_id FROM card_translation WHERE cat_cadid=?1 AND cat_lang=?2),?3),?1,?4,?2,?5,?6)",
                        params![effective, lang, new_id(), text(tr, "name"), text(tr, "content"), now_ms()],
                    )
                    .map_err(err)?;
                }
            }
            if let Some(children) = p["cards"].as_array() {
                tx.execute("DELETE FROM card_link WHERE cal_cadid_parent=?", [&effective]).map_err(err)?;
                for child in children.iter().filter_map(Value::as_str) {
                    if self.db.query_row("SELECT 1 FROM card WHERE cad_id=?", [child], |_| Ok(())).optional().map_err(err)?.is_some() {
                        tx.execute("INSERT OR IGNORE INTO card_link (cal_cadid_parent,cal_cadid_child) VALUES (?,?)", params![effective, child]).map_err(err)?;
                        out.song_links_added += 1;
                    }
                }
            }
            if let Some(urls) = p["urls"].as_array() {
                tx.execute("DELETE FROM url WHERE url_parent_id=? AND url_type='cad'", [&effective]).map_err(err)?;
                for (i, u) in urls.iter().enumerate() {
                    tx.execute("INSERT INTO url (url_id,url_parent_id,url_type,url_label,url_href,url_order) VALUES (?,?,?,?,?,?)", params![new_id(), effective, "cad", text(u, "label"), text(u, "href"), i as i64]).map_err(err)?;
                    out.song_urls_added += 1;
                }
            }
        }

        // A card belongs to one project at most: which cards this file has already put in one.
        let mut claimed: HashMap<String, String> = HashMap::new();
        for proj in projects {
            let id = text(proj, "id");
            if !is_uuid(id) {
                out.validation_errors.push(format!("Project skipped: invalid id \"{id}\""));
                continue;
            }
            let name = text(proj, "name");
            if name.is_empty() {
                out.validation_errors.push(format!("Project skipped: name must be a non-empty string (id: {id})"));
                continue;
            }
            let mut colour = text(proj, "color").to_owned();
            if !colour.is_empty() && !is_hex_colour(&colour) {
                out.validation_errors.push(format!("Project \"{name}\": invalid color \"{colour}\", using default"));
                colour.clear();
            }
            if colour.is_empty() {
                colour = "#7c6fff".to_owned();
            }
            let exists = self
                .db
                .query_row(
                    if backup { "SELECT 1 FROM project WHERE prj_id=?1" } else { "SELECT 1 FROM project WHERE prj_id=?1 AND prj_env=?2" },
                    rusqlite::params_from_iter(if backup { vec![id] } else { vec![id, target_env.as_str()] }),
                    |_| Ok(()),
                )
                .optional()
                .map_err(err)?
                .is_some();
            let proj_env = env_of(proj);
            let path = self.to_stored(text(proj, "path"));
            let finalized = truthy(&proj["finalized"]) as i64;
            let mut effective = id.to_owned();
            if exists {
                tx.execute("UPDATE project SET prj_name=?,prj_color=?,prj_parent=?,prj_env=?,prj_path=?,prj_finalized=? WHERE prj_id=?", params![name, colour, some_text(proj, "parent"), proj_env, path, finalized, id]).map_err(err)?;
                out.projects_updated += 1;
            } else {
                if !backup {
                    effective = new_id();
                }
                tx.execute(
                    "INSERT INTO project (prj_id,prj_name,prj_color,prj_date,prj_version,prj_parent,prj_env,prj_path,prj_finalized) VALUES (?,?,?,?,?,?,?,?,?)",
                    params![effective, name, colour, proj["date"].as_i64().filter(|d| *d != 0).unwrap_or_else(now_ms), proj["version"].as_i64().filter(|v| *v != 0).unwrap_or(1), some_text(proj, "parent"), proj_env, path, finalized],
                )
                .map_err(err)?;
                out.projects_created += 1;
            }
            // Membership is stored on the card. New files: each card names its project (done above). Old
            // files: the project lists its cards, handled here.
            if let Some(members) = proj["cards"].as_array() {
                tx.execute("UPDATE card SET cad_prjid=NULL WHERE cad_prjid=?", [&effective]).map_err(err)?;
                for member in members.iter().filter_map(Value::as_str) {
                    let card_name: Option<String> = self.db.query_row("SELECT cad_name FROM card WHERE cad_id=?", [member], |r| r.get(0)).optional().map_err(err)?;
                    if let Some(first) = claimed.get(member) {
                        out.validation_errors.push(format!("Card \"{}\" skipped in project \"{name}\": already linked to project \"{first}\" in this file", card_name.as_deref().unwrap_or(member)));
                        continue;
                    }
                    if card_name.is_some() {
                        tx.execute("UPDATE card SET cad_prjid=? WHERE cad_id=?", params![effective, member]).map_err(err)?;
                        claimed.insert(member.to_owned(), name.to_owned());
                    }
                }
            }
            if let Some(urls) = proj["urls"].as_array() {
                tx.execute("DELETE FROM url WHERE url_parent_id=? AND url_type='prj'", [&effective]).map_err(err)?;
                for (i, u) in urls.iter().enumerate() {
                    tx.execute("INSERT INTO url (url_id,url_parent_id,url_type,url_label,url_href,url_order) VALUES (?,?,?,?,?,?)", params![new_id(), effective, "prj", text(u, "label"), text(u, "href"), i as i64]).map_err(err)?;
                }
            }
        }
        tx.commit().map_err(err)?;

        // The user's own AI presets and templates. The built-in ones are never touched: they come from the seed.
        for p in body["aiPresets"].as_array().unwrap_or(&empty) {
            let (id, task, block, name) = (text(p, "id"), text(p, "task"), text(p, "block"), text(p, "name"));
            let Some(content) = p["content"].as_str() else { continue };
            if id.is_empty() || task.is_empty() || block.is_empty() || name.is_empty() {
                continue;
            }
            match self.db.query_row("SELECT abp_default FROM ai_block_preset WHERE abp_id=?", [id], |r| r.get::<_, i64>(0)).optional().map_err(err)? {
                Some(1) => {}
                Some(_) => {
                    self.db.execute("UPDATE ai_block_preset SET abp_name=?,abp_content=? WHERE abp_id=?", params![name, content, id]).map_err(err)?;
                }
                None => {
                    let ord: i64 = self.db.query_row("SELECT COUNT(*) FROM ai_block_preset WHERE abp_task=? AND abp_block=?", params![task, block], |r| r.get(0)).map_err(err)?;
                    self.db.execute("INSERT INTO ai_block_preset (abp_id,abp_task,abp_block,abp_name,abp_content,abp_default,abp_order) VALUES (?,?,?,?,?,0,?)", params![id, task, block, name, content, ord]).map_err(err)?;
                }
            }
        }
        for t in body["aiTemplates"].as_array().unwrap_or(&empty) {
            let (id, task, name) = (text(t, "id"), text(t, "task"), text(t, "name"));
            if id.is_empty() || task.is_empty() || name.is_empty() {
                continue;
            }
            match self.db.query_row("SELECT atp_default FROM ai_template WHERE atp_id=?", [id], |r| r.get::<_, i64>(0)).optional().map_err(err)? {
                Some(1) => continue,
                Some(_) => {
                    self.db.execute("UPDATE ai_template SET atp_name=? WHERE atp_id=?", params![name, id]).map_err(err)?;
                }
                None => {
                    let ord: i64 = self.db.query_row("SELECT COUNT(*) FROM ai_template WHERE atp_task=?", [task], |r| r.get(0)).map_err(err)?;
                    self.db.execute("INSERT INTO ai_template (atp_id,atp_task,atp_name,atp_default,atp_order) VALUES (?,?,?,0,?)", params![id, task, name, ord]).map_err(err)?;
                }
            }
            for b in t["blocks"].as_array().unwrap_or(&empty) {
                let block = text(b, "block");
                if block.is_empty() {
                    continue;
                }
                let enabled = truthy(&b["enabled"]) as i64;
                match self.db.query_row("SELECT atb_id FROM ai_template_block WHERE atb_atpid=? AND atb_block=?", params![id, block], |r| r.get::<_, String>(0)).optional().map_err(err)? {
                    Some(existing) => {
                        self.db
                            .execute("UPDATE ai_template_block SET atb_abpid=?,atb_enabled=?,atb_name=COALESCE(?,atb_name),atb_prefix=COALESCE(?,atb_prefix) WHERE atb_id=?", params![some_text(b, "presetId"), enabled, some_text(b, "name"), some_text(b, "prefix"), existing])
                            .map_err(err)?;
                    }
                    None => {
                        self.db
                            .execute(
                                "INSERT INTO ai_template_block (atb_id,atb_atpid,atb_block,atb_abpid,atb_enabled,atb_name,atb_prefix) VALUES (?,?,?,?,?,?,?)",
                                params![new_id(), id, block, some_text(b, "presetId"), enabled, some_text(b, "name").unwrap_or_else(|| block.to_owned()), some_text(b, "prefix").unwrap_or_else(|| block.to_uppercase())],
                            )
                            .map_err(err)?;
                    }
                }
            }
        }
        Ok(out)
    }

    /// Import an `.amlp`: one song card per track. With `target_project` (the folder the user is in) the
    /// tracks join that project and take ITS environment; without one the LP brings its own project, named
    /// after its title.
    pub fn import_amlp(&self, text: &str, target_env: Option<&str>, target_project: Option<&str>) -> Res<Result<AmlpResults, AmlpRefused>> {
        let target_env = target_env.filter(|e| !e.is_empty()).unwrap_or(&self.default_env).to_owned();
        if text.trim().is_empty() {
            return Ok(Err(AmlpRefused::Empty));
        }
        if target_env == self.global_env {
            return Ok(Err(AmlpRefused::GlobalEnv));
        }
        let (header, tracks) = amlp::parse(text);
        if tracks.is_empty() {
            return Ok(Err(AmlpRefused::NoTracks));
        }
        let open: Option<(String, String, String)> = match target_project.filter(|p| !p.is_empty()) {
            Some(p) => self.db.query_row("SELECT prj_id, prj_name, prj_env FROM project WHERE prj_id=?", [p], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).optional().map_err(err)?,
            None => None,
        };
        let (project_id, project_name, card_env) = match &open {
            Some((id, name, env)) => (id.clone(), name.clone(), env.clone()),
            None => (new_id(), if header.title.is_empty() { "Imported LP".to_owned() } else { header.title.clone() }, target_env),
        };
        if card_env == self.global_env {
            return Ok(Err(AmlpRefused::GlobalEnv));
        }
        let lang = langs::code_of_language_text(&header.lang);
        // the header's date is YYYY-MM-DD; anything else means now
        let date = chrono::NaiveDate::parse_from_str(header.date.trim(), "%Y-%m-%d").ok().and_then(|d| d.and_hms_opt(0, 0, 0)).map(|d| d.and_utc().timestamp_millis()).unwrap_or_else(now_ms);
        let mut out = AmlpResults { project_name: project_name.clone(), tracks: tracks.len(), ..Default::default() };

        let tx = self.db.unchecked_transaction().map_err(err)?;
        if open.is_none() {
            let version = self.next_project_version(&project_name, None)?;
            tx.execute("INSERT INTO project (prj_id,prj_name,prj_color,prj_date,prj_version,prj_parent,prj_env,prj_path) VALUES (?,?,?,?,?,NULL,?,'')", params![project_id, project_name, "#7c6fff", date, version, card_env]).map_err(err)?;
        }
        for t in &tracks {
            let card = new_id();
            let name = if t.title.is_empty() { format!("Track {:02}", t.num) } else { t.title.clone() };
            let version = self.next_card_version(&name, "song", None)?;
            // A track may state its own language; the header's is the fallback, also when the track names
            // one that is not in the list.
            let track_lang = Some(langs::code_of_language_text(&t.lang)).filter(|c| !c.is_empty()).unwrap_or_else(|| lang.clone());
            tx.execute(
                "INSERT INTO card(cad_id,cad_envid,cad_prjid,cad_name,cad_version,cad_type,cad_order,cad_desc,cad_style,cad_ref_artist,cad_ref_complement,cad_lang,cad_lyrics,cad_ai_tweaks,cad_media_path,cad_note,cad_tags,cad_favorite,cad_date) VALUES (?,?,?,?,?,'song',?,?,?,?,?,?,?,'{}','','','[]',0,?)",
                params![card, card_env, project_id, name, version, t.num, t.description, t.style, t.reference_artist, t.reference_complement, track_lang, t.lyrics, date],
            )
            .map_err(err)?;
            // A track's address becomes the card's link, which is also what makes the song read as "created".
            let href = t.url.trim();
            if href.to_lowercase().starts_with("http://") || href.to_lowercase().starts_with("https://") {
                tx.execute("INSERT INTO url (url_id,url_parent_id,url_type,url_label,url_href,url_order) VALUES (?,?,?,?,?,1)", params![new_id(), card, "cad", song_url_label(href), href]).map_err(err)?;
            }
            for tr in &t.translations {
                tx.execute("INSERT INTO card_translation (cat_id,cat_cadid,cat_lang,cat_name,cat_lyrics,cat_date) VALUES (?,?,?,?,?,?)", params![new_id(), card, tr.lang, tr.title, tr.lyrics, date]).map_err(err)?;
                out.translations_created += 1;
            }
            out.cards_created += 1;
        }
        tx.commit().map_err(err)?;
        Ok(Ok(out))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_link_is_labelled_by_its_site() {
        assert_eq!(song_url_label("https://suno.com/song/abc"), "suno");
        assert_eq!(song_url_label("https://www.producer.ai/x"), "producer");
        assert_eq!(song_url_label("https://www.example.com/a?b"), "example");
        assert_eq!(song_url_label("https://soundcloud.com/x"), "soundcloud");
        assert_eq!(song_url_label("https://music.yaiol.org/x"), "music.yaiol.org");
        assert_eq!(song_url_label("not a link"), "Link");
    }

    #[test]
    fn ids_colours_and_sort_numbers_are_checked() {
        assert!(is_uuid("00000000-0000-4000-8000-000000000000"));
        assert!(!is_uuid("00000000-0000-4000-8000-00000000000"));
        assert!(is_hex_colour("#7c6fff") && !is_hex_colour("7c6fff") && !is_hex_colour("#7c6ffg"));
        assert_eq!(order_int(&serde_json::json!(7)), Some(7));
        assert_eq!(order_int(&serde_json::json!("12b")), Some(12));
        assert_eq!(order_int(&serde_json::json!("")), None);
        assert_eq!(order_int(&Value::Null), None);
    }
}
