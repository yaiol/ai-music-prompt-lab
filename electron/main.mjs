// data-icon="yaiol:ai-music-prompt-lab"
import { app, BrowserWindow, shell, dialog, clipboard } from "electron";
import path from "path";
import fs from "fs";
import net from "net";
import crypto from "crypto";
import os from "os";
import https from "https";
import express from "express";
import cors from "cors";
import Database from "better-sqlite3";
import { spawn } from "child_process";
import { Document, Packer, Paragraph, TextRun, ImageRun, AlignmentType } from "docx";
import { fileURLToPath } from "node:url";
import { getLangName, SONG_LANGUAGES } from "./lang-helper.js";
import { parseAmlp } from "./parse-amlp.js";
import pkg from "../package.json" with { type: "json" };
import { mark, dumpStartupTiming } from "./startup-timing.mjs";
mark("electron boot + module imports");

// ESM has no __dirname - derive it from import.meta.url.
const __dirname = path.dirname(fileURLToPath(import.meta.url));
const isDev = !app.isPackaged;
const DEV_PORT = pkg.devPort;
const APP_NAME = pkg.productName;
// Storage namespace - single source: package.json `storagePrefix`. Never hardcode a prefix.
const STORAGE_PREFIX = pkg.storagePrefix;

// Fixed UUID for the Global environment - identical across all installations so
// backup files can be shared between machines without env ID mismatches.
const GLOBAL_ENV_ID = "00000000-0000-4000-8000-000000000000";
app.setPath('userData', path.join(app.getPath('appData'), 'yaiol', isDev ? `${pkg.productName} (Dev)` : pkg.productName));


let server;
let SERVER_PORT = 4000;

function startServer(callback) {

  function findFreePort(port, cb) {
    const tester = net.createServer();
    tester.once("error", () => findFreePort(port + 1, cb));
    tester.once("listening", () => { tester.close(() => cb(port)); });
    tester.listen(port);
  }

  // Dev and prod both store under userData — the `(Dev)` suffix on the userData path (above)
  // already keeps the two instances' data separate, so there's no dev/prod branch here.
  const dbPath = path.join(app.getPath("userData"), `${STORAGE_PREFIX}.db`);
  const settingsPath = path.join(app.getPath("userData"), "settings.json");

  function readSettings() {
    try { return JSON.parse(fs.readFileSync(settingsPath, "utf8")); } catch { return {}; }
  }
  function writeSettings(data) {
    fs.writeFileSync(settingsPath, JSON.stringify(data, null, 2), "utf8");
  }

  const db = new Database(dbPath);
  db.pragma("foreign_keys = ON");
  mark("sqlite opened");

  // ── Schema ───────────────────────────────────────────────────────────────────
  db.exec(`
    CREATE TABLE IF NOT EXISTS card (
      cad_id         TEXT PRIMARY KEY,
      cad_envid      TEXT NOT NULL DEFAULT '',
      cad_prjid      TEXT REFERENCES project(prj_id) ON DELETE SET NULL,
      cad_name       TEXT NOT NULL,
      cad_version    INTEGER NOT NULL DEFAULT 1,
      cad_type       TEXT NOT NULL,
      cad_order      INTEGER,
      cad_desc       TEXT NOT NULL DEFAULT '',
      cad_style      TEXT NOT NULL DEFAULT '',
      cad_lang       TEXT NOT NULL DEFAULT '',
      cad_lyrics     TEXT NOT NULL DEFAULT '',
      cad_ai_tweaks  TEXT NOT NULL DEFAULT '{}',
      cad_media_path TEXT NOT NULL DEFAULT '',
      cad_note       TEXT NOT NULL DEFAULT '',
      cad_tags       TEXT NOT NULL DEFAULT '[]',
      cad_favorite   INTEGER NOT NULL DEFAULT 0,
      cad_date       INTEGER NOT NULL
    );

    CREATE TABLE IF NOT EXISTS card_link (
      cal_cadid_parent TEXT NOT NULL,
      cal_cadid_child  TEXT NOT NULL,
      PRIMARY KEY (cal_cadid_parent, cal_cadid_child),
      FOREIGN KEY (cal_cadid_parent) REFERENCES card(cad_id) ON DELETE CASCADE,
      FOREIGN KEY (cal_cadid_child)  REFERENCES card(cad_id) ON DELETE CASCADE
    );

    CREATE TABLE IF NOT EXISTS project (
      prj_id      TEXT PRIMARY KEY,
      prj_name    TEXT NOT NULL,
      prj_version INTEGER NOT NULL DEFAULT 1,
      prj_color   TEXT NOT NULL DEFAULT '#7c6fff',
      prj_date    INTEGER NOT NULL,
      prj_parent  TEXT DEFAULT NULL,
      prj_path    TEXT NOT NULL DEFAULT '',
      prj_env     TEXT NOT NULL DEFAULT '',
      prj_finalized INTEGER NOT NULL DEFAULT 0
    );

    CREATE TABLE IF NOT EXISTS url (
      url_id          TEXT PRIMARY KEY,
      url_parent_id   TEXT NOT NULL,
      url_type        TEXT NOT NULL DEFAULT 'cad',
      url_label       TEXT NOT NULL DEFAULT '',
      url_href        TEXT NOT NULL,
      url_order       INTEGER NOT NULL DEFAULT 0
    );
  `);

  db.exec(`
    CREATE TABLE IF NOT EXISTS card_translation (
      cat_id      TEXT PRIMARY KEY,
      cat_cadid   TEXT NOT NULL REFERENCES card(cad_id) ON DELETE CASCADE,
      cat_lang    TEXT NOT NULL,
      cat_name    TEXT NOT NULL DEFAULT '',
      cat_lyrics  TEXT NOT NULL DEFAULT '',
      cat_date    INTEGER NOT NULL,
      UNIQUE(cat_cadid, cat_lang)
    );
  `);

  db.exec(`
    CREATE TABLE IF NOT EXISTS environment (
      env_id     TEXT PRIMARY KEY,
      env_name   TEXT NOT NULL,
      env_order  INTEGER NOT NULL DEFAULT 0,
      env_date   INTEGER NOT NULL,
      env_global INTEGER NOT NULL DEFAULT 0
    );
  `);

  // ── AI template tables ────────────────────────────────────────────────────────
  db.exec(`
    CREATE TABLE IF NOT EXISTS ai_block_preset (
      abp_id           TEXT PRIMARY KEY,
      abp_task         TEXT NOT NULL,
      abp_block        TEXT NOT NULL,
      abp_name         TEXT NOT NULL,
      abp_content      TEXT NOT NULL,
      abp_default      INTEGER NOT NULL DEFAULT 0,
      abp_order        INTEGER NOT NULL DEFAULT 0,
      abp_default_pick INTEGER NOT NULL DEFAULT 0
    );

    CREATE TABLE IF NOT EXISTS ai_template (
      atp_id      TEXT PRIMARY KEY,
      atp_task    TEXT NOT NULL,
      atp_name    TEXT NOT NULL,
      atp_default INTEGER NOT NULL DEFAULT 0,
      atp_order   INTEGER NOT NULL DEFAULT 0
    );

    CREATE TABLE IF NOT EXISTS ai_template_block (
      atb_id       TEXT PRIMARY KEY,
      atb_atpid    TEXT NOT NULL,
      atb_block    TEXT NOT NULL,
      atb_abpid    TEXT,
      atb_enabled  INTEGER NOT NULL DEFAULT 1,
      atb_name     TEXT NOT NULL DEFAULT '',
      atb_prefix   TEXT NOT NULL DEFAULT '',
      FOREIGN KEY (atb_atpid) REFERENCES ai_template(atp_id) ON DELETE CASCADE,
      FOREIGN KEY (atb_abpid) REFERENCES ai_block_preset(abp_id) ON DELETE SET NULL
    );

    CREATE TABLE IF NOT EXISTS ai_wizard (
      awz_id    TEXT PRIMARY KEY,
      awz_name  TEXT NOT NULL,
      awz_order INTEGER NOT NULL DEFAULT 0
    );

    CREATE TABLE IF NOT EXISTS ai_wizard_task (
      awt_awzid    TEXT NOT NULL,
      awt_task     TEXT NOT NULL,
      awt_category TEXT NOT NULL,
      awt_order    INTEGER NOT NULL DEFAULT 0,
      PRIMARY KEY (awt_awzid, awt_task),
      FOREIGN KEY (awt_awzid) REFERENCES ai_wizard(awz_id) ON DELETE CASCADE
    );
  `);


  // ── Seed environments ─────────────────────────────────────────────────────────
  let defaultEnvId, globalEnvId;
  {
    const envRows = db.prepare("SELECT * FROM environment").all();
    const globalRow = envRows.find(r => r.env_global === 1);
    if (!globalRow) {
      globalEnvId = GLOBAL_ENV_ID;
      db.prepare("INSERT INTO environment (env_id,env_name,env_order,env_date,env_global) VALUES (?,?,?,?,1)").run(globalEnvId, "(Global)", 0, Date.now());
    } else {
      globalEnvId = globalRow.env_id;
    }
    const regularRows = envRows.filter(r => r.env_global === 0);
    if (regularRows.length === 0) {
      defaultEnvId = crypto.randomUUID();
      db.prepare("INSERT INTO environment (env_id,env_name,env_order,env_date,env_global) VALUES (?,?,?,?,0)").run(defaultEnvId, "Default", 1, Date.now());
    } else {
      defaultEnvId = db.prepare("SELECT env_id FROM environment WHERE env_global=0 ORDER BY env_order ASC").get().env_id;
    }
    db.prepare("UPDATE card SET cad_envid=? WHERE cad_envid=''").run(defaultEnvId);
    db.prepare("UPDATE project SET prj_env=? WHERE prj_env=''").run(defaultEnvId);
  }

  mark("schema + migrations");

  // ── Seed default AI presets & templates ──────────────────────────────────────
  const defaults = JSON.parse(fs.readFileSync(path.join(__dirname, "..", "seeds", "ai-music.json"), "utf8"));

  // Build global block definitions map: id → { label, prefix }
  const blockDefs = {};
  for (const b of (defaults.blocks || [])) {
    blockDefs[b.id] = { name: b.name, prefix: b.prefix };
  }

  // Build system blocks map: block id → { prefix, tasks: { task → content } }
  const systemBlocks = (defaults.system || []);
  const systemBlockIds = systemBlocks.map(b => b.block);

  // ── Reconcile defaults from seeds/ai-music.json (runs on every launch) ─────
  const presetIds = {}; // key: "task|block|name" → id

  db.transaction(() => {
    // Presets - upsert by task+block+name+default
    defaults.presets.forEach((p, i) => {
      const existing = db.prepare("SELECT abp_id FROM ai_block_preset WHERE abp_task=? AND abp_block=? AND abp_name=? AND abp_default=1").get(p.task, p.block, p.name);
      if (existing) {
        db.prepare("UPDATE ai_block_preset SET abp_content=?, abp_order=?, abp_default_pick=? WHERE abp_id=?").run(p.content, i, p.default ? 1 : 0, existing.abp_id);
        presetIds[`${p.task}|${p.block}|${p.name}`] = existing.abp_id;
      } else {
        const id = crypto.randomUUID();
        db.prepare("INSERT INTO ai_block_preset (abp_id,abp_task,abp_block,abp_name,abp_content,abp_default,abp_default_pick,abp_order) VALUES (?,?,?,?,?,1,?,?)").run(id, p.task, p.block, p.name, p.content, p.default ? 1 : 0, i);
        presetIds[`${p.task}|${p.block}|${p.name}`] = id;
      }
    });

    // Templates - upsert by task+name+default
    defaults.templates.forEach((t, i) => {
      const existing = db.prepare("SELECT atp_id FROM ai_template WHERE atp_task=? AND atp_name=? AND atp_default=1").get(t.task, t.name);
      const tid = existing ? existing.atp_id : crypto.randomUUID();
      if (existing) {
        db.prepare("UPDATE ai_template SET atp_order=? WHERE atp_id=?").run(i, tid);
      } else {
        db.prepare("INSERT INTO ai_template (atp_id,atp_task,atp_name,atp_default,atp_order) VALUES (?,?,?,1,?)").run(tid, t.task, t.name, i);
      }
      // Template blocks - upsert by template+block
      const keepBlocks = t.blocks.map(b => b.block);
      for (const b of t.blocks) {
        const presetId = presetIds[`${t.task}|${b.block}|${b.preset}`] ?? null;
        const nm  = blockDefs[b.block]?.name   || b.block;
        const pfx = blockDefs[b.block]?.prefix || b.block.toUpperCase();
        const existingBlock = db.prepare("SELECT atb_id FROM ai_template_block WHERE atb_atpid=? AND atb_block=?").get(tid, b.block);
        if (existingBlock) {
          db.prepare("UPDATE ai_template_block SET atb_abpid=?, atb_enabled=?, atb_name=?, atb_prefix=? WHERE atb_id=?").run(presetId, b.enabled ? 1 : 0, nm, pfx, existingBlock.atb_id);
        } else {
          db.prepare("INSERT INTO ai_template_block (atb_id,atb_atpid,atb_block,atb_abpid,atb_enabled,atb_name,atb_prefix) VALUES (?,?,?,?,?,?,?)").run(crypto.randomUUID(), tid, b.block, presetId, b.enabled ? 1 : 0, nm, pfx);
        }
      }
      // Delete blocks no longer in this default template's definition
      db.prepare(
        `DELETE FROM ai_template_block WHERE atb_atpid=? AND atb_block NOT IN (${keepBlocks.map(() => "?").join(",")})`
      ).run(tid, ...keepBlocks);
    });

    // Delete default presets no longer in the JSON
    const keepPresets = defaults.presets.map(p => `${p.task}|${p.block}|${p.name}`);
    const oldPresets = db.prepare("SELECT abp_id, abp_task, abp_block, abp_name FROM ai_block_preset WHERE abp_default=1").all();
    for (const row of oldPresets) {
      if (!keepPresets.includes(`${row.abp_task}|${row.abp_block}|${row.abp_name}`)) {
        db.prepare("UPDATE ai_template_block SET atb_abpid=NULL WHERE atb_abpid=?").run(row.abp_id);
        db.prepare("DELETE FROM ai_block_preset WHERE abp_id=?").run(row.abp_id);
      }
    }

    // Delete default templates no longer in the JSON
    const keepTemplates = defaults.templates.map(t => `${t.task}|${t.name}`);
    const oldTemplates = db.prepare("SELECT atp_id, atp_task, atp_name FROM ai_template WHERE atp_default=1").all();
    for (const row of oldTemplates) {
      if (!keepTemplates.includes(`${row.atp_task}|${row.atp_name}`)) {
        db.prepare("DELETE FROM ai_template WHERE atp_id=?").run(row.atp_id);
      }
    }
  })();


  // Reconcile atb_name / atb_prefix for ALL template blocks from global block definitions
  db.transaction(() => {
    for (const [blockId, def] of Object.entries(blockDefs)) {
      db.prepare(
        "UPDATE ai_template_block SET atb_name=?, atb_prefix=? WHERE atb_block=?"
      ).run(def.name, def.prefix, blockId);
    }
  })();

  // Remove system blocks from DB - they are resolved at runtime from seeds
  if (systemBlockIds.length > 0) {
    db.transaction(() => {
      const placeholders = systemBlockIds.map(() => "?").join(",");
      db.prepare(`DELETE FROM ai_template_block WHERE atb_block IN (${placeholders})`).run(...systemBlockIds);
      db.prepare(`DELETE FROM ai_block_preset   WHERE abp_block  IN (${placeholders})`).run(...systemBlockIds);
    })();
  }

  // ── Seed wizard definitions ──────────────────────────────────────────────────
  if (defaults.wizards?.length) {
    db.transaction(() => {
      const keepWizardIds = defaults.wizards.map(w => w.id);

      defaults.wizards.forEach((w, i) => {
        const existing = db.prepare("SELECT 1 FROM ai_wizard WHERE awz_id=?").get(w.id);
        if (existing) {
          db.prepare("UPDATE ai_wizard SET awz_name=?, awz_order=? WHERE awz_id=?").run(w.name, i, w.id);
        } else {
          db.prepare("INSERT INTO ai_wizard (awz_id, awz_name, awz_order) VALUES (?,?,?)").run(w.id, w.name, i);
        }

        // Upsert wizard tasks (steps)
        const keepTasks = [];
        (w.steps || []).forEach((step, si) => {
          keepTasks.push(step.task);
          const existingStep = db.prepare("SELECT 1 FROM ai_wizard_task WHERE awt_awzid=? AND awt_task=?").get(w.id, step.task);
          if (existingStep) {
            db.prepare("UPDATE ai_wizard_task SET awt_category=?, awt_order=? WHERE awt_awzid=? AND awt_task=?").run(step.category || "", si, w.id, step.task);
          } else {
            db.prepare("INSERT INTO ai_wizard_task (awt_awzid, awt_task, awt_category, awt_order) VALUES (?,?,?,?)").run(w.id, step.task, step.category || "", si);
          }
        });

        // Delete steps no longer in this wizard
        if (keepTasks.length) {
          db.prepare(`DELETE FROM ai_wizard_task WHERE awt_awzid=? AND awt_task NOT IN (${keepTasks.map(() => "?").join(",")})`).run(w.id, ...keepTasks);
        } else {
          db.prepare("DELETE FROM ai_wizard_task WHERE awt_awzid=?").run(w.id);
        }
      });

      // Delete wizards no longer in the JSON
      const oldWizards = db.prepare("SELECT awz_id FROM ai_wizard").all();
      for (const row of oldWizards) {
        if (!keepWizardIds.includes(row.awz_id)) {
          db.prepare("DELETE FROM ai_wizard WHERE awz_id=?").run(row.awz_id);
        }
      }
    })();
  }

  // ── Helpers ──────────────────────────────────────────────────────────────────
  // ⚠ CLAUDE: "With lyrics" means the song actually has words. A song whose lyrics are
  // only structure markers ([Intro], (chorus), …) is instrumental — exclude it from the
  // "withLyrics" export filter, otherwise it exports with an empty (cleaned-away) lyric block.
  function hasRealLyrics(raw) {
    return (raw || "").split("\n").some(line => {
      const t = line.trim();
      return t && !/^\[.*\]$/.test(t) && !/^\(.*\)$/.test(t);
    });
  }

  // ── Export lyric content — format-independent ─────────────────────────────────
  // ⚠ CLAUDE: the lyric CONTENT (which lines to show) is identical for docx/md/adoc.
  // Compute it here once; each exporter only wraps these lines in its own syntax.
  // mode "clean" strips [..]/(..) section markers; "original" keeps them. Blank-normalized.
  function extractLyrics(raw, mode) {
    const out = [];
    let lastBlank = true;
    for (const line of (raw || "").split("\n")) {
      const t = line.trim();
      if (!t) { if (!lastBlank) { out.push(""); lastBlank = true; } continue; }
      if (mode === "clean" && (/^\[.*\]$/.test(t) || /^\(.*\)$/.test(t))) continue;
      out.push(t); lastBlank = false;
    }
    while (out.length && out[out.length - 1] === "") out.pop();
    return out;
  }

  // Lyric lines to render for a SONG: a song with no actual words is instrumental in BOTH
  // modes — markers-only or blank collapses to the marker, whether cleaned or original.
  function songLyricsLines(raw, mode, instrumentalLabel) {
    if (!hasRealLyrics(raw)) return [`(${instrumentalLabel})`];
    return extractLyrics(raw, mode);
  }

  function toOrderInt(val) {
    if (val === null || val === undefined || val === '') return null;
    const n = parseInt(val, 10);
    return isNaN(n) ? null : n;
  }

  function rowToCard(row) {
    const links = db.prepare("SELECT cal_cadid_child FROM card_link WHERE cal_cadid_parent = ?").all(row.cad_id);
    const urls  = db.prepare("SELECT url_id,url_label,url_href FROM url WHERE url_parent_id=? AND url_type='cad' ORDER BY url_order").all(row.cad_id);
    return {
      id:            row.cad_id,
      type:          row.cad_type,
      name:          row.cad_name,
      desc:          row.cad_desc || '',
      note:          row.cad_note || '',
      style:         row.cad_style,
      lyrics:        row.cad_lyrics || '',
      tags:          JSON.parse(row.cad_tags || '[]'),
      favorite:      row.cad_favorite === 1,
      date:          row.cad_date,
      version:       row.cad_version || 1,
      linkedCards: links.map(l => l.cal_cadid_child),
      urls:          urls.map(u => ({ id: u.url_id, label: u.url_label, href: u.url_href })),
      sortNumber:    row.cad_order ?? null,
      mediaPath:     row.cad_media_path || '',
      aiTweaks:      JSON.parse(row.cad_ai_tweaks || '{}'),
      lang:          row.cad_lang || '',
      env:           row.cad_envid || '',
      project:       row.cad_prjid || null,
    };
  }

  function rowToProject(row) {
    const items = db.prepare("SELECT cad_id FROM card WHERE cad_prjid=? ORDER BY cad_order, cad_name").all(row.prj_id);
    const urls  = db.prepare("SELECT url_id,url_label,url_href FROM url WHERE url_parent_id=? AND url_type='prj' ORDER BY url_order").all(row.prj_id);
    return {
      id:        row.prj_id,
      name:      row.prj_name,
      color:     row.prj_color,
      date:      row.prj_date,
      version:   row.prj_version || 1,
      parentId:  row.prj_parent || null,
      cardIds: items.map(p => p.cad_id),
      urls:      urls.map(u => ({ id: u.url_id, label: u.url_label, href: u.url_href })),
      env:       row.prj_env || '',
      path:      row.prj_path || '',
      finalized: !!row.prj_finalized,
    };
  }

  // nextVersion: find lowest unused version for name+type (excluding excludeId)
  function nextCardVersion(name, type, excludeId = null) {
    const rows = db.prepare("SELECT cad_version FROM card WHERE cad_name=? AND cad_type=?" + (excludeId ? " AND cad_id!=?" : ""))
      .all(...(excludeId ? [name, type, excludeId] : [name, type]));
    const used = new Set(rows.map(r => r.cad_version));
    let v = 1; while (used.has(v)) v++; return v;
  }
  function nextProjectVersion(name, excludeId = null) {
    const rows = db.prepare("SELECT prj_version FROM project WHERE prj_name=?" + (excludeId ? " AND prj_id!=?" : ""))
      .all(...(excludeId ? [name, excludeId] : [name]));
    const used = new Set(rows.map(r => r.prj_version));
    let v = 1; while (used.has(v)) v++; return v;
  }

  // ── Routes ───────────────────────────────────────────────────────────────────
  mark("seed reconciliation");

  const api = express();
  api.use(cors({ origin: "*" }));
  api.use(express.json({ limit: "50mb" }));

  // Prompts
  api.get("/cards", (req, res) => {
    res.json(db.prepare("SELECT * FROM card ORDER BY cad_date DESC").all().map(rowToCard));
  });

  api.post("/cards", (req, res) => {
    const { id, type, name, desc = '', note = '', style = '', lyrics = '', tags = [], favorite = false, date, linkedCards = [], sortNumber = '', mediaPath = '', env } = req.body;
    if (!id || !type || !name) return res.status(400).json({ error: "Missing fields" });
    const targetEnv = env || defaultEnvId;
    if ((type === 'song_l' || type === 'song_e' || type === 'song') && targetEnv === globalEnvId)
      return res.status(400).json({ error: "Songs cannot be added to Global environment" });
    db.transaction(() => {
      const version = nextCardVersion(name, type);
      db.prepare(`INSERT INTO card(cad_id,cad_name,cad_version,cad_type,cad_desc,cad_note,cad_style,cad_lyrics,cad_tags,cad_favorite,cad_date,cad_order,cad_media_path,cad_envid) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?)`)
        .run(id, name, version, type, desc, note, style, lyrics, JSON.stringify(tags), favorite ? 1 : 0, date || Date.now(), toOrderInt(sortNumber), mediaPath, targetEnv);
      const ins = db.prepare("INSERT OR IGNORE INTO card_link (cal_cadid_parent,cal_cadid_child) VALUES (?,?)");
      for (const c of linkedCards) ins.run(id, c);
    })();
    res.status(201).json(rowToCard(db.prepare("SELECT * FROM card WHERE cad_id=?").get(id)));
  });

  // Read the Suno song id from a media file's comment tag ("made with suno; created=…; id=…").
  // The flac's comment is scrubbed at tagging time, so fall back to the same-name sibling
  // (the wav keeps its comment as provenance — same rule as the suno-sync tool).
  async function readSunoId(mediaPath) {
    const mm = await import("music-metadata");
    const tryFile = async (p) => {
      if (!p || !fs.existsSync(p)) return null;
      try {
        const meta = await mm.parseFile(p, { duration: false, skipCovers: true });
        for (const c of (meta.common.comment || [])) {
          const text = typeof c === "string" ? c : (c?.text || "");
          const m = text.match(/made with suno.*?\bid=([0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})/i);
          if (m) return m[1];
        }
      } catch (_) {}
      return null;
    };
    let id = await tryFile(mediaPath);
    if (!id) {
      const base = mediaPath.replace(/\.[^.]+$/, "");
      const ext = path.extname(mediaPath).toLowerCase();
      for (const sib of [".wav", ".flac"].filter(e => e !== ext)) {
        id = await tryFile(base + sib);
        if (id) break;
      }
    }
    return id;
  }

  api.put("/cards/:id", async (req, res) => {
    const { id } = req.params;
    if (!db.prepare("SELECT cad_id FROM card WHERE cad_id=?").get(id)) return res.status(404).json({ error: "Not found" });
    const { type, name, desc, note, style, lyrics, tags, favorite, linkedCards, sortNumber, mediaPath, aiTweaks, lang, env, date } = req.body;
    const prevMediaPath = db.prepare("SELECT cad_media_path FROM card WHERE cad_id=?").get(id).cad_media_path;
    db.transaction(() => {
      const current = db.prepare("SELECT cad_name,cad_type FROM card WHERE cad_id=?").get(id);
      const effectiveName = name ?? current.cad_name;
      const effectiveType = type ?? current.cad_type;
      const newVersion = ((name && name !== current.cad_name) || (type && type !== current.cad_type))
        ? nextCardVersion(effectiveName, effectiveType, id) : null;
      db.prepare(`UPDATE card SET cad_name=COALESCE(?,cad_name),cad_type=COALESCE(?,cad_type),cad_desc=COALESCE(?,cad_desc),cad_note=COALESCE(?,cad_note),cad_style=COALESCE(?,cad_style),cad_lyrics=COALESCE(?,cad_lyrics),cad_tags=COALESCE(?,cad_tags),cad_favorite=COALESCE(?,cad_favorite),cad_order=COALESCE(?,cad_order),cad_media_path=COALESCE(?,cad_media_path),cad_ai_tweaks=COALESCE(?,cad_ai_tweaks),cad_lang=COALESCE(?,cad_lang),cad_envid=COALESCE(?,cad_envid),cad_date=COALESCE(?,cad_date)${newVersion ? ",cad_version=?" : ""} WHERE cad_id=?`)
        .run(name??null, type??null, desc!==undefined?desc:null, note!==undefined?note:null, style??null, lyrics??null, tags!==undefined?JSON.stringify(tags):null, favorite!==undefined?(favorite?1:0):null, sortNumber!==undefined?toOrderInt(sortNumber):null, mediaPath!==undefined?mediaPath:null, aiTweaks!==undefined?JSON.stringify(aiTweaks):null, lang??null, env??null, date??null, ...(newVersion ? [newVersion] : []), id);
      if (linkedCards !== undefined) {
        db.prepare("DELETE FROM card_link WHERE cal_cadid_parent=?").run(id);
        const ins = db.prepare("INSERT OR IGNORE INTO card_link (cal_cadid_parent,cal_cadid_child) VALUES (?,?)");
        for (const c of linkedCards) ins.run(id, c);
      }
    })();
    // Media newly linked → if the file carries Suno provenance, add the Suno song URL.
    if (mediaPath && mediaPath !== prevMediaPath) {
      const sunoId = await readSunoId(mediaPath);
      if (sunoId) {
        const href = `https://suno.com/song/${sunoId}`;
        const exists = db.prepare("SELECT url_id FROM url WHERE url_parent_id=? AND url_type='cad' AND url_href=?").get(id, href);
        if (!exists) {
          const maxOrder = db.prepare("SELECT COALESCE(MAX(url_order),0) AS m FROM url WHERE url_parent_id=? AND url_type='cad'").get(id).m;
          db.prepare("INSERT INTO url (url_id,url_parent_id,url_type,url_label,url_href,url_order) VALUES (?,?,?,?,?,?)")
            .run(crypto.randomUUID(), id, "cad", "Suno", href, maxOrder + 1);
        }
      }
    }
    res.json(rowToCard(db.prepare("SELECT * FROM card WHERE cad_id=?").get(id)));
  });

  // Bulk reorder — assign cad_order = position (1-based) for the given id sequence.
  // Used by the sort view to renumber a project's songs after a drag-to-insert.
  // When renameMedia is set, also rewrite each linked media file's leading "NN-" prefix to
  // match its new order (e.g. 07-Title.flac → 05-Title.flac), keeping the file on disk in sync.
  api.post("/cards/reorder", (req, res) => {
    const { ids, renameMedia } = req.body;
    if (!Array.isArray(ids)) return res.status(400).json({ error: "ids array required" });
    db.transaction(() => {
      const upd = db.prepare("UPDATE card SET cad_order=? WHERE cad_id=?");
      ids.forEach((id, i) => upd.run(i + 1, id));
    })();
    if (renameMedia) {
      const updPath = db.prepare("UPDATE card SET cad_media_path=? WHERE cad_id=?");
      ids.forEach((id, i) => {
        const row = db.prepare("SELECT cad_media_path FROM card WHERE cad_id=?").get(id);
        const mp = row?.cad_media_path;
        if (!mp || !fs.existsSync(mp)) return;
        const dir = path.dirname(mp), ext = path.extname(mp);
        const stripped = path.basename(mp, ext).replace(/^\s*\d+\s*[-_.]?\s*/, "");
        const newPath = path.join(dir, String(i + 1).padStart(2, "0") + "-" + stripped + ext);
        if (newPath === mp || fs.existsSync(newPath)) return; // skip no-op and collisions
        try { fs.renameSync(mp, newPath); updPath.run(newPath, id); } catch { /* leave path as-is on failure */ }
      });
    }
    res.json(ids.map(id => db.prepare("SELECT * FROM card WHERE cad_id=?").get(id)).filter(Boolean).map(rowToCard));
  });

  api.delete("/cards/:id", (req, res) => {
    const result = db.prepare("DELETE FROM card WHERE cad_id=?").run(req.params.id);
    if (result.changes === 0) return res.status(404).json({ error: "Not found" });
    res.json({ deleted: req.params.id });
  });

  // Projects
  api.get("/projects", (req, res) => {
    res.json(db.prepare("SELECT * FROM project ORDER BY prj_date DESC").all().map(rowToProject));
  });

  api.post("/projects", (req, res) => {
    const { id, name, color = "#7c6fff", date, parentId = null, env, path: projPath = '', urls = [], finalized = false } = req.body;
    if (!id || !name) return res.status(400).json({ error: "Missing fields" });
    const version = nextProjectVersion(name);
    const targetEnv = env || defaultEnvId;
    db.prepare("INSERT INTO project (prj_id,prj_name,prj_color,prj_date,prj_version,prj_parent,prj_env,prj_path,prj_finalized) VALUES (?,?,?,?,?,?,?,?,?)").run(id, name, color, date || Date.now(), version, parentId, targetEnv, projPath, finalized ? 1 : 0);
    if (urls.length) {
      const ins = db.prepare("INSERT INTO url (url_id,url_parent_id,url_type,url_label,url_href,url_order) VALUES (?,?,?,?,?,?)");
      urls.forEach((u, i) => ins.run(crypto.randomUUID(), id, 'prj', u.label || '', u.href, i));
    }
    res.status(201).json(rowToProject(db.prepare("SELECT * FROM project WHERE prj_id=?").get(id)));
  });

  api.put("/projects/:id", (req, res) => {
    const { id } = req.params;
    if (!db.prepare("SELECT prj_id FROM project WHERE prj_id=?").get(id)) return res.status(404).json({ error: "Not found" });
    const { name, color, date, parentId, path: projPath, finalized } = req.body;
    const hasParentId = "parentId" in req.body;
    const hasPath = "path" in req.body;
    const hasFinalized = "finalized" in req.body;
    const current = db.prepare("SELECT prj_name FROM project WHERE prj_id=?").get(id);
    const newVersion = (name && name !== current.prj_name) ? nextProjectVersion(name, id) : null;
    db.prepare(`UPDATE project SET prj_name=COALESCE(?,prj_name),prj_color=COALESCE(?,prj_color),prj_date=COALESCE(?,prj_date)${newVersion ? ",prj_version=?" : ""}${hasParentId ? ",prj_parent=?" : ""}${hasPath ? ",prj_path=?" : ""}${hasFinalized ? ",prj_finalized=?" : ""} WHERE prj_id=?`).run(name??null, color??null, date??null, ...(newVersion ? [newVersion] : []), ...(hasParentId ? [parentId] : []), ...(hasPath ? [projPath] : []), ...(hasFinalized ? [finalized ? 1 : 0] : []), id);

    // Auto-relocate orphaned media when project path changes
    let relocated = 0;
    if (hasPath && projPath && fs.existsSync(projPath)) {
      const projectCards = db.prepare("SELECT cad_id, cad_media_path FROM card WHERE cad_prjid=? AND cad_media_path != ''").all(id);
      const orphans = projectCards.filter(p => !fs.existsSync(p.cad_media_path));
      if (orphans.length) {
        // Scan new folder recursively, index by lowercase filename
        const fileIndex = new Map();
        const scanDir = (dir) => {
          try {
            for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
              if (entry.isDirectory()) scanDir(path.join(dir, entry.name));
              else if (!fileIndex.has(entry.name.toLowerCase())) fileIndex.set(entry.name.toLowerCase(), path.join(dir, entry.name));
            }
          } catch {}
        };
        scanDir(projPath);
        const update = db.prepare("UPDATE card SET cad_media_path = ? WHERE cad_id = ?");
        const txn = db.transaction(() => {
          for (const p of orphans) {
            const filename = path.basename(p.cad_media_path).toLowerCase();
            const newPath = fileIndex.get(filename);
            if (newPath) { update.run(newPath, p.cad_id); relocated++; }
          }
        });
        txn();
      }
    }

    const result = rowToProject(db.prepare("SELECT * FROM project WHERE prj_id=?").get(id));
    result.relocated = relocated;
    res.json(result);
  });

  api.delete("/projects/:id", (req, res) => {
    const deleteCards = req.query.cards === "1";
    let cardsDeleted = 0;
    const result = db.transaction(() => {
      if (deleteCards) cardsDeleted = db.prepare("DELETE FROM card WHERE cad_prjid=?").run(req.params.id).changes;
      return db.prepare("DELETE FROM project WHERE prj_id=?").run(req.params.id);
    })();
    if (result.changes === 0) return res.status(404).json({ error: "Not found" });
    res.json({ deleted: req.params.id, cardsDeleted });
  });

  // Move card to project (a card can only belong to one project at a time)
  api.post("/projects/:id/cards", (req, res) => {
    const { cardId } = req.body;
    db.prepare("UPDATE card SET cad_prjid=? WHERE cad_id=?").run(req.params.id, cardId);
    res.json(rowToProject(db.prepare("SELECT * FROM project WHERE prj_id=?").get(req.params.id)));
  });

  // Remove card from project
  api.delete("/projects/:id/cards/:cardId", (req, res) => {
    db.prepare("UPDATE card SET cad_prjid=NULL WHERE cad_id=? AND cad_prjid=?").run(req.params.cardId, req.params.id);
    res.json(rowToProject(db.prepare("SELECT * FROM project WHERE prj_id=?").get(req.params.id)));
  });

  // ── Environments ─────────────────────────────────────────────────────────────
  api.get("/environments", (req, res) => {
    const rows = db.prepare("SELECT * FROM environment ORDER BY env_global DESC, env_order ASC").all();
    res.json(rows.map(r => ({ id: r.env_id, name: r.env_name, order: r.env_order, isGlobal: r.env_global === 1 })));
  });

  api.post("/environments", (req, res) => {
    const { name } = req.body;
    if (!name) return res.status(400).json({ error: "Missing name" });
    const id = crypto.randomUUID();
    const maxOrder = db.prepare("SELECT MAX(env_order) as m FROM environment WHERE env_global=0").get().m ?? 0;
    db.prepare("INSERT INTO environment (env_id,env_name,env_order,env_date,env_global) VALUES (?,?,?,?,0)").run(id, name, maxOrder + 1, Date.now());
    res.status(201).json({ id, name, order: maxOrder + 1, isGlobal: false });
  });

  api.put("/environments/:id", (req, res) => {
    const row = db.prepare("SELECT * FROM environment WHERE env_id=?").get(req.params.id);
    if (!row) return res.status(404).json({ error: "Not found" });
    if (row.env_global === 1) return res.status(403).json({ error: "Cannot rename Global environment" });
    const { name } = req.body;
    if (!name) return res.status(400).json({ error: "Missing name" });
    db.prepare("UPDATE environment SET env_name=? WHERE env_id=?").run(name, req.params.id);
    res.json({ id: row.env_id, name, order: row.env_order, isGlobal: false });
  });

  api.delete("/environments/:id", (req, res) => {
    const row = db.prepare("SELECT * FROM environment WHERE env_id=?").get(req.params.id);
    if (!row) return res.status(404).json({ error: "Not found" });
    if (row.env_global === 1) return res.status(409).json({ error: "Cannot delete Global environment" });
    const cardCount  = db.prepare("SELECT COUNT(*) as n FROM card WHERE cad_envid=?").get(req.params.id).n;
    const projectCount = db.prepare("SELECT COUNT(*) as n FROM project WHERE prj_env=?").get(req.params.id).n;
    if (cardCount > 0 || projectCount > 0) return res.status(409).json({ error: "Environment is not empty" });
    db.prepare("DELETE FROM environment WHERE env_id=?").run(req.params.id);
    res.json({ deleted: req.params.id });
  });

  // ── Move Prompts ──────────────────────────────────────────────────────────────
  api.post("/move-cards", (req, res) => {
    const { cardIds = [], targetEnvId, targetProjectId } = req.body;
    if (!targetEnvId) return res.status(400).json({ error: "Missing targetEnvId" });
    const targetEnvRow = db.prepare("SELECT env_global FROM environment WHERE env_id=?").get(targetEnvId);
    if (!targetEnvRow) return res.status(404).json({ error: "Target environment not found" });

    const movedIds = [], duplicatedIds = [], skippedIds = [];
    let linkedMovedCount = 0;

    db.transaction(() => {
      // Determine if selection contains any songs
      const types = cardIds.map(pid => {
        const r = db.prepare("SELECT cad_type, cad_envid FROM card WHERE cad_id=?").get(pid);
        return r ? r.cad_type : null;
      }).filter(Boolean);
      const hasSongs = types.some(t => t === 'song' || t === 'song_l' || t === 'song_e');

      if (!hasSongs) {
        // Case A: direct move (music/vocal only)
        for (const pid of cardIds) {
          const prow = db.prepare("SELECT cad_style, cad_envid FROM card WHERE cad_id=?").get(pid);
          if (!prow) continue;
          // Merge style into linked songs in source env, then break those links
          const linkedSongs = db.prepare(
            "SELECT cad_id, cad_style FROM card WHERE cad_id IN (SELECT cal_cadid_parent FROM card_link WHERE cal_cadid_child=?) AND cad_envid=?"
          ).all(pid, prow.cad_envid);
          for (const song of linkedSongs) {
            const merged = [song.cad_style?.trim(), prow.cad_style?.trim()].filter(Boolean).join("\n");
            if (merged !== song.cad_style) {
              db.prepare("UPDATE card SET cad_style=? WHERE cad_id=?").run(merged, song.cad_id);
            }
          }
          db.prepare("DELETE FROM card_link WHERE cal_cadid_child=? AND cal_cadid_parent IN (SELECT cad_id FROM card WHERE cad_envid=?)").run(pid, prow.cad_envid);
          db.prepare("UPDATE card SET cad_prjid=NULL WHERE cad_id=? AND cad_prjid IN (SELECT prj_id FROM project WHERE prj_env=?)").run(pid, prow.cad_envid);
          db.prepare("UPDATE card SET cad_envid=? WHERE cad_id=?").run(targetEnvId, pid);
          movedIds.push(pid);
          if (targetProjectId) {
            db.prepare("UPDATE card SET cad_prjid=? WHERE cad_id=?").run(targetProjectId, pid);
          }
        }
      } else {
        // Case B: song move with cascade
        const movedChildIds = new Set(); // track children already moved in this transaction
        const nonSongIds = []; // music/vocal in selection - processed after songs
        for (const pid of cardIds) {
          const prow = db.prepare("SELECT cad_type, cad_envid FROM card WHERE cad_id=?").get(pid);
          if (!prow) continue;
          const isSong = prow.cad_type === 'song' || prow.cad_type === 'song_l' || prow.cad_type === 'song_e';
          if (!isSong) { nonSongIds.push(pid); continue; }

          const sourceEnvId = prow.cad_envid;
          const linked = db.prepare("SELECT cal_cadid_child FROM card_link WHERE cal_cadid_parent=?").all(pid);

          for (const { cal_cadid_child: childId } of linked) {
            if (movedChildIds.has(childId)) continue; // already moved by a previous song in this batch
            const childRow = db.prepare("SELECT cad_envid FROM card WHERE cad_id=?").get(childId);
            if (!childRow) continue;
            if (childRow.cad_envid === globalEnvId) continue; // global - skip
            if (childRow.cad_envid === targetEnvId) continue; // already in target

            // Count songs in source env linking to this child that are NOT in the selection (will stay behind)
            const allLinkingSongIds = db.prepare(
              "SELECT pl.cal_cadid_parent as id FROM card_link pl JOIN card p ON p.cad_id=pl.cal_cadid_parent WHERE pl.cal_cadid_child=? AND p.cad_envid=?"
            ).all(childId, sourceEnvId).map(r => r.id);
            const remainingCount = allLinkingSongIds.filter(id => !cardIds.includes(id)).length;

            if (remainingCount === 0) {
              // All songs that use this child are being moved - move directly
              db.prepare("UPDATE card SET cad_prjid=NULL WHERE cad_id=?").run(childId);
              db.prepare("UPDATE card SET cad_envid=? WHERE cad_id=?").run(targetEnvId, childId);
              movedChildIds.add(childId);
              linkedMovedCount++;
            } else {
              // Some songs remain in source - duplicate for target env
              const childData = db.prepare("SELECT * FROM card WHERE cad_id=?").get(childId);
              const newId = crypto.randomUUID();
              const newVersion = nextCardVersion(childData.cad_name, childData.cad_type);
              db.prepare("INSERT INTO card(cad_id,cad_name,cad_version,cad_type,cad_desc,cad_note,cad_style,cad_lyrics,cad_tags,cad_favorite,cad_date,cad_order,cad_media_path,cad_ai_tweaks,cad_lang,cad_envid) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)")
                .run(newId, childData.cad_name, newVersion, childData.cad_type, childData.cad_desc || '', childData.cad_note || '', childData.cad_style, childData.cad_lyrics, childData.cad_tags, childData.cad_favorite, Date.now(), childData.cad_order, childData.cad_media_path, childData.cad_ai_tweaks, childData.cad_lang, targetEnvId);
              // Copy translations
              const translations = db.prepare("SELECT * FROM card_translation WHERE cat_cadid=?").all(childId);
              for (const tr of translations) {
                db.prepare("INSERT INTO card_translation (cat_id,cat_cadid,cat_name,cat_lang,cat_lyrics,cat_date) VALUES (?,?,?,?,?,?)").run(crypto.randomUUID(), newId, tr.cat_name, tr.cat_lang, tr.cat_lyrics, tr.cat_date);
              }
              // Update link from this song to the new copy
              db.prepare("UPDATE card_link SET cal_cadid_child=? WHERE cal_cadid_parent=? AND cal_cadid_child=?").run(newId, pid, childId);
              duplicatedIds.push(newId);
            }
          }

          // Clear project membership in the source env
          db.prepare("UPDATE card SET cad_prjid=NULL WHERE cad_id=? AND cad_prjid IN (SELECT prj_id FROM project WHERE prj_env=?)").run(pid, sourceEnvId);
          db.prepare("UPDATE card SET cad_envid=? WHERE cad_id=?").run(targetEnvId, pid);
          movedIds.push(pid);
          if (targetProjectId) {
            db.prepare("UPDATE card SET cad_prjid=? WHERE cad_id=?").run(targetProjectId, pid);
          }
        }

        // Move music/vocal from selection not already moved by cascade
        for (const pid of nonSongIds) {
          if (movedChildIds.has(pid)) continue; // already moved as a cascade child
          const prow = db.prepare("SELECT cad_style, cad_envid FROM card WHERE cad_id=?").get(pid);
          if (!prow) continue;
          // Merge style into any remaining linked songs in source env, then break links
          const linkedSongs = db.prepare(
            "SELECT cad_id, cad_style FROM card WHERE cad_id IN (SELECT cal_cadid_parent FROM card_link WHERE cal_cadid_child=?) AND cad_envid=?"
          ).all(pid, prow.cad_envid);
          for (const song of linkedSongs) {
            const merged = [song.cad_style?.trim(), prow.cad_style?.trim()].filter(Boolean).join("\n");
            if (merged !== song.cad_style) db.prepare("UPDATE card SET cad_style=? WHERE cad_id=?").run(merged, song.cad_id);
          }
          db.prepare("DELETE FROM card_link WHERE cal_cadid_child=? AND cal_cadid_parent IN (SELECT cad_id FROM card WHERE cad_envid=?)").run(pid, prow.cad_envid);
          db.prepare("UPDATE card SET cad_prjid=NULL WHERE cad_id=? AND cad_prjid IN (SELECT prj_id FROM project WHERE prj_env=?)").run(pid, prow.cad_envid);
          db.prepare("UPDATE card SET cad_envid=? WHERE cad_id=?").run(targetEnvId, pid);
          movedIds.push(pid);
          if (targetProjectId) {
            db.prepare("UPDATE card SET cad_prjid=? WHERE cad_id=?").run(targetProjectId, pid);
          }
        }
      }
    })();

    res.json({ movedIds, duplicatedIds, skippedIds, linkedMovedCount });
  });

  // ── Move Folder ───────────────────────────────────────────────────────────────
  api.post("/move-folder", (req, res) => {
    const { projectId, targetEnvId } = req.body;
    if (!projectId || !targetEnvId) return res.status(400).json({ error: "Missing fields" });

    const movedProjectIds = [], movedCardIds = [], duplicatedCardIds = [];

    db.transaction(() => {
      // Collect full project subtree
      const subtree = [];
      const queue = [projectId];
      while (queue.length) {
        const pid = queue.shift();
        subtree.push(pid);
        const children = db.prepare("SELECT prj_id FROM project WHERE prj_parent=?").all(pid);
        for (const c of children) queue.push(c.prj_id);
      }

      const sourceEnvId = db.prepare("SELECT prj_env FROM project WHERE prj_id=?").get(projectId)?.prj_env;

      // Collect all card IDs in these projects
      const allCardIds = new Set();
      for (const projId of subtree) {
        const links = db.prepare("SELECT cad_id FROM card WHERE cad_prjid=?").all(projId);
        for (const l of links) allCardIds.add(l.cad_id);
      }

      // Apply cascade for each song
      for (const pid of allCardIds) {
        const prow = db.prepare("SELECT cad_type, cad_envid FROM card WHERE cad_id=?").get(pid);
        if (!prow) continue;
        const isSong = prow.cad_type === 'song' || prow.cad_type === 'song_l' || prow.cad_type === 'song_e';
        if (!isSong) continue;

        const linked = db.prepare("SELECT cal_cadid_child FROM card_link WHERE cal_cadid_parent=?").all(pid);
        for (const { cal_cadid_child: childId } of linked) {
          const childRow = db.prepare("SELECT cad_envid FROM card WHERE cad_id=?").get(childId);
          if (!childRow || childRow.cad_envid === globalEnvId) continue;

          const linkCount = db.prepare(
            "SELECT COUNT(*) as n FROM card_link pl JOIN card p ON p.cad_id=pl.cal_cadid_parent WHERE pl.cal_cadid_child=? AND p.cad_envid=?"
          ).get(childId, sourceEnvId).n;

          if (linkCount === 1) {
            db.prepare("UPDATE card SET cad_envid=? WHERE cad_id=?").run(targetEnvId, childId);
            movedCardIds.push(childId);
          } else {
            const childData = db.prepare("SELECT * FROM card WHERE cad_id=?").get(childId);
            const newId = crypto.randomUUID();
            const newVersion = nextCardVersion(childData.cad_name, childData.cad_type);
            db.prepare("INSERT INTO card(cad_id,cad_name,cad_version,cad_type,cad_style,cad_lyrics,cad_tags,cad_favorite,cad_date,cad_order,cad_media_path,cad_ai_tweaks,cad_lang,cad_envid) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?)")
              .run(newId, childData.cad_name, newVersion, childData.cad_type, childData.cad_style, childData.cad_lyrics, childData.cad_tags, childData.cad_favorite, Date.now(), childData.cad_order, childData.cad_media_path, childData.cad_ai_tweaks, childData.cad_lang, targetEnvId);
            const translations = db.prepare("SELECT * FROM card_translation WHERE cat_cadid=?").all(childId);
            for (const tr of translations) {
              db.prepare("INSERT INTO card_translation (cat_id,cat_cadid,cat_name,cat_lang,cat_lyrics,cat_date) VALUES (?,?,?,?,?,?)").run(crypto.randomUUID(), newId, tr.cat_name, tr.cat_lang, tr.cat_lyrics, tr.cat_date);
            }
            db.prepare("UPDATE card_link SET cal_cadid_child=? WHERE cal_cadid_parent=? AND cal_cadid_child=?").run(newId, pid, childId);
            duplicatedCardIds.push(newId);
          }
        }

        db.prepare("UPDATE card SET cad_envid=? WHERE cad_id=?").run(targetEnvId, pid);
        movedCardIds.push(pid);
      }

      // Move projects - root first, remap parents
      const idMap = {}; // oldId → newId (for merge cases)
      for (const projId of subtree) {
        const proj = db.prepare("SELECT * FROM project WHERE prj_id=?").get(projId);
        const mappedParent = proj.prj_parent ? (idMap[proj.prj_parent] ?? proj.prj_parent) : null;
        // Check for name clash in target env
        const clash = db.prepare("SELECT prj_id FROM project WHERE prj_name=? AND prj_env=? AND prj_id!=?").get(proj.prj_name, targetEnvId, projId);
        if (clash) {
          // Merge: move cards from source project to clash project, then delete source
          db.prepare("UPDATE card SET cad_prjid=? WHERE cad_prjid=?").run(clash.prj_id, projId);
          db.prepare("DELETE FROM project WHERE prj_id=?").run(projId);
          idMap[projId] = clash.prj_id;
        } else {
          db.prepare("UPDATE project SET prj_env=?, prj_parent=? WHERE prj_id=?").run(targetEnvId, mappedParent, projId);
          idMap[projId] = projId;
          movedProjectIds.push(projId);
        }
      }
    })();

    res.json({ movedProjectIds, movedCardIds, duplicatedCardIds });
  });

  // ── Bulk import (Excel) ──────────────────────────────────────────────────────
  api.post("/import", (req, res) => {
    const { cards: importCards = [], projects: importProjects = [], songLinks: importSongLinks = [], songUrls: importSongUrls = [] } = req.body;
    const results = { cardsCreated: 0, cardsUpdated: 0, cardsSkipped: 0, projectsCreated: 0, projectsUpdated: 0, songLinksAdded: 0, songUrlsAdded: 0, skippedAmbiguous: 0 };

    // Resolve card by name+version+type (type optional, falls back to name+version)
    const resolveCard = (name, version, type = null) => {
      if (!name) return null;
      const v = parseInt(version, 10) || 1;
      if (type) {
        const r = db.prepare("SELECT cad_id FROM card WHERE cad_name=? AND cad_version=? AND cad_type=?").get(name, v, type);
        return r ? r.cad_id : null;
      }
      const r = db.prepare("SELECT cad_id FROM card WHERE cad_name=? AND cad_version=?").get(name, v);
      return r ? r.cad_id : null;
    };
    // Resolve project by name+version composite key
    const resolveProject = (name, version) => {
      if (!name) return null;
      const v = parseInt(version, 10) || 1;
      const r = db.prepare("SELECT prj_id FROM project WHERE prj_name=? AND prj_version=?").get(name, v);
      return r ? r.prj_id : null;
    };

    try {
      db.transaction(() => {
        // ── Prompts - match by name+version composite key ────────────────────
        for (const p of importCards) {
          const name    = (p["Card Name"] || p["name"] || "").toString().trim();
          if (!name) continue;
          const version  = parseInt(p["Card Version"] || p["version"] || "1", 10) || 1;
          const type     = (p["Card Type"] || p["type"] || "music").toString().toLowerCase().trim();
          const pstyle   = (p["Style"] || p["style"] || "").toString().trim();
          const plyrcs   = (p["Lyrics"] || p["lyrics"] || "").toString().trim();
          const tagsRaw  = (p["Tags"] || p["tags"] || "").toString();
          const tags     = JSON.stringify(tagsRaw.split(",").map(t => t.trim()).filter(Boolean));
          const fav      = (p["Favorite"] || p["favorite"] || "") === "Yes" ? 1 : 0;

          const existing = db.prepare("SELECT * FROM card WHERE cad_name=? AND cad_version=? AND cad_type=?").get(name, version, type);
          if (existing) {
            const changed = pstyle !== existing.cad_style || plyrcs !== existing.cad_lyrics
                         || JSON.parse(tags).sort().join(",") !== JSON.parse(existing.cad_tags||'[]').sort().join(",")
                         || fav !== existing.cad_favorite;
            if (changed) {
              db.prepare("UPDATE card SET cad_style=?,cad_lyrics=?,cad_tags=?,cad_favorite=? WHERE cad_id=?")
                .run(pstyle, plyrcs, tags, fav, existing.cad_id);
              results.cardsUpdated++;
            } else { results.cardsSkipped++; }
          } else {
            // Use provided version; if that slot is somehow taken, find next available
            const safeVersion = db.prepare("SELECT cad_id FROM card WHERE cad_name=? AND cad_version=? AND cad_type=?").get(name, version, type) ? nextCardVersion(name, type) : version;
            db.prepare("INSERT INTO card(cad_id,cad_name,cad_version,cad_type,cad_style,cad_lyrics,cad_tags,cad_favorite,cad_date) VALUES (?,?,?,?,?,?,?,?,?)")
              .run(crypto.randomUUID(), name, safeVersion, type, pstyle, plyrcs, tags, fav, Date.now());
            results.cardsCreated++;
          }
        }

        // ── Projects - match by name+version composite key ───────────────────
        const seenProjects = {};
        const projectParents = {}; // seenKey -> { parentName, parentVersion }
        for (const row of importProjects) {
          const projName    = (row["Project Name"] || row["name"] || "").toString().trim();
          const projVersion = parseInt(row["Project Version"] || row["version"] || "1", 10) || 1;
          const cardName  = (row["Card Name"] || "").toString().trim();
          const cardVer   = parseInt(row["Card Version"] || "1", 10) || 1;
          const cardType  = (row["Card Type"] || "").toString().trim().toLowerCase() || null;
          const parentFolderName = (row["Parent Folder"] || "").toString().trim();
          const parentFolderVer  = parseInt(row["Parent Folder Version"] || "1", 10) || 1;
          if (!projName) continue;

          const seenKey = projName + "@v" + projVersion;
          if (!seenProjects[seenKey]) {
            let resolvedProjId = resolveProject(projName, projVersion);
            if (!resolvedProjId) {
              const newId = crypto.randomUUID();
              const date = row["Created"] ? (new Date(row["Created"]).getTime() || Date.now()) : Date.now();
              db.prepare("INSERT INTO project (prj_id,prj_name,prj_version,prj_color,prj_date) VALUES (?,?,?,?,?)").run(newId, projName, projVersion, "#7c6fff", date);
              resolvedProjId = newId;
              results.projectsCreated++;
            }
            const already = db.prepare("SELECT cad_id FROM card WHERE cad_prjid=?").all(resolvedProjId);
            seenProjects[seenKey] = { id: resolvedProjId, isNew: results.projectsCreated > 0, linkedBefore: new Set(already.map(r => r.cad_id)), addedCount: 0 };
          }
          if (parentFolderName) projectParents[seenKey] = { parentName: parentFolderName, parentVersion: parentFolderVer };

          const proj = seenProjects[seenKey];
          if (cardName) {
            const resolvedCardId = resolveCard(cardName, cardVer, cardType);
            if (resolvedCardId && !proj.linkedBefore.has(resolvedCardId)) {
              db.prepare("UPDATE card SET cad_prjid=? WHERE cad_id=?").run(proj.id, resolvedCardId);
              proj.addedCount++;
            } else if (!resolvedCardId) {
              results.skippedAmbiguous++;
            }
          }
        }
        for (const proj of Object.values(seenProjects)) {
          if (!proj.isNew && proj.addedCount > 0) results.projectsUpdated++;
        }
        // Second pass: set parent folder IDs
        for (const [seenKey, { parentName, parentVersion }] of Object.entries(projectParents)) {
          const proj = seenProjects[seenKey];
          const parentId = resolveProject(parentName, parentVersion);
          if (parentId && proj) db.prepare("UPDATE project SET prj_parent=? WHERE prj_id=?").run(parentId, proj.id);
        }

        // ── Song Links - match by name+version+type ──────────────────────────
        for (const row of importSongLinks) {
          const songTitle  = (row["Song Name"]      || "").toString().trim();
          const songVer    = parseInt(row["Song Version"]   || "1", 10) || 1;
          const linkedTitle= (row["Card Name"]    || "").toString().trim();
          const linkedVer  = parseInt(row["Card Version"] || "1", 10) || 1;
          const linkedType = (row["Card Type"]    || "").toString().trim().toLowerCase() || null;
          if (!songTitle || !linkedTitle) continue;

          const resolvedSong   = resolveCard(songTitle, songVer, "song");
          const resolvedLinked = resolveCard(linkedTitle, linkedVer, linkedType);
          if (!resolvedSong || !resolvedLinked) { results.skippedAmbiguous++; continue; }

          const exists = db.prepare("SELECT 1 FROM card_link WHERE cal_cadid_parent=? AND cal_cadid_child=?").get(resolvedSong, resolvedLinked);
          if (!exists) {
            db.prepare("INSERT OR IGNORE INTO card_link (cal_cadid_parent,cal_cadid_child) VALUES (?,?)").run(resolvedSong, resolvedLinked);
            results.songLinksAdded++;
          }
        }

        // ── Song URLs - match by name+version ────────────────────────────────
        for (const row of importSongUrls) {
          const songTitle = (row["Song Name"]    || "").toString().trim();
          const songVer   = parseInt(row["Song Version"] || "1", 10) || 1;
          const label     = (row["Label"]        || "").toString().trim();
          const href      = (row["URL"]          || "").toString().trim();
          if (!songTitle || !href) continue;

          const resolvedSong = resolveCard(songTitle, songVer);
          if (!resolvedSong) { results.skippedAmbiguous++; continue; }

          const exists = db.prepare("SELECT 1 FROM url WHERE url_parent_id=? AND url_href=?").get(resolvedSong, href);
          if (!exists) {
            const order = db.prepare("SELECT COUNT(*) as n FROM url WHERE url_parent_id=?").get(resolvedSong).n;
            db.prepare("INSERT INTO url (url_id,url_parent_id,url_type,url_label,url_href,url_order) VALUES (?,?,?,?,?,?)")
              .run(crypto.randomUUID(), resolvedSong, 'cad', label, href, order);
            results.songUrlsAdded++;
          }
        }
      })();

      const allCards  = db.prepare("SELECT * FROM card ORDER BY cad_date DESC").all().map(rowToCard);
      const allProjects = db.prepare("SELECT * FROM project ORDER BY prj_date DESC").all().map(rowToProject);
      res.json({ results, cards: allCards, projects: allProjects });
    } catch (err) {
      console.error("Import error:", err.message);
      res.status(500).json({ error: err.message });
    }
  });


  // ── JSON import (full snapshot, ID-based) ─────────────────────────────────────
  api.post("/import-json", (req, res) => {
    const isBackup = req.body.type === "backup";
    const targetEnvId = req.body.targetEnvId || defaultEnvId;
    const { cards: importCards = [], projects: importProjects = [] } = req.body;

    // Backup path: upsert environments first
    // envIdMap remaps backup UUIDs → DB UUIDs so card/project env FKs resolve correctly
    // even when the backup was made on a different machine (different seeded UUIDs)
    const envIdMap = {};
    if (isBackup && Array.isArray(req.body.environments)) {
      for (const e of req.body.environments) {
        if (!e.id || !e.name) continue;
        const existing = db.prepare("SELECT env_id FROM environment WHERE env_id=?").get(e.id);
        if (existing) {
          envIdMap[e.id] = e.id;
          if (!e.isGlobal) db.prepare("UPDATE environment SET env_name=?,env_order=? WHERE env_id=?").run(e.name, e.order ?? 0, e.id);
        } else {
          // ⚠ CLAUDE: (Global) is unique - always remap to the existing global env rather than
          // inserting a duplicate. Regular envs from other users are inserted as-is.
          if (e.isGlobal) {
            const existingGlobal = db.prepare("SELECT env_id FROM environment WHERE env_global=1").get();
            if (existingGlobal) {
              envIdMap[e.id] = existingGlobal.env_id;
            } else {
              db.prepare("INSERT INTO environment (env_id,env_name,env_order,env_date,env_global) VALUES (?,?,?,?,1)").run(e.id, e.name, e.order ?? 0, Date.now());
              envIdMap[e.id] = e.id;
            }
          } else {
            db.prepare("INSERT INTO environment (env_id,env_name,env_order,env_date,env_global) VALUES (?,?,?,?,0)").run(e.id, `${e.name} (Restored)`, e.order ?? 0, Date.now());
            envIdMap[e.id] = e.id;
          }
        }
      }
    }
    const UUID_RE      = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
    const HEX_RE       = /^#[0-9a-fA-F]{6}$/;
    const VALID_TYPES  = new Set(['music', 'vocal', 'song', 'song_l', 'song_e', 'note']);
    const results = { cardsCreated: 0, cardsUpdated: 0, cardsSkipped: 0, projectsCreated: 0, projectsUpdated: 0, songLinksAdded: 0, songUrlsAdded: 0, validationErrors: [] };
    try {
      db.transaction(() => {
        // Prompts
        for (const p of importCards) {
          if (p.type === "song_l" || p.type === "song_e") p.type = "song"; // backward compat
          if (!p.id || !UUID_RE.test(p.id))        { results.validationErrors.push(`Card skipped: invalid id "${p.id}"`); continue; }
          if (typeof p.name !== 'string' || !p.name) { results.validationErrors.push(`Card skipped: name must be a non-empty string (id: ${p.id})`); continue; }
          if (typeof p.style !== 'string')           { results.validationErrors.push(`Card skipped: style must be a string (id: ${p.id})`); continue; }
          if (!VALID_TYPES.has(p.type))              { results.validationErrors.push(`Card skipped: invalid type "${p.type}" (id: ${p.id})`); continue; }
          const tags = JSON.stringify(Array.isArray(p.tags) ? p.tags : []);
          const fav  = p.favorite ? 1 : 0;
          // ⚠ CLAUDE: non-backup lookup is env-scoped - a UUID from a different env must not match
          const existing = isBackup
            ? db.prepare("SELECT * FROM card WHERE cad_id=?").get(p.id)
            : db.prepare("SELECT * FROM card WHERE cad_id=? AND cad_envid=?").get(p.id, targetEnvId);
          let effectiveCardId = p.id; // may be replaced with new UUID for non-backup inserts
          if (existing) {
            const changed = p.type !== existing.cad_type || p.name !== existing.cad_name
                         || p.style !== existing.cad_style || (p.lyrics || '') !== existing.cad_lyrics
                         || tags !== existing.cad_tags || fav !== existing.cad_favorite
                         || toOrderInt(p.order) !== existing.cad_order;
            const cardEnv = isBackup ? (p.env ? (envIdMap[p.env] ?? p.env) : defaultEnvId) : targetEnvId;
            if (changed) {
              db.prepare("UPDATE card SET cad_type=?,cad_name=?,cad_desc=COALESCE(?,cad_desc),cad_note=COALESCE(?,cad_note),cad_style=?,cad_lyrics=?,cad_tags=?,cad_favorite=?,cad_media_path=?,cad_order=?,cad_lang=COALESCE(?,cad_lang),cad_ai_tweaks=COALESCE(?,cad_ai_tweaks),cad_prjid=COALESCE(?,cad_prjid),cad_envid=? WHERE cad_id=?")
                .run(p.type, p.name, p.desc || null, p.note || null, p.style, p.lyrics || '', tags, fav, p.media || '', toOrderInt(p.order), p.lang || null, p.aiTweaks ? JSON.stringify(p.aiTweaks) : null, p.project || null, cardEnv, p.id);
              results.cardsUpdated++;
            } else { results.cardsSkipped++; }
          } else {
            const cardEnv = isBackup ? (p.env ? (envIdMap[p.env] ?? p.env) : defaultEnvId) : targetEnvId;
            // ⚠ CLAUDE: non-backup insert uses a fresh UUID - the original UUID may belong to a different env
            effectiveCardId = isBackup ? p.id : crypto.randomUUID();
            db.prepare("INSERT INTO card(cad_id,cad_name,cad_version,cad_type,cad_desc,cad_note,cad_style,cad_lyrics,cad_tags,cad_favorite,cad_date,cad_media_path,cad_order,cad_lang,cad_ai_tweaks,cad_prjid,cad_envid) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)")
              .run(effectiveCardId, p.name, p.version || 1, p.type, p.desc || '', p.note || '', p.style, p.lyrics || '', tags, fav, p.date || Date.now(), p.media || '', toOrderInt(p.order), p.lang || '', p.aiTweaks ? JSON.stringify(p.aiTweaks) : '{}', p.project || null, cardEnv);
            results.cardsCreated++;
          }
          // Translations
          if (Array.isArray(p.translations)) {
            for (const tr of p.translations) {
              if (!tr.lang) continue;
              db.prepare("INSERT OR REPLACE INTO card_translation (cat_id,cat_cadid,cat_name,cat_lang,cat_lyrics,cat_date) VALUES (COALESCE((SELECT cat_id FROM card_translation WHERE cat_cadid=? AND cat_lang=?),?),?,?,?,?,?)")
                .run(effectiveCardId, tr.lang, crypto.randomUUID(), effectiveCardId, tr.name || '', tr.lang, tr.content || '', Date.now());
            }
          }
          // Linked cards
          if (Array.isArray(p.cards)) {
            db.prepare("DELETE FROM card_link WHERE cal_cadid_parent=?").run(effectiveCardId);
            const ins = db.prepare("INSERT OR IGNORE INTO card_link (cal_cadid_parent,cal_cadid_child) VALUES (?,?)");
            for (const cid of p.cards) {
              if (db.prepare("SELECT 1 FROM card WHERE cad_id=?").get(cid)) {
                ins.run(effectiveCardId, cid);
                results.songLinksAdded++;
              }
            }
          }
          // URLs
          if (Array.isArray(p.urls)) {
            db.prepare("DELETE FROM url WHERE url_parent_id=? AND url_type='cad'").run(effectiveCardId);
            p.urls.forEach((u, i) => {
              db.prepare("INSERT INTO url (url_id,url_parent_id,url_type,url_label,url_href,url_order) VALUES (?,?,?,?,?,?)")
                .run(crypto.randomUUID(), effectiveCardId, 'cad', u.label || "", u.href, i);
              results.songUrlsAdded++;
            });
          }
        }
        // Track which cards have been linked in this import run - a card can only appear in one project
        const importLinkedCards = new Map(); // cardId → projectName
        // Projects
        for (const proj of importProjects) {
          if (!proj.id || !UUID_RE.test(proj.id))         { results.validationErrors.push(`Project skipped: invalid id "${proj.id}"`); continue; }
          if (typeof proj.name !== 'string' || !proj.name) { results.validationErrors.push(`Project skipped: name must be a non-empty string (id: ${proj.id})`); continue; }
          if (proj.color && !HEX_RE.test(proj.color))     { results.validationErrors.push(`Project "${proj.name}": invalid color "${proj.color}", using default`); proj.color = '#7c6fff'; }
          // ⚠ CLAUDE: non-backup lookup is env-scoped - a UUID from a different env must not match
          const existing = isBackup
            ? db.prepare("SELECT prj_id FROM project WHERE prj_id=?").get(proj.id)
            : db.prepare("SELECT prj_id FROM project WHERE prj_id=? AND prj_env=?").get(proj.id, targetEnvId);
          let effectiveProjId = proj.id; // may be replaced with new UUID for non-backup inserts
          if (existing) {
            const projEnv = isBackup ? (proj.env ? (envIdMap[proj.env] ?? proj.env) : defaultEnvId) : targetEnvId;
            db.prepare("UPDATE project SET prj_name=?,prj_color=?,prj_parent=?,prj_env=?,prj_path=?,prj_finalized=? WHERE prj_id=?").run(proj.name, proj.color || "#7c6fff", proj.parent || null, projEnv, proj.path || '', proj.finalized ? 1 : 0, proj.id);
            results.projectsUpdated++;
          } else {
            const projEnv = isBackup ? (proj.env ? (envIdMap[proj.env] ?? proj.env) : defaultEnvId) : targetEnvId;
            // ⚠ CLAUDE: non-backup insert uses a fresh UUID - the original UUID may belong to a different env
            effectiveProjId = isBackup ? proj.id : crypto.randomUUID();
            db.prepare("INSERT INTO project (prj_id,prj_name,prj_color,prj_date,prj_version,prj_parent,prj_env,prj_path,prj_finalized) VALUES (?,?,?,?,?,?,?,?,?)").run(effectiveProjId, proj.name, proj.color || "#7c6fff", proj.date || Date.now(), proj.version || 1, proj.parent || null, projEnv, proj.path || '', proj.finalized ? 1 : 0);
            results.projectsCreated++;
          }
          // ⚠ CLAUDE: project membership is stored on the card (cad_prjid), not in a link table.
          // New format: each card carries p.project = projectId. Old format: project carries proj.cards array.
          // Both are handled: card pass (above) sets cad_prjid from p.project; this pass handles old format.
          if (Array.isArray(proj.cards)) {
            // Clear existing membership for cards in this project (handles old format re-import)
            db.prepare("UPDATE card SET cad_prjid=NULL WHERE cad_prjid=?").run(effectiveProjId);
            proj.cards.forEach((pid) => {
              // ⚠ CLAUDE: a card can only belong to one project - reject if already claimed in this import
              if (importLinkedCards.has(pid)) {
                const pRow = db.prepare("SELECT cad_name FROM card WHERE cad_id=?").get(pid);
                const pName = pRow ? pRow.cad_name : pid;
                results.validationErrors.push(`Card "${pName}" skipped in project "${proj.name}": already linked to project "${importLinkedCards.get(pid)}" in this file`);
                return;
              }
              if (db.prepare("SELECT 1 FROM card WHERE cad_id=?").get(pid)) {
                db.prepare("UPDATE card SET cad_prjid=? WHERE cad_id=?").run(effectiveProjId, pid);
                importLinkedCards.set(pid, proj.name);
              }
            });
          }
          // Import project URLs
          if (Array.isArray(proj.urls)) {
            db.prepare("DELETE FROM url WHERE url_parent_id=? AND url_type='prj'").run(effectiveProjId);
            proj.urls.forEach((u, i) => {
              db.prepare("INSERT INTO url (url_id,url_parent_id,url_type,url_label,url_href,url_order) VALUES (?,?,?,?,?,?)")
                .run(crypto.randomUUID(), effectiveProjId, 'prj', u.label || "", u.href, i);
            });
          }
        }
      })();
      // AI presets (custom only - skip defaults, they're seeded from JSON)
      for (const p of (req.body.aiPresets || [])) {
        if (!p.id || !p.task || !p.block || !p.name || p.content === undefined) continue;
        const existing = db.prepare("SELECT abp_default FROM ai_block_preset WHERE abp_id=?").get(p.id);
        if (existing) {
          if (existing.abp_default === 1) continue; // never overwrite defaults
          db.prepare("UPDATE ai_block_preset SET abp_name=?,abp_content=? WHERE abp_id=?").run(p.name, p.content, p.id);
        } else {
          const ord = db.prepare("SELECT COUNT(*) as n FROM ai_block_preset WHERE abp_task=? AND abp_block=?").get(p.task, p.block).n;
          db.prepare("INSERT INTO ai_block_preset (abp_id,abp_task,abp_block,abp_name,abp_content,abp_default,abp_order) VALUES (?,?,?,?,?,0,?)").run(p.id, p.task, p.block, p.name, p.content, ord);
        }
      }
      // AI templates (custom only)
      for (const t of (req.body.aiTemplates || [])) {
        if (!t.id || !t.task || !t.name) continue;
        const existing = db.prepare("SELECT atp_default FROM ai_template WHERE atp_id=?").get(t.id);
        if (existing) {
          if (existing.atp_default === 1) continue;
          db.prepare("UPDATE ai_template SET atp_name=? WHERE atp_id=?").run(t.name, t.id);
        } else {
          const ord = db.prepare("SELECT COUNT(*) as n FROM ai_template WHERE atp_task=?").get(t.task).n;
          db.prepare("INSERT INTO ai_template (atp_id,atp_task,atp_name,atp_default,atp_order) VALUES (?,?,?,0,?)").run(t.id, t.task, t.name, ord);
        }
        for (const b of (t.blocks || [])) {
          const eb = db.prepare("SELECT atb_id FROM ai_template_block WHERE atb_atpid=? AND atb_block=?").get(t.id, b.block);
          if (eb) {
            db.prepare("UPDATE ai_template_block SET atb_abpid=?,atb_enabled=?,atb_name=COALESCE(?,atb_name),atb_prefix=COALESCE(?,atb_prefix) WHERE atb_id=?").run(b.presetId || null, b.enabled ? 1 : 0, b.name || null, b.prefix || null, eb.atb_id);
          } else {
            db.prepare("INSERT INTO ai_template_block (atb_id,atb_atpid,atb_block,atb_abpid,atb_enabled,atb_name,atb_prefix) VALUES (?,?,?,?,?,?,?)").run(crypto.randomUUID(), t.id, b.block, b.presetId || null, b.enabled ? 1 : 0, b.name || b.block, b.prefix || b.block.toUpperCase());
          }
        }
      }
      const allEnvs = db.prepare("SELECT * FROM environment ORDER BY env_global DESC, env_order ASC").all().map(r => ({ id: r.env_id, name: r.env_name, order: r.env_order, isGlobal: r.env_global === 1 }));
      res.json({ results, cards: db.prepare("SELECT * FROM card ORDER BY cad_date DESC").all().map(rowToCard), projects: db.prepare("SELECT * FROM project ORDER BY prj_date DESC").all().map(rowToProject), environments: allEnvs });
    } catch(err) {
      console.error("JSON import error:", err.message);
      res.status(500).json({ error: err.message });
    }
  });

  // ⚠ CLAUDE: mirrors the auto-label mapping in App.jsx's handleAddUrl — keep the two in step.
  // The LABEL, not the href, is what getSongState matches against the AI/music site lists, so a
  // mislabelled link leaves an imported song sitting at *undefined* with a working URL on it.
  function songUrlLabel(href) {
    const h = String(href || "");
    if (h.includes("mozartai.com")) return "mozartai";
    if (h.includes("producer.ai"))  return "producer";
    if (h.includes("suno.com"))     return "suno";
    if (h.includes("tunee.ai"))     return "tunee";
    if (h.includes("musicfy.club")) return "musicfy";
    try { return new URL(h).hostname.replace("www.", "").replace(/\.(com|ai)$/, ""); } catch { return "Link"; }
  }

  // ── .amlp LP import ──────────────────────────────────────────────────────────
  // An .amlp file (authored by the /suno skill) is one LP: header + N track
  // prompts. It maps 1:1 onto ampl — the LP becomes a project, each track a
  // `song` card (style + embedded lyrics). Validation failures are returned as
  // 200 { error } so the renderer can show a message without apiFetch throwing.
  // The .amlp LANG header is prose ("French (17th-century court idiom)"); we
  // best-effort resolve the leading language name to a song-language code.
  function sunoLangToCode(raw) {
    const s = String(raw || '').trim().toLowerCase();
    if (!s) return '';
    let bestCode = '', bestLen = 0;
    for (const l of SONG_LANGUAGES) {
      const n = l.name.toLowerCase();
      // Match the longest known language name the string starts with, at a word boundary.
      if (s.startsWith(n) && (s.length === n.length || /^[\s,(]/.test(s.slice(n.length))) && n.length > bestLen) {
        bestLen = n.length; bestCode = l.code;
      }
    }
    return bestCode;
  }

  api.post("/import-amlp", (req, res) => {
    const targetEnvId = req.body.targetEnvId || defaultEnvId;
    const text = req.body.text;
    if (typeof text !== 'string' || !text.trim()) return res.json({ error: "empty" });
    // Songs cannot live in the Global environment (same rule as POST /cards).
    if (targetEnvId === globalEnvId) return res.json({ error: "globalEnv" });

    let parsed;
    try { parsed = parseAmlp(text); }
    catch (e) { console.error(".amlp parse error:", e.message); return res.json({ error: "parse" }); }
    const { project: header, tracks } = parsed;
    if (!tracks.length) return res.json({ error: "noTracks" });

    // The tracks land in the project the user is currently in, when there is one — the
    // renderer passes it as targetProjectId. With no project open (or an id that no longer
    // exists) the LP brings its own: a new project named after the header TITLE.
    const openProject = req.body.targetProjectId
      ? db.prepare("SELECT prj_id, prj_name, prj_env FROM project WHERE prj_id=?").get(req.body.targetProjectId)
      : null;
    const projId   = openProject ? openProject.prj_id : crypto.randomUUID();
    const projName = openProject ? openProject.prj_name : (header.title || "Imported LP");
    // A project carries its own environment — importing into it must not scatter the cards
    // into a different one just because the sidebar selection and the env picker disagree.
    const cardEnvId = openProject ? openProject.prj_env : targetEnvId;
    if (cardEnvId === globalEnvId) return res.json({ error: "globalEnv" });
    const langCode = sunoLangToCode(header.lang);
    // .amlp DATE is "YYYY-MM-DD"; fall back to now if absent/unparseable.
    const parsedDate = header.date ? Date.parse(header.date) : NaN;
    const baseDate   = Number.isNaN(parsedDate) ? Date.now() : parsedDate;
    const results = { projectName: projName, cardsCreated: 0, tracks: tracks.length };

    try {
      db.transaction(() => {
        if (!openProject) {
          const projVersion = nextProjectVersion(projName);
          db.prepare("INSERT INTO project (prj_id,prj_name,prj_color,prj_date,prj_version,prj_parent,prj_env,prj_path) VALUES (?,?,?,?,?,?,?,?)")
            .run(projId, projName, "#7c6fff", baseDate, projVersion, null, cardEnvId, '');
        }
        const ins = db.prepare("INSERT INTO card(cad_id,cad_envid,cad_prjid,cad_name,cad_version,cad_type,cad_order,cad_desc,cad_style,cad_lang,cad_lyrics,cad_ai_tweaks,cad_media_path,cad_note,cad_tags,cad_favorite,cad_date) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)");
        const insUrl = db.prepare("INSERT INTO url (url_id,url_parent_id,url_type,url_label,url_href,url_order) VALUES (?,?,?,?,?,?)");
        for (const t of tracks) {
          const cardId  = crypto.randomUUID();
          const name    = t.title || `Track ${String(t.num).padStart(2, '0')}`;
          const version = nextCardVersion(name, 'song');
          ins.run(cardId, cardEnvId, projId, name, version, 'song', toOrderInt(t.num), t.description || '', t.style || '', langCode, t.lyrics || '', '{}', '', '', '[]', 0, baseDate);
          // A track's URL (written by the Suno Bridge extension when it saves the song) becomes
          // the card's link — which is also what turns the song's colour state to *created*.
          const href = (t.url || '').trim();
          if (/^https?:\/\//i.test(href)) insUrl.run(crypto.randomUUID(), cardId, "cad", songUrlLabel(href), href, 1);
          results.cardsCreated++;
        }
      })();
    } catch (err) {
      console.error(".amlp import error:", err.message);
      return res.status(500).json({ error: err.message });
    }

    res.json({
      results,
      cards:    db.prepare("SELECT * FROM card ORDER BY cad_date DESC").all().map(rowToCard),
      projects: db.prepare("SELECT * FROM project ORDER BY prj_date DESC").all().map(rowToProject),
    });
  });

  // ── Gemini proxy ─────────────────────────────────────────────────────────────
  api.post("/gemini/generate", (req, res) => {
    const { request, type, apiKey, limit, maxTokens } = req.body;
    if (!apiKey) return res.status(401).json({ error: "No API key provided. Add your Gemini key in Settings." });

    const charLimit = limit || (type === "vocal_style" ? 300 : type === "music_style" ? 700 : 1000);
    const defaultTokens = type === "lyrics" ? 4096 : 2048;

    const body = JSON.stringify({
      contents: [{ parts: [{ text: request }] }],
      generationConfig: {
        temperature: 0.95,
        maxOutputTokens: maxTokens || defaultTokens,
        ...(type === "translate" ? { thinkingConfig: { thinkingBudget: 0 } } : {})
      }
    });

    const options = {
      hostname: "generativelanguage.googleapis.com",
      path: `/v1beta/models/gemini-2.5-flash:generateContent?key=${apiKey}`,
      method: "POST",
      headers: { "Content-Type": "application/json" }
    };

    const proxyReq = https.request(options, (proxyRes) => {
      proxyRes.setEncoding("utf8");
      let data = "";
      proxyRes.on("data", chunk => data += chunk);
      proxyRes.on("end", () => {
        try {
          const json = JSON.parse(data);
          if (json.error) return res.status(400).json({ error: json.error.message });
          let raw = json.candidates?.[0]?.content?.parts?.[0]?.text || "";

          if (req.body.raw) return res.json({ text: raw });

          if (type === "translate") {
            const finishReason = json.candidates?.[0]?.finishReason;
            if (!raw && finishReason) console.error("Gemini translate stopped:", finishReason, JSON.stringify(json).slice(0, 300));
            return res.json({ text: raw.trim() });
          }

          const sugMatch     = raw.match(/NAME:\s*\n([\s\S]*?)\nCONTENT:/);
          const contentMatch = raw.match(/CONTENT:\s*\n([\s\S]*)/);
          const suggestions  = sugMatch
            ? sugMatch[1].split("\n").map(l => l.replace(/^\d+\.\s*/, "").trim()).filter(Boolean).slice(0, 5)
            : [];
          let text = contentMatch ? contentMatch[1].trim() : raw.trim();
          if (charLimit && text.length > charLimit) {
            text = text.slice(0, charLimit).replace(/\s+\S*$/, "").trimEnd();
          }
          res.json({ text, nameSuggestions: suggestions });
        } catch(e) { res.status(500).json({ error: "Parse error" }); }
      });
    });
    proxyReq.on("error", e => res.status(500).json({ error: e.message }));
    proxyReq.write(body);
    proxyReq.end();
  });


  // ── AI Presets ────────────────────────────────────────────────────────────────
  api.get("/ai-presets", (req, res) => {
    const { task, block } = req.query;
    let query = "SELECT * FROM ai_block_preset WHERE 1=1";
    const params = [];
    if (task)  { query += " AND abp_task=?";  params.push(task); }
    if (block) { query += " AND abp_block=?"; params.push(block); }
    query += " ORDER BY abp_order";
    const rows = db.prepare(query).all(...params);
    res.json(rows.map(r => ({ id: r.abp_id, task: r.abp_task, block: r.abp_block, name: r.abp_name, content: r.abp_content, isDefault: r.abp_default === 1, isDefaultPick: r.abp_default_pick === 1 })));
  });

  api.post("/ai-presets", (req, res) => {
    const { task, block, name, content } = req.body;
    if (!task || !block || !name || content === undefined) return res.status(400).json({ error: "Missing fields" });
    const id = crypto.randomUUID();
    const order = db.prepare("SELECT COUNT(*) as n FROM ai_block_preset WHERE abp_task=? AND abp_block=?").get(task, block).n;
    db.prepare("INSERT INTO ai_block_preset (abp_id,abp_task,abp_block,abp_name,abp_content,abp_default,abp_order) VALUES (?,?,?,?,?,0,?)").run(id, task, block, name, content, order);
    const row = db.prepare("SELECT * FROM ai_block_preset WHERE abp_id=?").get(id);
    res.status(201).json({ id: row.abp_id, task: row.abp_task, block: row.abp_block, name: row.abp_name, content: row.abp_content, isDefault: false });
  });

  api.put("/ai-presets/:id", (req, res) => {
    const row = db.prepare("SELECT * FROM ai_block_preset WHERE abp_id=?").get(req.params.id);
    if (!row) return res.status(404).json({ error: "Not found" });
    if (row.abp_default === 1) return res.status(403).json({ error: "Cannot edit a default preset" });
    const { name, content } = req.body;
    db.prepare("UPDATE ai_block_preset SET abp_name=COALESCE(?,abp_name),abp_content=COALESCE(?,abp_content) WHERE abp_id=?").run(name ?? null, content ?? null, req.params.id);
    const updated = db.prepare("SELECT * FROM ai_block_preset WHERE abp_id=?").get(req.params.id);
    res.json({ id: updated.abp_id, task: updated.abp_task, block: updated.abp_block, name: updated.abp_name, content: updated.abp_content, isDefault: updated.abp_default === 1 });
  });

  api.delete("/ai-presets/:id", (req, res) => {
    const row = db.prepare("SELECT * FROM ai_block_preset WHERE abp_id=?").get(req.params.id);
    if (!row) return res.status(404).json({ error: "Not found" });
    if (row.abp_default === 1) return res.status(403).json({ error: "Cannot delete a default preset" });
    db.transaction(() => {
      db.prepare("UPDATE ai_template_block SET atb_abpid=NULL WHERE atb_abpid=?").run(req.params.id);
      db.prepare("DELETE FROM ai_block_preset WHERE abp_id=?").run(req.params.id);
    })();
    res.json({ deleted: req.params.id });
  });

  // ── AI Templates ──────────────────────────────────────────────────────────────
  function rowToTemplate(row) {
    const blocks = db.prepare("SELECT * FROM ai_template_block WHERE atb_atpid=?").all(row.atp_id);
    return {
      id:        row.atp_id,
      task:      row.atp_task,
      name:      row.atp_name,
      isDefault: row.atp_default === 1,
      blocks:    blocks.map(b => ({ id: b.atb_id, block: b.atb_block, presetId: b.atb_abpid, enabled: b.atb_enabled === 1, name: b.atb_name || b.atb_block, prefix: b.atb_prefix || b.atb_block.toUpperCase(), hidden: blockDefs[b.atb_block]?.hidden || false })),
    };
  }

  api.get("/ai-templates", (req, res) => {
    const { task } = req.query;
    let query = "SELECT * FROM ai_template";
    const params = [];
    if (task) { query += " WHERE atp_task=?"; params.push(task); }
    query += " ORDER BY atp_order";
    res.json(db.prepare(query).all(...params).map(rowToTemplate));
  });

  api.post("/ai-templates", (req, res) => {
    const { task, name, blocks = [] } = req.body;
    if (!task || !name) return res.status(400).json({ error: "Missing fields" });
    const id = crypto.randomUUID();
    const order = db.prepare("SELECT COUNT(*) as n FROM ai_template WHERE atp_task=?").get(task).n;
    db.transaction(() => {
      db.prepare("INSERT INTO ai_template (atp_id,atp_task,atp_name,atp_default,atp_order) VALUES (?,?,?,0,?)").run(id, task, name, order);
      const ins = db.prepare("INSERT INTO ai_template_block (atb_id,atb_atpid,atb_block,atb_abpid,atb_enabled,atb_name,atb_prefix) VALUES (?,?,?,?,?,?,?)");
      for (const b of blocks) ins.run(crypto.randomUUID(), id, b.block, b.presetId ?? null, b.enabled !== false ? 1 : 0, b.name ?? b.block, b.prefix ?? b.block.toUpperCase());
    })();
    res.status(201).json(rowToTemplate(db.prepare("SELECT * FROM ai_template WHERE atp_id=?").get(id)));
  });

  api.put("/ai-templates/:id", (req, res) => {
    const row = db.prepare("SELECT * FROM ai_template WHERE atp_id=?").get(req.params.id);
    if (!row) return res.status(404).json({ error: "Not found" });
    const { name } = req.body;
    db.prepare("UPDATE ai_template SET atp_name=COALESCE(?,atp_name) WHERE atp_id=?").run(name ?? null, req.params.id);
    res.json(rowToTemplate(db.prepare("SELECT * FROM ai_template WHERE atp_id=?").get(req.params.id)));
  });

  api.delete("/ai-templates/:id", (req, res) => {
    const row = db.prepare("SELECT * FROM ai_template WHERE atp_id=?").get(req.params.id);
    if (!row) return res.status(404).json({ error: "Not found" });
    if (row.atp_default === 1) return res.status(403).json({ error: "Cannot delete a default template" });
    db.prepare("DELETE FROM ai_template WHERE atp_id=?").run(req.params.id);
    res.json({ deleted: req.params.id });
  });

  api.get("/ai-templates/:id/blocks", (req, res) => {
    const tpl = db.prepare("SELECT * FROM ai_template WHERE atp_id=?").get(req.params.id);
    if (!tpl) return res.status(404).json({ error: "Not found" });
    res.json(rowToTemplate(tpl).blocks);
  });

  api.put("/ai-templates/:id/blocks/:block", (req, res) => {
    const tpl = db.prepare("SELECT * FROM ai_template WHERE atp_id=?").get(req.params.id);
    if (!tpl) return res.status(404).json({ error: "Not found" });
    const { presetId, enabled, name, prefix } = req.body;
    const existing = db.prepare("SELECT * FROM ai_template_block WHERE atb_atpid=? AND atb_block=?").get(req.params.id, req.params.block);
    if (existing) {
      db.prepare("UPDATE ai_template_block SET atb_abpid=?,atb_enabled=?,atb_name=COALESCE(?,atb_name),atb_prefix=COALESCE(?,atb_prefix) WHERE atb_id=?")
        .run(presetId ?? null, enabled !== false ? 1 : 0, name ?? null, prefix ?? null, existing.atb_id);
    } else {
      db.prepare("INSERT INTO ai_template_block (atb_id,atb_atpid,atb_block,atb_abpid,atb_enabled,atb_name,atb_prefix) VALUES (?,?,?,?,?,?,?)")
        .run(crypto.randomUUID(), req.params.id, req.params.block, presetId ?? null, enabled !== false ? 1 : 0, name ?? req.params.block, prefix ?? req.params.block.toUpperCase());
    }
    res.json(rowToTemplate(db.prepare("SELECT * FROM ai_template WHERE atp_id=?").get(req.params.id)));
  });

  api.delete("/ai-templates/:id/blocks/:block", (req, res) => {
    const tpl = db.prepare("SELECT * FROM ai_template WHERE atp_id=?").get(req.params.id);
    if (!tpl) return res.status(404).json({ error: "Not found" });
    if (tpl.atp_default === 1) return res.status(403).json({ error: "Cannot modify a default template" });
    db.prepare("DELETE FROM ai_template_block WHERE atb_atpid=? AND atb_block=?").run(req.params.id, req.params.block);
    res.json(rowToTemplate(db.prepare("SELECT * FROM ai_template WHERE atp_id=?").get(req.params.id)));
  });

  // ── AI System blocks (read-only from seeds) ───────────────────────────────────
  api.get("/ai-system", (req, res) => {
    res.json(systemBlocks);
  });

  api.get("/ai-wizards", (req, res) => {
    const wizards = db.prepare("SELECT awz_id AS id, awz_name AS name, awz_order AS 'order' FROM ai_wizard ORDER BY awz_order").all();
    for (const w of wizards) {
      w.steps = db.prepare("SELECT awt_task AS task, awt_category AS category, awt_order AS 'order' FROM ai_wizard_task WHERE awt_awzid=? ORDER BY awt_order").all(w.id);
    }
    res.json(wizards);
  });

  // ── Debug write (temporary) ──────────────────────────────────────────────────
  api.post("/debug-write", (req, res) => {
    const { filename, content } = req.body;
    if (!filename || !content) return res.status(400).json({ error: "Missing fields" });
    const safeName = filename.replace(/[^a-zA-Z0-9._-]/g, "");
    const logsDir = isDev
      ? path.join(__dirname, "..", "local", "logs")
      : path.join(app.getPath("userData"), "Logs");
    fs.mkdirSync(logsDir, { recursive: true });
    fs.writeFileSync(path.join(logsDir, safeName), content, "utf8");
    res.json({ ok: true });
  });

  // ── Settings ──────────────────────────────────────────────────────────────────
  api.get("/settings", (req, res) => {
    res.json(readSettings());
  });

  api.post("/settings", (req, res) => {
    const current = readSettings();
    const updated = { ...current, ...req.body };
    writeSettings(updated);
    res.json(updated);
  });

  // ── Card URLs ───────────────────────────────────────────────────────────────
  api.get("/cards/:id/urls", (req, res) => {
    const rows = db.prepare("SELECT * FROM url WHERE url_parent_id=? ORDER BY url_order").all(req.params.id);
    res.json(rows.map(u => ({ id: u.url_id, label: u.url_label, href: u.url_href })));
  });

  api.post("/cards/:id/urls", (req, res) => {
    const { label, href } = req.body;
    if (!href) return res.status(400).json({ error: "href required" });
    const id = crypto.randomUUID();
    const order = db.prepare("SELECT COUNT(*) as n FROM url WHERE url_parent_id=?").get(req.params.id).n;
    db.prepare("INSERT INTO url (url_id,url_parent_id,url_type,url_label,url_href,url_order) VALUES (?,?,?,?,?,?)").run(id, req.params.id, 'cad', label || "", href, order);
    res.json(rowToCard(db.prepare("SELECT * FROM card WHERE cad_id=?").get(req.params.id)));
  });

  api.delete("/cards/:id/urls/:urlId", (req, res) => {
    db.prepare("DELETE FROM url WHERE url_id=? AND url_parent_id=?").run(req.params.urlId, req.params.id);
    res.json(rowToCard(db.prepare("SELECT * FROM card WHERE cad_id=?").get(req.params.id)));
  });

  // ── Project URLs ──────────────────────────────────────────────────────────────
  api.get("/projects/:id/urls", (req, res) => {
    const rows = db.prepare("SELECT * FROM url WHERE url_parent_id=? AND url_type='prj' ORDER BY url_order").all(req.params.id);
    res.json(rows.map(u => ({ id: u.url_id, label: u.url_label, href: u.url_href })));
  });

  api.post("/projects/:id/urls", (req, res) => {
    const { label, href } = req.body;
    if (!href) return res.status(400).json({ error: "href required" });
    const id = crypto.randomUUID();
    const order = db.prepare("SELECT COUNT(*) as n FROM url WHERE url_parent_id=? AND url_type='prj'").get(req.params.id).n;
    db.prepare("INSERT INTO url (url_id,url_parent_id,url_type,url_label,url_href,url_order) VALUES (?,?,?,?,?,?)").run(id, req.params.id, 'prj', label || "", href, order);
    res.json(rowToProject(db.prepare("SELECT * FROM project WHERE prj_id=?").get(req.params.id)));
  });

  api.delete("/projects/:id/urls/:urlId", (req, res) => {
    db.prepare("DELETE FROM url WHERE url_id=? AND url_parent_id=? AND url_type='prj'").run(req.params.urlId, req.params.id);
    res.json(rowToProject(db.prepare("SELECT * FROM project WHERE prj_id=?").get(req.params.id)));
  });

  // ── Sync Music URLs from Playlist ──────────────────────────────────────────────
  api.post("/projects/:id/sync-music-urls", async (req, res) => {
    try {
      const musicSites = (readSettings()[`${STORAGE_PREFIX}-music-sites`] || "soundcloud").split(",").map(s => s.trim().toLowerCase()).filter(Boolean);

      // 1. Find the project's playlist URL
      const projUrls = db.prepare("SELECT * FROM url WHERE url_parent_id=? AND url_type='prj'").all(req.params.id);
      let playlistUrl = null;
      let siteLabel = null;
      for (const u of projUrls) {
        try {
          const host = new URL(u.url_href).hostname.toLowerCase();
          const match = musicSites.find(s => host.includes(s));
          if (match) { playlistUrl = u.url_href; siteLabel = match; break; }
        } catch (_) {}
      }
      if (!playlistUrl) return res.status(400).json({ error: "noPlaylistUrl" });

      // 2. Fetch and parse the playlist page
      const resp = await fetch(playlistUrl, { headers: { "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36" } });
      if (!resp.ok) return res.status(502).json({ error: `Fetch failed: ${resp.status}` });
      const html = await resp.text();

      // 3. Extract __sc_hydration JSON
      const hydrationMatch = html.match(/window\.__sc_hydration\s*=\s*(\[.+?\]);\s*<\/script>/s);
      if (!hydrationMatch) return res.status(502).json({ error: "Could not parse playlist page" });

      let hydration;
      try { hydration = JSON.parse(hydrationMatch[1]); } catch (_) {
        return res.status(502).json({ error: "Could not parse hydration data" });
      }

      // 4. Find playlist and extract tracks
      const playlistData = hydration.find(h => h.hydratable === "playlist")?.data;
      if (!playlistData || !playlistData.tracks) return res.status(502).json({ error: "No playlist data found" });

      const rawTracks = playlistData.tracks;
      const fullTracks = [];
      const stubIds = [];
      const stubPositions = {};

      for (let i = 0; i < rawTracks.length; i++) {
        const t = rawTracks[i];
        if (t.permalink_url) {
          fullTracks.push({ index: i, url: t.permalink_url, title: t.title || "" });
        } else if (t.id) {
          stubIds.push(t.id);
          stubPositions[t.id] = i;
        }
      }

      // 5. Resolve stub tracks via SoundCloud internal API
      if (stubIds.length > 0) {
        const clientData = hydration.find(h => h.hydratable === "apiClient")?.data;
        const clientId = clientData?.client_id || clientData?.id;
        if (clientId) {
          try {
            const apiUrl = `https://api-v2.soundcloud.com/tracks?ids=${stubIds.join(",")}&client_id=${clientId}`;
            const apiResp = await fetch(apiUrl, { headers: { "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36" } });
            if (apiResp.ok) {
              const apiTracks = await apiResp.json();
              for (const t of apiTracks) {
                if (t.permalink_url && stubPositions[t.id] !== undefined) {
                  fullTracks.push({ index: stubPositions[t.id], url: t.permalink_url, title: t.title || "" });
                }
              }
            }
          } catch (_) { /* stubs stay unresolved */ }
        }
      }

      // Sort by original playlist position
      fullTracks.sort((a, b) => a.index - b.index);

      if (!fullTracks.length) return res.status(502).json({ error: "No track URLs found" });

      // 6. Build ?in= suffix from playlist URL path
      const playlistPath = new URL(playlistUrl).pathname.replace(/^\//, "");

      // 7. Get project songs
      const songs = db.prepare("SELECT * FROM card WHERE cad_prjid=? AND cad_type='song' ORDER BY cad_order, cad_name").all(req.params.id);
      if (!songs.length) return res.status(400).json({ error: "noSongs" });

      // 8. Cascading match: sortNumber+exact → sortNumber+partial → partial name only
      const matched = new Map(); // songId → track
      const usedTracks = new Set();

      function normalize(s) { return (s || "").toLowerCase().replace(/[^\p{L}\p{N}]/gu, ""); }

      // Pass 1: sortNumber + exact name
      for (const song of songs) {
        if (matched.has(song.cad_id)) continue;
        const idx = (song.cad_order ?? -1) - 1; // sortNumber is 1-based, track index is 0-based
        if (idx < 0 || idx >= fullTracks.length || usedTracks.has(idx)) continue;
        const track = fullTracks[idx];
        if (normalize(song.cad_name) === normalize(track.title)) {
          matched.set(song.cad_id, track);
          usedTracks.add(idx);
        }
      }

      // Pass 2: sortNumber + partial name
      for (const song of songs) {
        if (matched.has(song.cad_id)) continue;
        const idx = (song.cad_order ?? -1) - 1;
        if (idx < 0 || idx >= fullTracks.length || usedTracks.has(idx)) continue;
        const track = fullTracks[idx];
        const ns = normalize(song.cad_name);
        const nt = normalize(track.title);
        if (ns && nt && (nt.includes(ns) || ns.includes(nt))) {
          matched.set(song.cad_id, track);
          usedTracks.add(idx);
        }
      }

      // Pass 3: partial name match only (no position constraint)
      for (const song of songs) {
        if (matched.has(song.cad_id)) continue;
        const ns = normalize(song.cad_name);
        if (!ns) continue;
        for (let i = 0; i < fullTracks.length; i++) {
          if (usedTracks.has(i)) continue;
          const nt = normalize(fullTracks[i].title);
          if (nt && (nt.includes(ns) || ns.includes(nt))) {
            matched.set(song.cad_id, fullTracks[i]);
            usedTracks.add(i);
            break;
          }
        }
      }

      // 9. Upsert URLs for matched songs
      let added = 0, updated = 0;
      const stmtFind = db.prepare("SELECT url_id FROM url WHERE url_parent_id=? AND url_type='cad' AND LOWER(url_label)=?");
      const stmtUpdate = db.prepare("UPDATE url SET url_href=? WHERE url_id=?");
      const stmtCount = db.prepare("SELECT COUNT(*) as n FROM url WHERE url_parent_id=?");
      const stmtInsert = db.prepare("INSERT INTO url (url_id,url_parent_id,url_type,url_label,url_href,url_order) VALUES (?,?,?,?,?,?)");

      for (const [songId, track] of matched) {
        const finalUrl = `${track.url}?in=${playlistPath}`;
        const existing = stmtFind.get(songId, siteLabel);
        if (existing) {
          stmtUpdate.run(finalUrl, existing.url_id);
          updated++;
        } else {
          const order = stmtCount.get(songId).n;
          stmtInsert.run(crypto.randomUUID(), songId, "prt", siteLabel, finalUrl, order);
          added++;
        }
      }

      // 10. Return updated cards
      const updatedCards = songs.map(s => rowToCard(s));
      res.json({ matched: matched.size, added, updated, skipped: songs.length - matched.size, totalTracks: fullTracks.length, totalSongs: songs.length, cards: updatedCards });
    } catch (err) {
      console.error("sync-music-urls error:", err);
      res.status(500).json({ error: err.message });
    }
  });

  // ── Translations ───────────────────────────────────────────────────────────────
  api.get("/translations", (req, res) => {
    const rows = db.prepare("SELECT cat_cadid, cat_lyrics FROM card_translation").all();
    const index = {};
    for (const r of rows) {
      if (!index[r.cat_cadid]) index[r.cat_cadid] = "";
      else index[r.cat_cadid] += "\n";
      index[r.cat_cadid] += r.cat_lyrics;
    }
    res.json(index);
  });

  api.get("/translations/:cardId", (req, res) => {
    const rows = db.prepare("SELECT * FROM card_translation WHERE cat_cadid=? ORDER BY cat_date ASC").all(req.params.cardId);
    res.json(rows.map(r => ({ id: r.cat_id, name: r.cat_name, lang: r.cat_lang, content: r.cat_lyrics, date: r.cat_date })));
  });

  api.post("/translations", (req, res) => {
    const { cardId, lang, name = '', content = '' } = req.body;
    if (!cardId || !lang) return res.status(400).json({ error: "Missing fields" });
    const exists = db.prepare("SELECT cat_id FROM card_translation WHERE cat_cadid=? AND cat_lang=?").get(cardId, lang);
    if (exists) return res.status(409).json({ error: "Translation for this language already exists" });
    const id = crypto.randomUUID();
    const now = Date.now();
    db.prepare("INSERT INTO card_translation (cat_id,cat_cadid,cat_name,cat_lang,cat_lyrics,cat_date) VALUES (?,?,?,?,?,?)").run(id, cardId, name, lang, content, now);
    res.status(201).json({ id, name, lang, content, date: now });
  });

  api.put("/translations/:id", (req, res) => {
    const { name, content } = req.body;
    const row = db.prepare("SELECT * FROM card_translation WHERE cat_id=?").get(req.params.id);
    if (!row) return res.status(404).json({ error: "Not found" });
    db.prepare("UPDATE card_translation SET cat_name=?,cat_lyrics=?,cat_date=? WHERE cat_id=?").run(name ?? row.cat_name, content ?? row.cat_lyrics, Date.now(), req.params.id);
    const updated = db.prepare("SELECT * FROM card_translation WHERE cat_id=?").get(req.params.id);
    res.json({ id: updated.cat_id, name: updated.cat_name, lang: updated.cat_lang, content: updated.cat_lyrics, date: updated.cat_date });
  });

  api.delete("/translations/:id", (req, res) => {
    const result = db.prepare("DELETE FROM card_translation WHERE cat_id=?").run(req.params.id);
    if (result.changes === 0) return res.status(404).json({ error: "Not found" });
    res.json({ deleted: req.params.id });
  });

  api.post("/translations/languages", (req, res) => {
    const { projectIds = [] } = req.body;
    let rows;
    if (projectIds.length > 0) {
      const ph = projectIds.map(() => "?").join(",");
      rows = db.prepare(`
        SELECT DISTINCT pt.cat_lang
        FROM card_translation pt
        JOIN card p ON p.cad_id = pt.cat_cadid
        WHERE p.cad_prjid IN (${ph}) AND p.cad_type = 'song'
        ORDER BY pt.cat_lang
      `).all(...projectIds);
    } else {
      rows = db.prepare(`
        SELECT DISTINCT pt.cat_lang
        FROM card_translation pt
        JOIN card p ON p.cad_id = pt.cat_cadid
        WHERE p.cad_type = 'song'
        ORDER BY pt.cat_lang
      `).all();
    }
    res.json(rows.map(r => r.cat_lang));
  });

  // ── Folder dialog ─────────────────────────────────────────────────────────────
  api.get("/open-folder-dialog", async (req, res) => {
    const { canceled, filePaths } = await dialog.showOpenDialog(mainWindow, {
      properties: ["openDirectory"],
    });
    res.json(canceled ? { path: null } : { path: filePaths[0] });
  });

  // ── Import songs from folder ───────────────────────────────────────────────────
  api.post("/import-folder", (req, res) => {
    const { folderPath, projectId, songType } = req.body;
    if (!folderPath) return res.status(400).json({ error: "folderPath required" });
    const resolvedType = "song";

    let files;
    try {
      files = fs.readdirSync(folderPath).filter(f => /\.(mp3|flac)$/i.test(f)).sort();
    } catch (err) {
      return res.status(400).json({ error: "Cannot read folder: " + err.message });
    }

    if (!files.length) return res.json({ created: 0 });

    const created = [];
    db.transaction(() => {
      for (const file of files) {
        const ext = path.extname(file);
        const base = path.basename(file, ext);
        // Parse "##-Name" or "## - Name" pattern
        const match = base.match(/^(\d+)\s*[--]\s*(.+)$/);
        const sortNumber = match ? parseInt(match[1], 10) : null;
        const name = match ? match[2].trim() : base;
        const mediaPath = path.join(folderPath, file);

        const id = crypto.randomUUID();
        const version = nextCardVersion(name, resolvedType);
        const order = db.prepare("SELECT COUNT(*) as n FROM card").get().n;
        db.prepare("INSERT INTO card(cad_id,cad_name,cad_version,cad_type,cad_style,cad_tags,cad_favorite,cad_date,cad_order,cad_media_path) VALUES (?,?,?,?,?,?,?,?,?,?)")
          .run(id, name, version, resolvedType, "", "[]", 0, Date.now(), sortNumber, mediaPath);

        if (projectId) {
          db.prepare("UPDATE card SET cad_prjid=? WHERE cad_id=?").run(projectId, id);
        }
        created.push(id);
      }
    })();

    res.json({ created: created.length });
  });

  // Read the OS clipboard — done in the main process because the renderer's
  // navigator.clipboard.readText() needs a focused document and a permission grant.
  api.get("/clipboard", (_req, res) => res.json({ text: clipboard.readText() || "" }));

  // A .amlp opened from the desktop, waiting to be imported. Read-once: the path is cleared
  // as it is handed over, so a re-check (the renderer asks again on every window focus) can
  // never import the same file twice.
  api.get("/pending-open", (_req, res) => {
    const filePath = pendingOpenPath;
    pendingOpenPath = null;
    if (!filePath) return res.json({});
    try {
      return res.json({ name: path.basename(filePath), text: fs.readFileSync(filePath, "utf8") });
    } catch (err) {
      console.error("pending .amlp read failed:", err.message);
      return res.json({ error: err.message });
    }
  });

  // Minimal entity decode — <meta> content is HTML-escaped ("Rock &amp; Roll").
  const decodeEntities = (s) => String(s || "")
    .replace(/&quot;/g, '"').replace(/&#0?39;|&#x27;/gi, "'")
    .replace(/&lt;/g, "<").replace(/&gt;/g, ">").replace(/&amp;/g, "&");

  // Browser-looking headers for the song-page scrapers below — these sites serve a stripped
  // page (or nothing) to a bare Node user agent.
  const SCRAPE_HEADERS = {
    "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36",
    "Accept": "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
    "Accept-Language": "en-US,en;q=0.5",
  };

  // ── Fetch tunee.ai lyrics ─────────────────────────────────────────────────────
  api.post("/fetch-tunee-lyrics", async (req, res) => {
    const { url } = req.body;
    if (!url || !url.includes("tunee.ai/music/")) return res.status(400).json({ error: "Not a tunee.ai music URL" });
    try {
      const html = await new Promise((resolve, reject) => {
        const request = https.get(url, {
          headers: SCRAPE_HEADERS
        }, (response) => {
          let data = "";
          response.on("data", chunk => data += chunk);
          response.on("end", () => resolve(data));
        });
        request.on("error", reject);
        request.setTimeout(15000, () => { request.destroy(); reject(new Error("Request timed out")); });
      });

      // Tunee uses Next.js App Router RSC streaming.
      // Lyrics are stored as a T-type (text) RSC chunk referenced by "$N" in the song data.
      // Format inside a push([1,"..."]): "...N:T{hex_len},{content...}" possibly split across pushes.

      // Collect all decoded RSC push strings in order
      const rscRe = /self\.__next_f\.push\(\[1,"((?:[^"\\]|\\.)*)"\]\)/g;
      const pushes = [];
      let m;
      while ((m = rscRe.exec(html)) !== null) {
        try { pushes.push(JSON.parse('"' + m[1] + '"')); } catch (_) { pushes.push(""); }
      }

      // Step 1: find which chunk ID holds the lyrics via "lyrics":"$N"
      let lyricsId = null;
      for (const p of pushes) {
        const ref = p.match(/"lyrics":"\$(\d+)"/);
        if (ref) { lyricsId = ref[1]; break; }
      }
      if (!lyricsId) return res.json({ lyrics: "" });

      // Step 2: find the T-type chunk for that exact ID and collect its full content
      for (let i = 0; i < pushes.length; i++) {
        const tMatch = pushes[i].match(new RegExp("\\n" + lyricsId + ":T([0-9a-f]+),([\\s\\S]*)$"));
        if (!tMatch) continue;
        const expectedBytes = parseInt(tMatch[1], 16);
        let collected = tMatch[2];
        for (let j = i + 1; j < pushes.length && Buffer.byteLength(collected, "utf8") < expectedBytes; j++) {
          collected += pushes[j];
        }
        const lyrics = Buffer.from(collected, "utf8").slice(0, expectedBytes).toString("utf8").trim();
        if (lyrics.length > 0) return res.json({ lyrics });
      }

      return res.json({ lyrics: "" });
    } catch (err) {
      return res.status(500).json({ error: err.message });
    }
  });

  // ── Resolve a Suno share link to its canonical song URL ───────────────────────
  // A share link (suno.com/s/<code>) is a redirect stub: Suno answers it with a 307 whose
  // Location is /song/<uuid>?sh=<code>. Reading that one header is enough — the response body
  // is never consumed (the song page is megabytes of RSC), and only the uuid is kept, so the
  // stored URL matches the one media provenance produces (https://suno.com/song/<uuid>).
  api.post("/resolve-suno-share", async (req, res) => {
    const { url } = req.body;
    if (!url || !/^https?:\/\/(www\.)?suno\.com\/s\/[A-Za-z0-9_-]+/.test(url)) {
      return res.status(400).json({ error: "Not a Suno share URL" });
    }
    try {
      const location = await new Promise((resolve, reject) => {
        const request = https.get(url, { headers: SCRAPE_HEADERS }, (response) => {
          response.resume(); // headers are all we need — drain so the socket can be freed
          const isRedirect = response.statusCode >= 300 && response.statusCode < 400;
          resolve(isRedirect ? (response.headers.location || "") : "");
        });
        request.on("error", reject);
        request.setTimeout(15000, () => { request.destroy(); reject(new Error("Request timed out")); });
      });
      const id = location.match(/\/song\/([0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})/i);
      return res.json({ url: id ? `https://suno.com/song/${id[1].toLowerCase()}` : "" });
    } catch (err) {
      return res.status(500).json({ error: err.message });
    }
  });

  // ── Fetch Suno lyrics ─────────────────────────────────────────────────────────
  api.post("/fetch-suno-lyrics", async (req, res) => {
    const { url } = req.body;
    if (!url || !url.includes("suno.com/song/")) return res.status(400).json({ error: "Not a Suno song URL" });
    try {
      const html = await new Promise((resolve, reject) => {
        const request = https.get(url, {
          headers: SCRAPE_HEADERS
        }, (response) => {
          let data = "";
          response.on("data", chunk => data += chunk);
          response.on("end", () => resolve(data));
        });
        request.on("error", reject);
        request.setTimeout(15000, () => { request.destroy(); reject(new Error("Request timed out")); });
      });

      // Suno uses Next.js App Router RSC streaming.
      // Song data is double-encoded inside self.__next_f.push([1,"..."]) script tags.
      // Collect all decoded RSC push strings in order.
      const rscRe = /self\.__next_f\.push\(\[1,"((?:[^"\\]|\\.)*)"\]\)/g;
      let m;
      const pushes = [];
      while ((m = rscRe.exec(html)) !== null) {
        try { pushes.push(JSON.parse('"' + m[1] + '"')); } catch (_) { pushes.push(""); }
      }

      // Extract style from metadata.tags or display_tags.
      // ⚠ CLAUDE: these are JSON string values inside the RSC payload, so the capture MUST be the
      // escape-aware `(?:[^"\\]|\\.)*` and go through JSON.parse — a plain `[^"]+` truncates the
      // style at the first escaped quote (a prompt saying a driving \"walking\" bass line lost
      // everything after "driving").
      const jsonStr = (raw) => { try { return JSON.parse('"' + raw + '"'); } catch (_) { return raw; } };
      let style = "";
      for (const p of pushes) {
        // metadata.tags — take the first "tags" after the object opens; it is metadata's own.
        const metaIdx = p.indexOf('"metadata":{');
        if (metaIdx !== -1) {
          const tagsM = p.slice(metaIdx).match(/"tags":"((?:[^"\\]|\\.)*)"/);
          if (tagsM) { style = jsonStr(tagsM[1]).trim(); break; }
        }
        // display_tags fallback
        const tm = p.match(/"display_tags":"((?:[^"\\]|\\.)*)"/);
        if (tm) { style = jsonStr(tm[1]).trim(); break; }
      }

      // Song title + creation date — what a card built from the link uses for its name,
      // its sort number ("10-Sanctus") and its date. og:title is the bare title; the
      // <title> tag adds " by <artist> | Suno", so it is only the fallback.
      let title = "";
      const ogM = html.match(/property="og:title"[^>]*content="([^"]*)"/i) || html.match(/content="([^"]*)"[^>]*property="og:title"/i);
      if (ogM) title = decodeEntities(ogM[1]).trim();
      if (!title) {
        const hM = html.match(/<title>([^<]*)<\/title>/i);
        if (hM) title = decodeEntities(hM[1]).replace(/\s*\|\s*Suno\s*$/i, "").replace(/\s+by\s+[^|]*$/i, "").trim();
      }
      let date = "";
      for (const p of pushes) {
        const dm = p.match(/"created_at":"([^"]+)"/);
        if (dm) { date = dm[1]; break; }
      }
      const info = { style, title, date };

      // Case 1: inline lyrics - "prompt":"[Verse]..." (string value, starts with "[")
      for (const inner of pushes) {
        const pRe = /"prompt":"(\[(?:[^"\\]|\\.)*)"/g;
        let pm;
        while ((pm = pRe.exec(inner)) !== null) {
          try {
            const lyrics = JSON.parse('"' + pm[1] + '"');
            if (lyrics.trim()) return res.json({ lyrics: lyrics.trim(), ...info });
          } catch (_) {}
        }
      }

      // Case 2: T-chunk reference - "prompt":"$3f" → find T-chunk "3f:T{hex_len},{content}"
      let promptChunkId = null;
      for (const p of pushes) {
        const ref = p.match(/"prompt":"\$([0-9a-f]+)"/);
        if (ref) { promptChunkId = ref[1]; break; }
      }
      if (promptChunkId) {
        for (let i = 0; i < pushes.length; i++) {
          const tMatch = pushes[i].match(new RegExp("(?:^|\\n)" + promptChunkId + ":T([0-9a-f]+),([\\s\\S]*)$"));
          if (!tMatch) continue;
          const expectedBytes = parseInt(tMatch[1], 16);
          let collected = tMatch[2];
          for (let j = i + 1; j < pushes.length && Buffer.byteLength(collected, "utf8") < expectedBytes; j++) {
            collected += pushes[j];
          }
          const lyrics = Buffer.from(collected, "utf8").slice(0, expectedBytes).toString("utf8").trim();
          if (lyrics.length > 0) return res.json({ lyrics, ...info });
        }
      }

      return res.json({ lyrics: "", ...info });
    } catch (err) {
      return res.status(500).json({ error: err.message });
    }
  });

  // ── Fetch producer.ai lyrics (hidden BrowserWindow - bypasses Cloudflare) ─────
  api.post("/fetch-producer-lyrics", async (req, res) => {
    const { url } = req.body;
    if (!url || !url.includes("producer.ai/song/")) return res.status(400).json({ error: "Not a producer.ai song URL" });

    const BW = BrowserWindow;
    let win = null;
    try {
      win = new BW({
        show: false, width: 1280, height: 800,
        webPreferences: { nodeIntegration: false, contextIsolation: true }
      });

      // Wait until the window actually lands on the producer.ai song page
      // (Cloudflare challenge may cause 1-2 intermediate navigations first)
      await Promise.race([
        new Promise((resolve, reject) => {
          const onLoad = () => {
            const cur = win.webContents.getURL();
            if (cur.includes("producer.ai/song/")) {
              win.webContents.removeListener("did-finish-load", onLoad);
              resolve();
            }
            // else: still on Cloudflare or redirect - keep waiting for next load
          };
          win.webContents.on("did-finish-load", onLoad);
          win.webContents.once("did-fail-load", (e, code, desc) => reject(new Error(desc)));
          win.loadURL(url);
        }),
        new Promise((_, reject) => setTimeout(() => reject(new Error("Timeout: page never reached producer.ai")), 30000))
      ]);

      // Wait for React to hydrate after landing on the real page
      await new Promise(r => setTimeout(r, 3000));

      const extractJS = `(function() {
        let lyrics = '', style = '';

        // ── Helper: find section text after a keyword, stop at next ALL-CAPS header ─
        function extractSection(text, keywords) {
          for (const kw of keywords) {
            const re = new RegExp('\\\\b' + kw + '\\\\b');
            const idx = text.search(re);
            if (idx === -1) continue;
            const after = text.substring(idx).replace(new RegExp('^' + kw + '[\\\\r\\\\n]+'), '').trim();
            const end = after.search(/\\n[A-Z]{4,}(\\s|\\n|$)/);
            const candidate = (end !== -1 ? after.substring(0, end) : after.substring(0, 3000)).trim();
            if (candidate.length > 20) return candidate;
          }
          return '';
        }

        // ── Body text extraction ───────────────────────────────────────────────
        const text = document.body.innerText;
        if (!style) style = extractSection(text, ['SOUND', 'STYLE', 'PROMPT']);
        if (!lyrics) lyrics = extractSection(text, ['LYRICS']);

        return JSON.stringify({ lyrics, style });
      })()`;

      const raw = await win.webContents.executeJavaScript(extractJS);
      const { lyrics, style } = JSON.parse(raw || '{"lyrics":"","style":""}');
      res.json({ lyrics: lyrics.trim(), style: style.trim() });
    } catch (err) {
      res.status(500).json({ error: err.message });
    } finally {
      if (win && !win.isDestroyed()) win.close();
    }
  });

  // ── Media dialog + stream ─────────────────────────────────────────────────────
  api.get("/open-media-dialog", async (req, res) => {
    const { canceled, filePaths } = await dialog.showOpenDialog(mainWindow, {
      properties: ["openFile"],
      defaultPath: req.query.defaultPath || undefined,
      filters: [{ name: "Audio / Video", extensions: ["mp3", "flac", "wav", "ogg", "aac", "m4a", "mp4", "webm", "opus"] }],
    });
    res.json(canceled ? { path: null } : { path: filePaths[0] });
  });

  api.get("/scan-media-folder", (req, res) => {
    const folderPath = req.query.path;
    if (!folderPath) return res.status(400).json({ error: "path required" });
    try {
      const files = fs.readdirSync(folderPath)
        .filter(f => /\.(mp3|flac|wav|ogg|aac|m4a|opus|mp4|webm)$/i.test(f))
        .sort()
        .map(f => ({ name: f, path: path.join(folderPath, f) }));
      res.json({ folder: folderPath, files });
    } catch (err) {
      res.status(400).json({ error: "Cannot read folder: " + err.message });
    }
  });

  api.get("/stream", (req, res) => {
    const filePath = req.query.path;
    if (!filePath) return res.status(400).json({ error: "path required" });
    res.sendFile(filePath, (err) => {
      if (err && !res.headersSent) res.status(404).json({ error: "File not found" });
    });
  });

  // ── Save Excel file ───────────────────────────────────────────────────────────
  api.post("/save-excel", async (req, res) => {
    const { data, title, defaultPath } = req.body;
    const { canceled, filePath } = await dialog.showSaveDialog(mainWindow, {
      title,
      defaultPath,
      filters: [{ name: "Excel Spreadsheet", extensions: ["xlsx"] }],
    });
    if (canceled || !filePath) return res.json({ canceled: true });
    fs.writeFileSync(filePath, Buffer.from(data, "base64"));
    res.json({ saved: true, filePath });
  });

  // ⚠ CLAUDE: JSON export saves through the Electron save dialog, NOT a renderer
  // <a download> blob click — that silently no-ops in Electron (anchor never
  // triggers a download), which is why "Export" appeared to do nothing.
  api.post("/save-json", async (req, res) => {
    const { data, title, defaultPath } = req.body;
    const { canceled, filePath } = await dialog.showSaveDialog(mainWindow, {
      title,
      defaultPath,
      filters: [{ name: "JSON", extensions: ["json"] }],
    });
    if (canceled || !filePath) return res.json({ canceled: true });
    fs.writeFileSync(filePath, data, "utf8");
    res.json({ saved: true, filePath });
  });

  // ── Export Word document ──────────────────────────────────────────────────────
  api.post("/export-docx", async (req, res) => {
    try {
      const { projectIds = [], contentLyrics = "clean", contentLangs = [], contentImages = true, contentDesc = true, filterLyrics = "withLyrics", filterPublished = "all", uiLang = "en", title, instrumentalLabel = "Instrumental" } = req.body;
      const mm = await import("music-metadata");

      // Page geometry: A4 (11906 twips wide), margins 900 twips each side
      // Content width in pixels at 96 DPI: (11906 - 1800) * 635 / 9525 ≈ 673
      const CONTENT_WIDTH_PX = 673;
      const PAGE_MARGIN = 900; // twips

      // Fetch songs ordered by their position in the project
      let songs;
      if (projectIds.length > 0) {
        const placeholders = projectIds.map(() => "?").join(",");
        const rows = db.prepare(`
          SELECT * FROM card
          WHERE cad_prjid IN (${placeholders})
            AND cad_type = 'song'
          ORDER BY cad_order, cad_name
        `).all(...projectIds);
        songs = rows.map(rowToCard);
      } else {
        const rows = db.prepare(
          "SELECT * FROM card WHERE cad_type = 'song' ORDER BY cad_order, cad_name"
        ).all();
        songs = rows.map(rowToCard);
      }

      if (!songs.length) return res.status(400).json({ error: "No songs found" });
      if (filterLyrics === "withLyrics") songs = songs.filter(s => hasRealLyrics(s.lyrics));
      if (filterPublished === "publishedOnly") { const ms = (readSettings()[`${STORAGE_PREFIX}-music-sites`] || "soundcloud").split(",").map(s => s.trim().toLowerCase()).filter(Boolean); songs = songs.filter(s => (s.urls || []).some(u => ms.includes((u.label || "").toLowerCase().trim()))); }
      if (!songs.length) return res.status(400).json({ error: "No songs match the filters (lyrics / published)" });

      // Default filename: project name when exactly one project selected
      let defaultName = "songs";
      let defaultDir = "";
      if (projectIds.length === 1) {
        const proj = db.prepare("SELECT prj_name, prj_path FROM project WHERE prj_id = ?").get(projectIds[0]);
        if (proj) {
          defaultName = proj.prj_name.replace(/[\\/:*?"<>|]/g, "_");
          if (proj.prj_path) defaultDir = proj.prj_path;
        }
      }

      // Save dialog
      const { canceled, filePath } = await dialog.showSaveDialog(mainWindow, {
        title,
        defaultPath: defaultDir ? path.join(defaultDir, `${defaultName}.docx`) : `${defaultName}.docx`,
        filters: [{ name: "Word Document", extensions: ["docx"] }],
      });
      if (canceled || !filePath) return res.json({ canceled: true });

      // ── Helpers ──────────────────────────────────────────────────────────────
      function mimeToType(mime) {
        return ({ "image/jpeg": "jpg", "image/png": "png", "image/gif": "gif", "image/bmp": "bmp", "image/webp": "jpg" })[mime] || "jpg";
      }

      function getImgDimensions(buf, mime) {
        try {
          if (mime === "image/png") {
            return { w: buf.readUInt32BE(16), h: buf.readUInt32BE(20) };
          }
          // JPEG: scan for SOF marker
          let i = 2;
          while (i < buf.length - 3) {
            if (buf[i] !== 0xFF) break;
            const marker = buf[i + 1];
            const segLen = buf.readUInt16BE(i + 2);
            if (marker >= 0xC0 && marker <= 0xCF && marker !== 0xC4 && marker !== 0xC8 && marker !== 0xCC) {
              return { h: buf.readUInt16BE(i + 5), w: buf.readUInt16BE(i + 7) };
            }
            i += 2 + segLen;
          }
        } catch (_) {}
        return null;
      }

      async function extractCover(mediaPath) {
        if (!mediaPath || !fs.existsSync(mediaPath)) return null;
        try {
          const meta = await mm.parseFile(mediaPath, { duration: false, skipCovers: false });
          const pic = meta.common.picture?.[0];
          if (!pic) return null;
          const type = mimeToType(pic.format);
          const dims = getImgDimensions(pic.data, pic.format);
          const w = CONTENT_WIDTH_PX;
          const h = dims ? Math.round(w * dims.h / dims.w) : w;
          return { data: pic.data, type, w, h };
        } catch (_) { return null; }
      }

      // folder.jpg fallback — the album cover beside the media, used when the
      // audio file carries no embedded art (covers are tagged externally, so
      // embedded art is often absent). Cached per directory.
      const folderCoverCache = new Map();
      function folderCover(mediaPath) {
        const dir = path.dirname(mediaPath);
        if (!folderCoverCache.has(dir)) {
          let cover = null;
          try {
            const candidate = path.join(dir, "folder.jpg");
            if (fs.existsSync(candidate)) {
              const data = fs.readFileSync(candidate);
              const dims = getImgDimensions(data, "image/jpeg");
              const w = CONTENT_WIDTH_PX;
              const h = dims ? Math.round(w * dims.h / dims.w) : w;
              cover = { data, type: "jpg", w, h };
            }
          } catch (_) {}
          folderCoverCache.set(dir, cover);
        }
        return folderCoverCache.get(dir);
      }

      function lyricsToRuns(lines) {
        return lines.map(line => {
          const t = line.trim();
          if (!t) return new Paragraph({ text: "", spacing: { before: 60, after: 60 } });
          const isMarker = /^\[.*\]$/.test(t) || /^\(.*\)$/.test(t);
          return new Paragraph({
            alignment: AlignmentType.CENTER,
            spacing: isMarker ? { before: 200, after: 60 } : { before: 0, after: 40 },
            children: [new TextRun({ text: t, size: isMarker ? 20 : 24, font: "Calibri", bold: isMarker, italics: isMarker, color: isMarker ? "999999" : "000000" })],
          });
        });
      }

      // ── Build document ────────────────────────────────────────────────────────
      function coverKey(buf) {
        return crypto.createHash("md5").update(buf).digest("hex");
      }

      const docChildren = [];
      let lastCoverKey = null;

      for (let i = 0; i < songs.length; i++) {
        const song = songs[i];
        if (i > 0) {
          docChildren.push(new Paragraph({ pageBreakBefore: true, text: "" }));
        }

        // Cover art - skip if images disabled or image data is identical to previous song
        if (contentImages && song.mediaPath) {
          const cover = await extractCover(song.mediaPath) || folderCover(song.mediaPath);
          if (cover) {
            const key = coverKey(cover.data);
            if (key !== lastCoverKey) {
              docChildren.push(new Paragraph({
                alignment: AlignmentType.CENTER,
                spacing: { before: 0, after: 360 },
                children: [new ImageRun({ data: cover.data, type: cover.type, transformation: { width: cover.w, height: cover.h } })],
              }));
              lastCoverKey = key;
            }
          }
        }

        // Title
        const title = song.sortNumber ? `${song.sortNumber}. ${song.name}` : song.name;
        docChildren.push(new Paragraph({
          alignment: AlignmentType.CENTER,
          spacing: { before: 0, after: 480 },
          children: [new TextRun({ text: title, bold: true, size: 48, color: "7c6fff", font: "Calibri Light" })],
        }));

        // Description (cad_desc)
        if (contentDesc && song.desc) {
          docChildren.push(new Paragraph({
            alignment: AlignmentType.CENTER,
            spacing: { before: 0, after: 240 },
            children: [new TextRun({ text: song.desc, size: 22, font: "Calibri", italics: true, color: "555555" })],
          }));
        }

        // Lyrics
        const lines = songLyricsLines(song.lyrics, contentLyrics, instrumentalLabel);
        if (lines.length) docChildren.push(...lyricsToRuns(lines));

        // Translations
        if (contentLangs.length > 0) {
          const trs = db.prepare("SELECT * FROM card_translation WHERE cat_cadid=? ORDER BY cat_date ASC").all(song.id);
          for (const tr of trs.filter(tr => contentLangs.includes(tr.cat_lang))) {
            const langLabel = getLangName(tr.cat_lang, uiLang);
            docChildren.push(new Paragraph({
              alignment: AlignmentType.CENTER,
              spacing: { before: 480, after: 200 },
              children: [new TextRun({ text: `── ${langLabel} (${tr.cat_lang}) ──`, size: 20, font: "Calibri", color: "888888", italics: true })],
            }));
            const trLines = extractLyrics(tr.cat_lyrics, contentLyrics);
            if (trLines.length) {
              docChildren.push(...lyricsToRuns(trLines));
            }
          }
        }
      }

      const doc = new Document({
        styles: { default: { document: { run: { font: "Calibri", size: 24 } } } },
        sections: [{
          properties: { page: { margin: { top: 720, bottom: 720, left: PAGE_MARGIN, right: PAGE_MARGIN } } },
          children: docChildren,
        }],
      });

      const buffer = await Packer.toBuffer(doc);
      fs.writeFileSync(filePath, buffer);
      res.json({ success: true, path: filePath });
    } catch (err) {
      console.error("export-docx error:", err);
      res.status(500).json({ error: err.message });
    }
  });

  // ── Export .amlp (the LP prompt file) ─────────────────────────────────────────
  // The exact inverse of POST /import-amlp: a project (or the whole song list) back out as one
  // LP file. ⚠ CLAUDE: the grammar spec is app/node/suno-sync/main/README.md — header fields
  // inline, per-track fields value-below, and LYRICS LAST in each track (it runs to the next
  // "=== PROMPT ===", so a field written after it would be read back as lyric text).
  api.post("/export-amlp", async (req, res) => {
    try {
      // The renderer decides WHAT to export — the songs selected in the view, or everything
      // currently listed when nothing is selected. The server only orders and formats them, so
      // "what you see is what you export" can never drift from what the grid shows.
      const { cardIds = [], projectId = null, title } = req.body;
      if (!cardIds.length) return res.json({ error: "noSongs" });

      const placeholders = cardIds.map(() => "?").join(",");
      const songs = db.prepare(`SELECT * FROM card WHERE cad_id IN (${placeholders}) AND cad_type='song' ORDER BY cad_order, cad_name`)
        .all(...cardIds).map(rowToCard);
      if (!songs.length) return res.json({ error: "noSongs" });

      // The project only supplies the file's identity (name, folder, date) — never its contents.
      let lpName = "songs", lpDate = "", defaultDir = "";
      if (projectId) {
        const proj = db.prepare("SELECT prj_name, prj_path, prj_date FROM project WHERE prj_id=?").get(projectId);
        if (proj) {
          lpName = proj.prj_name;
          lpDate = new Date(proj.prj_date || Date.now()).toISOString().slice(0, 10);
          if (proj.prj_path) defaultDir = proj.prj_path;
        }
      }
      if (!lpDate) lpDate = new Date().toISOString().slice(0, 10);

      const safeName = lpName.replace(/[\\/:*?"<>|]/g, "_");
      const { canceled, filePath } = await dialog.showSaveDialog(mainWindow, {
        title,
        defaultPath: defaultDir ? path.join(defaultDir, `${safeName}.amlp`) : `${safeName}.amlp`,
        filters: [{ name: "AI Music Prompt Lab LP", extensions: ["amlp"] }],
      });
      if (canceled || !filePath) return res.json({ canceled: true });

      // LANG is a plain language name — the shape sunoLangToCode() reads back on import.
      // ⚠ CLAUDE: always the ENGLISH name ("en" here, never the UI language). sunoLangToCode()
      // matches SONG_LANGUAGES[].name, which is English-only, so writing a localized name
      // ("Latín" on a Spanish UI) would export a file whose language silently fails to import.
      const langCode = songs.find(s => s.lang)?.lang || "";
      const linkedStyles = db.prepare("SELECT cad_style FROM card WHERE cad_id IN (SELECT cal_cadid_child FROM card_link WHERE cal_cadid_parent=?)");

      const out = [
        `TITLE: ${lpName}`,
        `DATE: ${lpDate}`,
        `STYLE: `,
        `THEME: `,
        `LANG: ${langCode ? getLangName(langCode, "en") : ""}`,
        ``,
      ];
      songs.forEach((song, i) => {
        // A song's style is its own text plus the styles of the music/vocal cards linked to it —
        // the same composition the card's "copy with linked" action puts on the clipboard, i.e.
        // what actually gets pasted into Suno. A song with no links exports its style verbatim.
        const styles = [song.style, ...linkedStyles.all(song.id).map(r => r.cad_style)]
          .map(s => (s || "").trim()).filter(Boolean);
        const aiUrl = (song.urls || []).find(u => /suno\.com|producer\.ai|tunee\.ai|mozartai\.com/i.test(u.href || ""));
        const num = String(song.sortNumber ?? (i + 1)).padStart(2, "0");
        out.push(
          `=== PROMPT ${num} ===`,
          `TITLE:`, song.name || "", ``,
          `DESCRIPTION:`, song.desc || "", ``,
          `STYLE:`, styles.join("\n"), ``,
        );
        if (aiUrl) out.push(`URL:`, aiUrl.href, ``);
        out.push(`LYRICS:`, song.lyrics || "", ``);
      });

      fs.writeFileSync(filePath, out.join("\n"), "utf8");
      res.json({ saved: true, filePath, songs: songs.length });
    } catch (err) {
      console.error(".amlp export error:", err.message);
      res.status(500).json({ error: err.message });
    }
  });

  // ── Export Markdown ───────────────────────────────────────────────────────────
  api.post("/export-md", async (req, res) => {
    try {
      const { projectIds = [], contentLyrics = "clean", contentLangs = [], contentImages = true, contentDesc = true, filterLyrics = "withLyrics", filterPublished = "all", uiLang = "en", title, translationLabel = "Translation", instrumentalLabel = "Instrumental" } = req.body;
      const musicSites = (readSettings()[`${STORAGE_PREFIX}-music-sites`] || "soundcloud").split(",").map(s => s.trim().toLowerCase()).filter(Boolean);

      let songs;
      if (projectIds.length > 0) {
        const placeholders = projectIds.map(() => "?").join(",");
        const rows = db.prepare(`
          SELECT * FROM card
          WHERE cad_prjid IN (${placeholders})
            AND cad_type = 'song'
          ORDER BY cad_order, cad_name
        `).all(...projectIds);
        songs = rows.map(rowToCard);
      } else {
        songs = db.prepare("SELECT * FROM card WHERE cad_type = 'song' ORDER BY cad_order, cad_name").all().map(rowToCard);
      }

      if (!songs.length) return res.status(400).json({ error: "No songs found" });
      if (filterLyrics === "withLyrics") songs = songs.filter(s => hasRealLyrics(s.lyrics));
      if (filterPublished === "publishedOnly") songs = songs.filter(s => (s.urls || []).some(u => musicSites.includes((u.label || "").toLowerCase().trim())));
      if (!songs.length) return res.status(400).json({ error: "No songs match the filters (lyrics / published)" });

      let defaultName = "songs";
      let defaultDir = "";
      if (projectIds.length === 1) {
        const proj = db.prepare("SELECT prj_name, prj_path FROM project WHERE prj_id = ?").get(projectIds[0]);
        if (proj) {
          defaultName = proj.prj_name.replace(/[\\/:*?"<>|]/g, "_");
          if (proj.prj_path) defaultDir = proj.prj_path;
        }
      }

      const { canceled, filePath } = await dialog.showSaveDialog(mainWindow, {
        title,
        defaultPath: defaultDir ? path.join(defaultDir, `${defaultName}.md`) : `${defaultName}.md`,
        filters: [{ name: "Markdown", extensions: ["md"] }],
      });
      if (canceled || !filePath) return res.json({ canceled: true });

      songs.sort((a, b) => {
        const na = parseFloat(a.sortNumber) || Infinity;
        const nb = parseFloat(b.sortNumber) || Infinity;
        if (na !== nb) return na - nb;
        return (a.name || '').localeCompare(b.name || '');
      });

      // Find folder.jpg in the directory of the first song that has a media path
      let folderImageRef = null;
      if (contentImages) {
        const mediaPath = songs.find(s => s.mediaPath)?.mediaPath;
        if (mediaPath) {
          const dir = path.dirname(mediaPath);
          const candidate = path.join(dir, "folder.jpg");
          if (fs.existsSync(candidate)) folderImageRef = "folder.jpg";
        }
      }

      const parts = [];
      if (folderImageRef) parts.push(`![](${folderImageRef})\n`);
      for (const song of songs) {
        const heading = song.sortNumber ? `${song.sortNumber}. ${song.name}` : song.name;
        let block = `## ${heading}\n`;
        if (song.lang) {
          const langLabel = getLangName(song.lang, uiLang);
          block += `\n_${langLabel}_\n`;
        }
        const publishedUrls = (song.urls || []).filter(u => musicSites.includes((u.label || '').toLowerCase().trim()));
        if (publishedUrls.length) block += '\n' + publishedUrls.map(u => `[${u.label}](${u.href})`).join('  \n') + '\n';
        if (contentDesc && song.desc) block += `\n${song.desc.split("\n").map(l => l.trimEnd() + "  ").join("\n").trimEnd()}\n`;
        const lines = songLyricsLines(song.lyrics, contentLyrics, instrumentalLabel);
        if (lines.length) block += `\n\`\`\`\n${lines.join("\n")}\n\`\`\`\n`;
        if (contentLangs.length > 0) {
          const trs = db.prepare("SELECT * FROM card_translation WHERE cat_cadid=? ORDER BY cat_date ASC").all(song.id);
          for (const tr of trs.filter(tr => contentLangs.includes(tr.cat_lang))) {
            const langLabel = getLangName(tr.cat_lang, uiLang);
            block += `\n### ${translationLabel} - ${langLabel}\n`;
            const trLines = extractLyrics(tr.cat_lyrics, contentLyrics);
            if (trLines.length) block += `\n\`\`\`\n${trLines.join("\n")}\n\`\`\`\n`;
          }
        }
        parts.push(block);
      }

      fs.writeFileSync(filePath, parts.join("\n---\n\n"), "utf8");
      res.json({ success: true, path: filePath });
    } catch (err) {
      console.error("export-md error:", err);
      res.status(500).json({ error: err.message });
    }
  });

  // ── Export AsciiDoc ───────────────────────────────────────────────────────────
  api.post("/export-adoc", async (req, res) => {
    try {
      const { projectIds = [], contentLyrics = "clean", contentLangs = [], contentImages = true, contentDesc = true, filterLyrics = "withLyrics", filterPublished = "all", uiLang = "en", title, translationLabel = "Translation", instrumentalLabel = "Instrumental" } = req.body;
      const musicSites = (readSettings()[`${STORAGE_PREFIX}-music-sites`] || "soundcloud").split(",").map(s => s.trim().toLowerCase()).filter(Boolean);

      let songs;
      if (projectIds.length > 0) {
        const placeholders = projectIds.map(() => "?").join(",");
        const rows = db.prepare(`
          SELECT * FROM card
          WHERE cad_prjid IN (${placeholders})
            AND cad_type = 'song'
          ORDER BY cad_order, cad_name
        `).all(...projectIds);
        songs = rows.map(rowToCard);
      } else {
        songs = db.prepare("SELECT * FROM card WHERE cad_type = 'song' ORDER BY cad_order, cad_name").all().map(rowToCard);
      }

      if (!songs.length) return res.status(400).json({ error: "No songs found" });
      if (filterLyrics === "withLyrics") songs = songs.filter(s => hasRealLyrics(s.lyrics));
      if (filterPublished === "publishedOnly") songs = songs.filter(s => (s.urls || []).some(u => musicSites.includes((u.label || "").toLowerCase().trim())));
      if (!songs.length) return res.status(400).json({ error: "No songs match the filters (lyrics / published)" });

      songs.sort((a, b) => {
        const na = parseFloat(a.sortNumber) || Infinity;
        const nb = parseFloat(b.sortNumber) || Infinity;
        if (na !== nb) return na - nb;
        return (a.name || '').localeCompare(b.name || '');
      });

      let defaultName = "songs";
      let defaultDir = "";
      if (projectIds.length === 1) {
        const proj = db.prepare("SELECT prj_name, prj_path FROM project WHERE prj_id = ?").get(projectIds[0]);
        if (proj) {
          defaultName = proj.prj_name.replace(/[\\/:*?"<>|]/g, "_");
          if (proj.prj_path) defaultDir = proj.prj_path;
        }
      }

      const { canceled, filePath } = await dialog.showSaveDialog(mainWindow, {
        title,
        defaultPath: defaultDir ? path.join(defaultDir, `${defaultName}.adoc`) : `${defaultName}.adoc`,
        filters: [{ name: "AsciiDoc", extensions: ["adoc"] }],
      });
      if (canceled || !filePath) return res.json({ canceled: true });

      let folderImageRef = null;
      if (contentImages) {
        const mediaPath = songs.find(s => s.mediaPath)?.mediaPath;
        if (mediaPath) {
          const candidate = path.join(path.dirname(mediaPath), "folder.jpg");
          if (fs.existsSync(candidate)) folderImageRef = "folder.jpg";
        }
      }

      const parts = [];
      if (folderImageRef) parts.push(`image::${folderImageRef}[]\n`);
      for (const song of songs) {
        const heading = song.sortNumber ? `${song.sortNumber}. ${song.name}` : song.name;
        let block = `== ${heading}\n`;
        if (song.lang) {
          const langLabel = getLangName(song.lang, uiLang);
          block += `\n_${langLabel}_\n`;
        }
        const publishedUrls = (song.urls || []).filter(u => musicSites.includes((u.label || '').toLowerCase().trim()));
        if (publishedUrls.length) block += '\n' + publishedUrls.map(u => `link:${u.href}[${u.label}]`).join(' +\n') + '\n';
        if (contentDesc && song.desc) block += `\n${song.desc.split("\n").map(l => l.trimEnd() + " +").join("\n").trimEnd()}\n`;
        const lines = songLyricsLines(song.lyrics, contentLyrics, instrumentalLabel);
        if (lines.length) block += `\n....\n${lines.join("\n")}\n....\n`;
        if (contentLangs.length > 0) {
          const trs = db.prepare("SELECT * FROM card_translation WHERE cat_cadid=? ORDER BY cat_date ASC").all(song.id);
          for (const tr of trs.filter(tr => contentLangs.includes(tr.cat_lang))) {
            const langLabel = getLangName(tr.cat_lang, uiLang);
            block += `\n=== ${translationLabel} - ${langLabel}\n`;
            const trLines = extractLyrics(tr.cat_lyrics, contentLyrics);
            if (trLines.length) block += `\n....\n${trLines.join("\n")}\n....\n`;
          }
        }
        parts.push(block);
      }

      fs.writeFileSync(filePath, parts.join("\n---\n\n"), "utf8");
      res.json({ success: true, path: filePath });
    } catch (err) {
      console.error("export-adoc error:", err);
      res.status(500).json({ error: err.message });
    }
  });

  // ── LRC Editor integration ────────────────────────────────────────────────────
  function findLrcEditorPath() {
    const candidates = [];
    if (!isDev) {
      // Sibling install: assume both apps are installed under the same parent folder
      candidates.push(path.join(path.dirname(path.dirname(app.getPath('exe'))), 'LRC Editor', 'LRC Editor.exe'));
    }
    const lAppData = process.env.LOCALAPPDATA || path.join(os.homedir(), 'AppData', 'Local');
    const progFiles = process.env.PROGRAMFILES || 'C:\\Program Files';
    candidates.push(
      path.join(lAppData, 'Programs', 'yaiol', 'LRC Editor', 'LRC Editor.exe'),
      path.join(progFiles, 'yaiol', 'LRC Editor', 'LRC Editor.exe'),
    );
    return candidates.find(p => { try { return fs.existsSync(p); } catch { return false; } }) || null;
  }

  api.post("/api/check-lrc", (req, res) => {
    const { mediaPath } = req.body;
    if (!mediaPath) return res.json({ exists: false });
    const lrcPath = mediaPath.replace(/\.[^.]+$/, '.lrc');
    res.json({ exists: fs.existsSync(lrcPath) });
  });

  api.post("/api/check-media-batch", (req, res) => {
    const { paths } = req.body; // [{ id, mediaPath }]
    if (!Array.isArray(paths)) return res.json({ results: {} });
    const results = {};
    for (const { id, mediaPath } of paths) {
      results[id] = mediaPath ? fs.existsSync(mediaPath) : false;
    }
    res.json({ results });
  });


  api.post("/launch-lrc-editor", async (req, res) => {
    const { mediaPath, lyrics } = req.body;
    const lrcEditorPath = findLrcEditorPath();
    if (!lrcEditorPath) { res.json({ ok: false, error: 'not_found' }); return; }

    const lrcPath = mediaPath ? mediaPath.replace(/\.[^.]+$/, '.lrc') : null;
    const handoff = { mediaPath };
    if (lrcPath && fs.existsSync(lrcPath)) {
      handoff.lrcPath = lrcPath;
    } else if (lyrics) {
      handoff.lyrics = lyrics;
    }

    const tmpFile = path.join(os.tmpdir(), `lrc-handoff-${Date.now()}.json`);
    fs.writeFileSync(tmpFile, JSON.stringify(handoff), 'utf8');

    spawn(lrcEditorPath, [`--handoff=${tmpFile}`], { detached: true, stdio: 'ignore' }).unref();
    res.json({ ok: true });
  });

  mark("express route registration");

  findFreePort(4000, (port) => {
    SERVER_PORT = port;
    mark(`free port probe (landed on ${port})`);
    server = api.listen(port, () => {
      mark("express listening");
      console.log(`✅ ${APP_NAME} API on http://localhost:${port} - DB: ${dbPath}`);
      if (callback) callback(port);
    });
  });
}

// ⚠ CLAUDE: Always use this module-level mainWindow for all dialog calls.
// Never use BrowserWindow.fromWebContents(event.sender) - it silently breaks in packaged .exe.
// See root CLAUDE.md "Electron Dialog Windows".
let mainWindow;
function createWindow(port) {
  mainWindow = new BrowserWindow({
    width: 1280, height: 860, minWidth: 800, minHeight: 600,
    // ⚠ CLAUDE: no `backgroundColor` — it painted the window near-black until the first render
    // while this app's default theme is light. No other app sets one; don't reintroduce it.
    title: APP_NAME,
    icon: path.join(__dirname, isDev ? '../public/app.ico' : '../dist/app.ico'),
    webPreferences: { nodeIntegration: false, contextIsolation: true },
    titleBarStyle: process.platform === "darwin" ? "hiddenInset" : "default",
  });
  mainWindow.setMenuBarVisibility(false);
  mark("BrowserWindow created");
  if (isDev) {
    mainWindow.loadURL(`http://localhost:${DEV_PORT}?apiPort=${port}`);
  } else {
    mainWindow.loadFile(path.join(__dirname, "../dist/index.html"), { query: { apiPort: String(port) } });
  }
  mainWindow.webContents.once('did-finish-load', () => { mark("renderer did-finish-load"); dumpStartupTiming({ appName: APP_NAME, isDev, userDataPath: app.getPath("userData") }); });
  mainWindow.webContents.on('did-finish-load', () => mainWindow.setTitle(isDev ? `${APP_NAME} (Dev)` : APP_NAME));
  // ⚠ CLAUDE: Ctrl+Shift+I is dead because Menu.setApplicationMenu(null) removes the default shortcut.
  // This restores it in dev only - do NOT remove.
  if (isDev) {
    mainWindow.webContents.on('before-input-event', (event, input) => {
      if (input.control && input.shift && input.key === 'I') {
        mainWindow.webContents.toggleDevTools();
        event.preventDefault();
      }
    });
  }
  mainWindow.webContents.setWindowOpenHandler(({ url }) => { shell.openExternal(url); return { action: "deny" }; });
  mainWindow.webContents.on("will-navigate", (e, url) => { if (!url.startsWith("file://")) { e.preventDefault(); shell.openExternal(url); } });
  mainWindow.on("closed", () => { mainWindow = null; });
}

// ── Opening a .amlp file from the desktop ─────────────────────────────────────
// Double-clicking a .amlp (the file association is declared in package.json →
// build.fileAssociations) launches the app with the path in argv, or — when a window is
// already up — fires "second-instance" with that instance's argv. Either way the path is
// parked here and the renderer collects it from GET /pending-open, then runs the very same
// import as the Import button (which is why the file is handed over as text, not imported
// here: only the renderer knows which environment is active).
// ⚠ CLAUDE: the single-instance lock is load-bearing, not hygiene — a second instance would
// open its own SQLite handle on the same file and its own Express port, so an association
// double-click would silently run two apps against one database.
let pendingOpenPath = null;

function amlpPathFromArgv(argv) {
  return argv.slice(1).find((a) => !a.startsWith("-") && /\.amlp$/i.test(a)) || null;
}

function focusMainWindow() {
  if (!mainWindow) return;
  if (mainWindow.isMinimized()) mainWindow.restore();
  mainWindow.focus();
}

// macOS hands the file over by event, and it can arrive before the window exists.
app.on("open-file", (event, filePath) => {
  event.preventDefault();
  pendingOpenPath = filePath;
  focusMainWindow();
});

if (!app.requestSingleInstanceLock()) {
  app.quit();
} else {
  app.on("second-instance", (_event, argv) => {
    const file = amlpPathFromArgv(argv);
    if (file) pendingOpenPath = file;
    // Focusing is what makes the running renderer look: it re-checks /pending-open on window
    // focus (the app has no IPC bridge — everything goes through the local HTTP API).
    focusMainWindow();
  });

  pendingOpenPath = amlpPathFromArgv(process.argv);

  app.whenReady().then(() => { mark("app.whenReady"); startServer((port) => { createWindow(port); }); });
  app.on("window-all-closed", () => { if (server) server.close(); if (process.platform !== "darwin") app.quit(); });
  app.on("activate", () => { if (mainWindow === null) createWindow(SERVER_PORT); });
}
