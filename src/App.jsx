// ⚠ CLAUDE - NO HARDCODED UI STRINGS IN THIS FILE.
//   Every string a user can read - JSX text, title=, placeholder=, aria-label=,
//   confirm/alert/setMsg arguments - MUST go through t('keyName'). No exceptions.
//   Workflow: add keys to src/i18n.js EN, then translate, sort and audit every
//   language via the i18n key workflow. Full procedure: see CLAUDE-i18n.md.
//   Never paste translations by hand. The scripts ARE the work.
import { useState, useEffect, useRef, useImperativeHandle, forwardRef } from "react";
import React from "react";
import { createPortal } from "react-dom";
import * as XLSX from "xlsx";
import { useT, getT, LANGUAGES, LANG_LOCALE } from "./i18n-gen";
import yaiolLogo from "./assets/yaiol-logo.svg";
import { SONG_LANGUAGES, getLangName } from "./languages";
import {
  Mic2, Music, Music2, Search, X, Star, Settings, HelpCircle, Plus, Copy, Save, TextCursor, Pencil, Trash2,
  Check, ChevronDown, ChevronUp, Link2, CalendarArrowDown, CalendarArrowUp, ArrowDownAZ, ArrowDownZA,
  Sun, Moon, Plug, RotateCcw, ArrowUpDown, FolderOpen, Folder, FolderPlus, FolderUp,
  Download, LayoutGrid, LayoutList, CheckSquare, Square, Upload, ChevronLeft, ChevronRight,
  Sparkles, ScrollText, ExternalLink, Globe, Tag, MoreVertical, Play, Pause, Pipette, CircleDashed,
  Volume1, Volume2, VolumeX, Shuffle, Repeat, Repeat2, FileSpreadsheet, FilePlus, Languages, Database, AlertTriangle,
  FileText, StickyNote, Disc, AlignLeft, FileMusic, RefreshCw, Wand2, Unlink
} from "lucide-react";
import pkg from '../package.json';
import { checkForUpdate } from './lib/update-check';
import { UpdateBanner } from './lib/ui-update-banner';
import { AppHeader } from './lib/ui-header';
import { GithubIcon } from './lib/ui-icons';
import { Splitter } from './lib/ui-ctl-splitter';
import { NumberField } from './lib/ui-ctl-numberfield';
import { ColorPicker } from './lib/ui-ctl-colorpicker';
import { DatePicker } from './lib/ui-ctl-datepicker';
import { CollapseToggle } from './lib/ui-ctl-collapsetoggle';
import { Menu, MenuItem } from './lib/ui-ctl-menu';
import { Combobox } from './lib/ui-ctl-combobox';
import { useToast } from './lib/ui-fx-toast';
import { Popover } from './lib/ui-ctl-popover';
// Storage namespace - single source: package.json `storagePrefix`. Never hardcode a prefix.
const STORAGE_PREFIX = pkg.storagePrefix;

// ─── App identity - single source of truth for the app name ──────────────────
const APP_NAME    = pkg.productName;
const APP_VERSION = pkg.version;

// Help page URL - the id comes from package.json `name` (the canonical app id =
// folder = apps.yaiol.com slug). At runtime the lang segment is swapped to the current
// UI language; the help site falls back to EN for languages it does not publish, so any
// code is safe to send.
// CLAUDE: NEVER hardcode the slug here. A typed literal silently drifts from the id and
// 404s the help button - it did, unnoticed, in two shipped apps until 2026-07-28.
const HELP_URL = `https://apps.yaiol.com/en/p/${pkg.name}/help/`;
// GitHub source - owner is constant (yaiol); repo name is the app id (pkg.name).
const GITHUB_URL = `https://github.com/yaiol/${pkg.name}`;
const UNLINKED_KEY  = "__unlinked__";
const UNTAGGED_KEY  = "__untagged__";

const getApiPort = () => {
  const params = new URLSearchParams(window.location.search);
  return params.get('apiPort') || '4000';
};
const API = `http://localhost:${getApiPort()}`;

// ─── Dialog layout constants — local, non-CSS values (textarea rows, tab heights) ────
const UIDLG = {
  textareaRowsDescNote:       13,            // textarea rows for desc and note (all card types)
  textareaRowsStyle:          11,            // textarea rows for style (all card types)
  textareaRowsLyrics:         11,            // textarea rows for lyrics (song)
  textareaRowsTranslation:    9,             // textarea rows for translation content
  textareaRowsEditBonus:      2,             // extra rows added when editing an existing card
  tabMinHeightNew:            290,           // min height of tab content area (new card)
  tabMinHeightEdit:           330,           // min height of tab content area (edit card)
};

// Color-picker default = the live theme accent, read from CSS (single source of truth).
const accentDefault = () => getComputedStyle(document.documentElement).getPropertyValue('--accent').trim();

const TYPE_LABEL_KEYS = {
  note:     "lblDlgSettingsColorNote",
  music:    "lblDlgSettingsColorMusic",
  vocal: "lblDlgSettingsColorVocal",
  SongStateUndefined:   "lblDlgSettingsColorSongUndefined",
  SongStateCreated:   "lblDlgSettingsColorSongCreated",
  SongStatePublished:   "lblDlgSettingsColorSongPublished",
};

const DEFAULT_TYPE_COLORS = {
  note:         "#22c55e",
  music:        "#ff6eb4",
  vocal:        "#f0c040",
  SongStateCreated:       "#3f61d2",
  SongStateUndefined:       "#60a5fa",
  SongStatePublished:       "#645492",
};

const TAG_PALETTE = ["#4dc8c8","#ff6eb4","#a78bfa","#f0c040","#5dba6f","#e06060","#60a5fa","#fb923c","#34d399","#f87171","#38bdf8","#e879f9","#fbbf24","#a3e635","#7de8e8","#ff9ecf"];

function hexToRgba(hex, alpha) {
  const r = parseInt(hex.slice(1,3),16), g = parseInt(hex.slice(3,5),16), b = parseInt(hex.slice(5,7),16);
  return `rgba(${r},${g},${b},${alpha})`;
}

function buildTypes(colors) {
  return {
    note:        { label: "Note",     Icon: StickyNote, accent: colors.note,    accentDim: hexToRgba(colors.note,        0.13) },
    music:       { label: "Music",    Icon: Music,  accent: colors.music,       accentDim: hexToRgba(colors.music,       0.13) },
    vocal:       { label: "Vocal",    Icon: Mic2,   accent: colors.vocal,       accentDim: hexToRgba(colors.vocal,       0.13) },
    SongStateCreated: { label: "Song",     Icon: Music2, accent: colors.SongStateCreated, accentDim: hexToRgba(colors.SongStateCreated, 0.13) },
    SongStateUndefined: { label: "Song",     Icon: Music2, accent: colors.SongStateUndefined,  accentDim: hexToRgba(colors.SongStateUndefined,  0.13) },
    SongStatePublished: { label: "Song",     Icon: Music2, accent: colors.SongStatePublished,  accentDim: hexToRgba(colors.SongStatePublished,  0.13) },
  };
}

function getSongState(card, types, txtSettingsCardsAiSites = "udio, suno, producer, tunee", txtSettingsCardsMusicSites = "soundcloud") {
  const labels = (card.urls || []).map(u => (u.label || "").toLowerCase().trim());
  const musicList = txtSettingsCardsMusicSites.split(",").map(s => s.trim().toLowerCase()).filter(Boolean);
  const aiList    = txtSettingsCardsAiSites.split(",").map(s => s.trim().toLowerCase()).filter(Boolean);
  if (labels.some(l => musicList.includes(l))) return types.SongStatePublished;
  if (labels.some(l => aiList.includes(l)))    return types.SongStateCreated;
  return types.SongStateUndefined;
}

// ⚠ CLAUDE: mirrors hasRealLyrics in electron/main.mjs (the server owns export filtering) — keep the two in step.
function hasRealLyrics(raw) {
  return (raw || "").split("\n").some(line => {
    const t = line.trim();
    return t && !/^\[.*\]$/.test(t) && !/^\(.*\)$/.test(t);
  });
}

// Static fallback used outside component scope (tag palettes etc.)
const TYPES = buildTypes(DEFAULT_TYPE_COLORS);

const PROJECT_COLORS = ["#7c6fff","#ff6eb4","#4dc8c8","#a78bfa","#f0c040","#5dba6f","#e06060","#ff9860"];

function generateId() {
  return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, (c) => {
    const r = Math.random() * 16 | 0;
    return (c === 'x' ? r : (r & 0x3 | 0x8)).toString(16);
  });
}

const tagColorOverrides = {};
function getTagColor(tag) {
  if (tagColorOverrides[tag]) return tagColorOverrides[tag];
  let h = 0;
  for (let i = 0; i < tag.length; i++) { h = (h << 5) - h + tag.charCodeAt(i); h |= 0; }
  return TAG_PALETTE[Math.abs(h) % TAG_PALETTE.length];
}

async function apiFetch(path, opts = {}) {
  const res = await fetch(`${API}${path}`, { headers: { "Content-Type": "application/json" }, ...opts });
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}

async function importFromJson(file, apiFetch, setCards, setProjects, showToast, t, activeEnvId, setEnvironments, setImportErrors) {
  try {
    const text = await file.text();
    const snapshot = JSON.parse(text);
    const hasCards = snapshot.cards || snapshot.projects;
    const hasAi      = snapshot.aiPresets || snapshot.aiTemplates;
    if (!hasCards && !hasAi) {
      showToast("❌ " + t("tstAppImportJSONJsonInvalid"));
      return;
    }
    const isBackup = snapshot.type === "backup";
    const body = isBackup ? { ...snapshot } : { ...snapshot, targetEnvId: activeEnvId };
    const result = await apiFetch("/import-json", {
      method: "POST",
      body: JSON.stringify(body),
    });
    setCards(result.cards);
    setProjects(result.projects);
    if (result.results?.validationErrors?.length) setImportErrors(result.results.validationErrors);
    if (isBackup) {
      const freshEnvs = await apiFetch("/environments");
      setEnvironments(freshEnvs || []);
      showToast("✅ " + t("tstAppImportJSONBackupRestored"));
      return;
    }
    const r = result.results;
    const parts = [];
    if (r.cardsCreated)   parts.push(`${r.cardsCreated} ${t("msgAppImportJSONCardsCreated")}`);
    if (r.cardsUpdated)   parts.push(`${r.cardsUpdated} ${t("msgAppImportJSONCardsUpdated")}`);
    if (r.cardsSkipped)   parts.push(`${r.cardsSkipped} ${t("msgAppImportJSONCardsUnchanged")}`);
    if (r.projectsCreated)  parts.push(`${r.projectsCreated} ${t("msgAppImportJSONProjectsCreated")}`);
    if (r.projectsUpdated)  parts.push(`${r.projectsUpdated} ${t("msgAppImportJSONProjectsUpdated")}`);
    if (r.songLinksAdded)   parts.push(`${r.songLinksAdded} ${t("msgAppImportJSONSongLinksAdded")}`);
    if (r.songUrlsAdded)    parts.push(`${r.songUrlsAdded} ${t("msgAppImportJSONSongUrlsAdded")}`);
    if (hasAi && !hasCards) { showToast("✅ " + t("tstAppImportJSONAiImported")); return; }
    showToast(`✅ JSON: ${parts.length ? parts.join(", ") : t("tstAppImportJSONNothing")}`);
  } catch(err) {
    showToast("❌ " + t("tstAppImportJSONJsonFailed") + ": " + err.message);
  }
}

async function importFromAmlp(file, apiFetch, setCards, setProjects, showToast, t, activeEnvId, targetProjectId) {
  return importAmlpText(await file.text(), apiFetch, setCards, setProjects, showToast, t, activeEnvId, targetProjectId);
}

// The one .amlp import path — used by the Import button (a picked File), by a .amlp opened from
// the desktop (text handed over by the main process) and by a file dropped on the window. Same
// endpoint, same toasts, same environment rules, so the three entries can never drift.
// targetProjectId = the project the user is currently in; the tracks are added to it. Null (no
// project open, or several selected) means the LP brings its own project, named after its TITLE.
async function importAmlpText(text, apiFetch, setCards, setProjects, showToast, t, activeEnvId, targetProjectId) {
  try {
    const result = await apiFetch("/import-amlp", {
      method: "POST",
      body: JSON.stringify({ text, targetEnvId: activeEnvId, targetProjectId: targetProjectId || null }),
    });
    if (result.error) {
      const msg = result.error === "globalEnv" ? t("tstAppImportAmlpGlobalEnv") : t("tstAppImportAmlpFailed");
      showToast("❌ " + msg, 6000);
      return;
    }
    setCards(result.cards);
    setProjects(result.projects);
    const r = result.results;
    showToast(`✅ ${r.projectName}: ${r.cardsCreated} ${t("msgAppImportJSONCardsCreated")}`);
  } catch (err) {
    // Never surface the raw response body (a 404/500 can be a full HTML page) — log it, toast a clean line.
    console.error(".amlp import failed:", err.message);
    showToast("❌ " + t("tstAppImportAmlpFailed"), 6000);
  }
}

// A Suno link anywhere in the clipboard — either the share stub (suno.com/s/<code>) or the
// canonical song URL. Matched loosely because a copied link often arrives with text around it.
const SUNO_LINK_RE = /https?:\/\/(?:www\.)?suno\.com\/(?:s\/[A-Za-z0-9_-]+|song\/[0-9a-fA-F-]{36})/;

// Expand a Suno share stub (suno.com/s/<code>) into the canonical suno.com/song/<uuid>.
// Anything else — a canonical link already, an unreachable Suno — comes back unchanged, so
// callers can always use the return value.
async function resolveSunoLink(href) {
  if (!/^https?:\/\/(?:www\.)?suno\.com\/s\//i.test(href)) return href;
  try {
    const resolved = await apiFetch("/resolve-suno-share", { method: "POST", body: JSON.stringify({ url: href }) });
    return resolved.url || href;
  } catch (err) {
    console.error("Suno share resolve failed:", err.message);
    return href;
  }
}

// Build a draft song card out of a Suno link sitting in the clipboard — name, sort number,
// style, lyrics, date and the link itself. Returns null whenever the clipboard holds no Suno
// link or Suno can't be reached, so the caller just opens an empty card as before.
async function sunoDraftFromClipboard() {
  try {
    const clip = await apiFetch("/clipboard");
    const found = (clip.text || "").match(SUNO_LINK_RE)?.[0];
    if (!found) return null;
    const href = await resolveSunoLink(found);
    if (!href.includes("/song/")) return null;   // share stub that never resolved
    const data = await apiFetch("/fetch-suno-lyrics", { method: "POST", body: JSON.stringify({ url: href }) });
    // "10-Sanctus" → sort number 10 + name "Sanctus"; a title without that prefix is the name.
    const numbered = (data.title || "").match(/^\s*(\d{1,4})\s*[-–—]\s*(.+)$/);
    const ms = data.date ? Date.parse(data.date) : NaN;
    return {
      type: "song",
      name: (numbered ? numbered[2] : data.title || "").trim(),
      sortNumber: numbered ? String(parseInt(numbered[1], 10)) : "",
      style: data.style || "",
      lyrics: data.lyrics || "",
      date: Number.isNaN(ms) ? Date.now() : ms,
      urls: [{ id: `suno-${Date.now()}`, label: "suno", href }],
    };
  } catch (err) {
    console.error("Suno clipboard draft failed:", err.message);
    return null;
  }
}

function cleanLyrics(text) {
  return (text || "").replace(/\r/g, "").replace(/\[.*?\]/g, "").replace(/\(.*?\)/g, "").replace(/\n +/g, "\n").replace(/\n{3,}/g, "\n\n").replace(/^\n+/, "").replace(/\n+$/, "");
}

async function exportToExcel(cards, projects, projectName = null, title = "Save Document", apiBase = "") {
  const wb = XLSX.utils.book_new();
  const date = new Date().toISOString().slice(0, 10);

  // ── Sheet 1: Cards ─────────────────────────────────────────────────────────
  const cardRows = cards.map(p => ({
    "Card Type": p.type.charAt(0).toUpperCase() + p.type.slice(1),
    "Card Name": p.name,
    "Card Version": p.version || 1,
    "Card Sort": p.sortNumber ?? "",
    "Style":    p.style,
    "Lyrics":   p.lyrics,
    "Tags":     p.tags.join(", "),
    "Favorite": p.favorite ? "Yes" : "No",
    "Created":  new Date(p.date).toLocaleDateString(),
  }));
  const ws1 = XLSX.utils.json_to_sheet(cardRows);
  ws1["!cols"] = [{ wch: 12 }, { wch: 30 }, { wch: 8 }, { wch: 8 }, { wch: 60 }, { wch: 80 }, { wch: 25 }, { wch: 10 }, { wch: 14 }];
  XLSX.utils.book_append_sheet(wb, ws1, "Card");

  // ── Sheet 2: Projects ────────────────────────────────────────────────────────
  const projectRows = projects.flatMap(proj => {
    const parentProj = proj.parentId ? projects.find(p => p.id === proj.parentId) : null;
    const base = { "Project Name": proj.name, "Project Version": proj.version || 1, "Parent Folder": parentProj ? parentProj.name : "", "Parent Folder Version": parentProj ? (parentProj.version || 1) : "", "Created": new Date(proj.date).toLocaleDateString() };
    return proj.cardIds.length === 0
      ? [{ ...base, "Card Name": "", "Card Version": "", "Card Type": "" }]
      : proj.cardIds.map(pid => {
          const p = cards.find(x => x.id === pid);
          return { ...base, "Card Name": p ? p.name : "(deleted)", "Card Version": p ? (p.version || 1) : "", "Card Type": p ? p.type : "" };
        });
  });
  const ws2 = XLSX.utils.json_to_sheet(projectRows);
  ws2["!cols"] = [{ wch: 25 }, { wch: 8 }, { wch: 25 }, { wch: 8 }, { wch: 14 }, { wch: 30 }, { wch: 14 }, { wch: 12 }];
  XLSX.utils.book_append_sheet(wb, ws2, "Project");

  // ── Sheet 3: Song Links ───────────────────────────────────────────────────────
  const songs = cards.filter(p => p.type === "song" && p.linkedCards && p.linkedCards.length > 0);
  if (songs.length > 0) {
    const songRows = songs.flatMap(song =>
      song.linkedCards.map(lid => {
        const linked = cards.find(x => x.id === lid);
        return { "Song Name": song.name, "Song Version": song.version || 1, "Card Name": linked ? linked.name : "(deleted)", "Card Version": linked ? (linked.version || 1) : "", "Card Type": linked ? linked.type : "" };
      })
    );
    const ws3 = XLSX.utils.json_to_sheet(songRows);
    ws3["!cols"] = [{ wch: 30 }, { wch: 14 }, { wch: 30 }, { wch: 14 }, { wch: 12 }];
    XLSX.utils.book_append_sheet(wb, ws3, "Song Links");
  }

  // ── Sheet 4: URLs ─────────────────────────────────────────────────────────────
  const cardsWithUrls = cards.filter(p => p.urls && p.urls.length > 0);
  if (cardsWithUrls.length > 0) {
    const urlRows = cardsWithUrls.flatMap(p =>
      p.urls.map(u => ({ "Type": p.type, "Card Name": p.name, "Card Version": p.version || 1, "Label": u.label, "URL": u.href }))
    );
    const ws4 = XLSX.utils.json_to_sheet(urlRows);
    ws4["!cols"] = [{ wch: 10 }, { wch: 30 }, { wch: 14 }, { wch: 15 }, { wch: 60 }];
    XLSX.utils.book_append_sheet(wb, ws4, "URLs");
  }

  const defaultPath = `${APP_NAME}${projectName ? "-" + projectName : ""}-${date}.xlsx`;
  const base64 = XLSX.write(wb, { bookType: "xlsx", type: "base64" });
  await fetch(`${apiBase}/save-excel`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ data: base64, title, defaultPath }),
  });
}

// Save a JS object as pretty JSON through the Electron save dialog.
// ⚠ CLAUDE: never save JSON with a renderer <a download> blob click — it
// silently no-ops in Electron (that was the long-standing "Export does nothing"
// bug). All JSON export routes through the server /save-json dialog, mirroring
// how /save-excel works.
async function saveJsonFile(snapshot, defaultPath, title = "Save JSON") {
  return apiFetch("/save-json", {
    method: "POST",
    body: JSON.stringify({ data: JSON.stringify(snapshot, null, 2), title, defaultPath }),
  });
}

async function exportToJson(cards, projects, projectName = null, title = "Save JSON") {
  const translationsMap = {};
  await Promise.all(cards.map(async (p) => {
    try {
      const trs = await fetch(`${API}/translations/${p.id}`).then(r => r.json());
      if (Array.isArray(trs) && trs.length) translationsMap[p.id] = trs.map(tr => ({ lang: tr.lang, name: tr.name || '', content: tr.content }));
    } catch (_) {}
  }));
  const snapshot = {
    version: APP_VERSION,
    date: new Date().toISOString(),
    cards: cards.map(p => ({
      id: p.id, type: p.type, name: p.name, version: p.version || 1, order: p.sortNumber ?? null, desc: p.desc || '', note: p.note || '', style: p.style, lyrics: p.lyrics || '',
      tags: p.tags, favorite: p.favorite, date: p.date || null,
      lang: p.lang || '',
      aiTweaks: p.aiTweaks || {},
      project: p.project || null,
      cards: p.linkedCards || [],
      urls: p.urls || [],
      media: p.mediaPath || '',
      ...(translationsMap[p.id] ? { translations: translationsMap[p.id] } : {}),
    })),
    projects: projects.map(proj => ({
      id: proj.id, name: proj.name, version: proj.version || 1, color: proj.color,
      date: proj.date || null, parent: proj.parentId || null, cards: proj.cardIds,
      urls: proj.urls || [],
      path: proj.path || '',
      finalized: proj.finalized || false,
    })),
  };
  return saveJsonFile(snapshot, `${APP_NAME}-${projectName ? projectName : "Cards"}-${new Date().toISOString().slice(0, 10)}.json`, title);
}

async function exportAiToJson(title = "Save JSON") {
  const [presetsRaw, templatesRaw] = await Promise.all([
    fetch(`${API}/ai-presets`).then(r => r.json()),
    fetch(`${API}/ai-templates`).then(r => r.json()),
  ]);
  const snapshot = {
    version: APP_VERSION,
    date: new Date().toISOString(),
    aiPresets:   presetsRaw  .filter(p => !p.isDefault).map(p => ({ id: p.id, task: p.task, block: p.block, name: p.name, content: p.content })),
    aiTemplates: templatesRaw.filter(t => !t.isDefault).map(t => ({ id: t.id, task: t.task, name: t.name, blocks: t.blocks.map(b => ({ block: b.block, presetId: b.presetId, enabled: b.enabled })) })),
  };
  return saveJsonFile(snapshot, `${APP_NAME}-AI-${new Date().toISOString().slice(0, 10)}.json`, title);
}

async function exportBackupToJson(cards, projects, environments, title = "Save JSON") {
  const translationsMap = {};
  await Promise.all(cards.map(async (p) => {
    try {
      const trs = await fetch(`${API}/translations/${p.id}`).then(r => r.json());
      if (Array.isArray(trs) && trs.length) translationsMap[p.id] = trs.map(tr => ({ lang: tr.lang, name: tr.name || '', content: tr.content }));
    } catch (_) {}
  }));
  const data = {
    type: "backup",
    version: APP_VERSION,
    date: new Date().toISOString(),
    environments: environments.map(e => ({ id: e.id, name: e.name, order: e.order, isGlobal: e.isGlobal })),
    cards: cards.map(p => ({
      id: p.id, type: p.type, name: p.name, version: p.version || 1, order: p.sortNumber ?? null, desc: p.desc || '', note: p.note || '', style: p.style, lyrics: p.lyrics || '',
      tags: p.tags, favorite: p.favorite, date: p.date || null,
      lang: p.lang || '',
      aiTweaks: p.aiTweaks || {},
      cards: p.linkedCards || [],
      urls: p.urls || [],
      media: p.mediaPath || '',
      env: p.env || '',
      ...(translationsMap[p.id] ? { translations: translationsMap[p.id] } : {}),
    })),
    projects: projects.map(proj => ({
      id: proj.id, name: proj.name, version: proj.version || 1, color: proj.color,
      date: proj.date || null, parent: proj.parentId || null, cards: proj.cardIds, env: proj.env || '',
      urls: proj.urls || [],
      path: proj.path || '',
      finalized: proj.finalized || false,
    })),
  };
  return saveJsonFile(data, `${APP_NAME}-Backup-${new Date().toISOString().slice(0, 10)}.json`, title);
}

export default function App() {
  const jsonRef         = useRef(null);
  const exportBtnRef    = useRef(null);
  const envBtnRef       = useRef(null);
  const [exportMenuOpen, setExportMenuOpen] = useState(false);
  const [cards, setCards]               = useState([]);
  const [lrcExists, setLrcExists]         = useState({});
  const [mediaExists, setMediaExists]     = useState({});
  const [projects, setProjects]           = useState([]);
  const [loading, setLoading]             = useState(true);
  const [apiError, setApiError]           = useState(null);
  const [activeType, setActiveType]         = useState(null);
  const [activeProject, setActiveProject]   = useState(null); // null | UNLINKED_KEY
  const [activeProjectIds, setActiveProjectIds] = useState([]); // regular folder multi-selection
  // "The folder you are in" — the one selected project, or null when none (or several) are
  // selected. Every .amlp import targets it: the tracks are added to that project instead of the
  // LP creating its own.
  const openProjectId = activeProjectIds.length === 1 ? activeProjectIds[0] : null;
  const [search, setSearch]               = useState("");
  const [translationsIndex, setTranslationsIndex] = useState({});
  const [selectedTags, setSelectedTags]   = useState([]);
  const [showFavOnly, setShowFavOnly]         = useState(false);
  const [showWarningNoTagged, setShowWarningNoTagged]       = useState(false);
  const [showWarningNoAiLink, setShowWarningNoAiLink]     = useState(false);
  const [showWarningNoMedia, setShowWarningNoMedia]         = useState(false);
  const [showWarningNoLrc, setShowWarningNoLrc]             = useState(false);
  const [showWarningNoPublished, setShowWarningNoPublished] = useState(false);
  const [modalOpen, setModalOpen]         = useState(false);
  const [editingCard, setEditingCard] = useState(null);
  // Unsaved card the New button pre-fills from a Suno link in the clipboard. Kept apart from
  // editingCard on purpose: handleSave branches on editingCard, and a draft has no id yet.
  const [draftCard, setDraftCard]         = useState(null);
  const [draftFetching, setDraftFetching] = useState(false);
  const [copiedId, setCopiedId]           = useState(null);
  const [deleteConfirm, setDeleteConfirm] = useState(null);
  const [settingsOpen, setSettingsOpen]   = useState(false);
  const [tabSettingsActive, setTabSettingsActive]     = useState("display");
  const [projectModalOpen, setProjectModalOpen] = useState(false);
  const [docCreateOpen, setDocCreateOpen]       = useState(false);
  const [docCreateSettings, setDocCreateSettings] = useState(() => {
    try { const s = JSON.parse(localStorage.getItem(`${STORAGE_PREFIX}-doc-create-settings`)); if (s) return { contentLyrics: "clean", contentImages: "with", contentDesc: "with", filterLyrics: "withLyrics", filterPublished: "all", contentLangs: [], ...s }; } catch {}
    return { contentLyrics: "clean", contentImages: "with", contentDesc: "with", filterLyrics: "withLyrics", filterPublished: "all", contentLangs: [] };
  });
  useEffect(() => { localStorage.setItem(`${STORAGE_PREFIX}-doc-create-settings`, JSON.stringify(docCreateSettings)); }, [docCreateSettings]);
  const [availableTranslationLangs, setAvailableTranslationLangs] = useState([]);
  const [editingProject, setEditingProject]     = useState(null);
  const [newFolderParentId, setNewFolderParentId] = useState(null);
  const [expandedProjects, setExpandedProjects] = useState(new Set());
  const [draggingProjectId, setDraggingProjectId] = useState(null);
  const [dragOverFolderId, setDragOverFolderId] = useState(null);
  const [rootDropOver, setRootDropOver] = useState(false);
  const [deleteProjectConfirm, setDeleteProjectConfirm] = useState(null);
  const [deleteProjectCards, setDeleteProjectCards]     = useState(false);
  const [importFolderPending, setImportFolderPending] = useState(null); // { projectId, path }
  const [sidebarOpen, setSidebarOpen]     = useState(true);
  const [sidebarWidth, setSidebarWidth]   = useState(() => parseInt(localStorage.getItem(`${STORAGE_PREFIX}-sidebar-width`) || "220"));
  const isResizingRef = useRef(false);
  const resizeStartX  = useRef(0);
  const resizeStartW  = useRef(0);
  const [linkPanelWidth, setLinkPanelWidth] = useState(() => parseInt(localStorage.getItem(`${STORAGE_PREFIX}-link-panel-width`) || "320"));
  const [linkMVScope, setLinkMVScope]       = useState("local"); // "local" | "global"
  const [linkMVSearch, setLinkMVSearch]     = useState("");
  const [mediaLinkFolder, setMediaLinkFolder] = useState("");
  const [mediaLinkFiles,  setMediaLinkFiles]  = useState([]);
  const [hideLinkedMedia, setHideLinkedMedia] = useState(() => localStorage.getItem(`${STORAGE_PREFIX}-hide-linked-media`) === "1");
  const [renameMediaOnSort, setRenameMediaOnSort] = useState(() => localStorage.getItem(`${STORAGE_PREFIX}-sort-rename-media`) === "1");
  const [dragMediaFile,   setDragMediaFile]   = useState(null); // { name, path }
  const isLinkResizingRef = useRef(false);
  const linkResizeStartX  = useRef(0);
  const linkResizeStartW  = useRef(0);
  const [projectSort, setProjectSort]     = useState(() => localStorage.getItem(`${STORAGE_PREFIX}-proj-sort`) || "date");
  const [projectSortDir, setProjectSortDir] = useState(() => localStorage.getItem(`${STORAGE_PREFIX}-proj-sort-dir`) || "desc");
  const [draggingId, setDraggingId]       = useState(null);
  const [selectedIds, setSelectedIds]     = useState([]);
  const [bulkDeleteConfirm, setBulkDeleteConfirm] = useState(false);
  const [unlinkConfirm, setUnlinkConfirm] = useState(false);
  const [bulkTagOpen, setBulkTagOpen]         = useState(false);
  const [importErrors, setImportErrors]       = useState(null);
  const [bulkTagInput, setBulkTagInput]       = useState("");
  const [viewMode, setViewMode]           = useState(() => localStorage.getItem(`${STORAGE_PREFIX}-view`) || "grid");
  const [themeName, setThemeName]         = useState(() => localStorage.getItem(`${STORAGE_PREFIX}-theme`) || "light");
  const [sortBy, setSortBy]               = useState(() => localStorage.getItem(`${STORAGE_PREFIX}-sort`) || "date");
  const [sortDir, setSortDir]             = useState(() => localStorage.getItem(`${STORAGE_PREFIX}-sort-dir`) || "desc");
  const [clrSettingsCardsColors, setClrSettingsCardsColors]       = useState(() => {
    try {
      const s = JSON.parse(localStorage.getItem(`${STORAGE_PREFIX}-type-colors`));
      if (!s) return DEFAULT_TYPE_COLORS;
      // Migrate vocalist → vocal for users upgrading from older versions
      if (s.vocalist !== undefined && s.vocal === undefined) { s.vocal = s.vocalist; delete s.vocalist; }
      return { ...DEFAULT_TYPE_COLORS, ...s };
    }
    catch { return DEFAULT_TYPE_COLORS; }
  });
  const [langKey, setLangKey]             = useState(() => {
    const saved = localStorage.getItem(`${STORAGE_PREFIX}-lang`);
    if (saved) return saved;
    const browser = (navigator.language || navigator.languages?.[0] || "en").split("-")[0].toLowerCase();
    return LANGUAGES.some(l => l.key === browser) ? browser : "en";
  });
  const [txtSettingsAiApiKey, setTxtSettingsAiApiKey]         = useState(() => localStorage.getItem(`${STORAGE_PREFIX}-gemini-key`) || "");
  const [clrSettingsTags, setClrSettingsTags]         = useState(() => { try { return JSON.parse(localStorage.getItem(`${STORAGE_PREFIX}-tag-colors`)) || {}; } catch { return {}; } });
  const [txtSettingsAiLimitsVocal, setTxtSettingsAiLimitsVocal] = useState(() => parseInt(localStorage.getItem(`${STORAGE_PREFIX}-ai-limit-vocal`) || "300"));
  const [txtSettingsAiLimitsMusic, setTxtSettingsAiLimitsMusic]       = useState(() => parseInt(localStorage.getItem(`${STORAGE_PREFIX}-ai-limit-music`) || "700"));
  const [txtSettingsAiStyleThreshold, setTxtSettingsAiStyleThreshold]         = useState(() => parseInt(localStorage.getItem(`${STORAGE_PREFIX}-ai-warn-limit`) || "1000"));
  const [txtSettingsAiLyricsThreshold, setTxtSettingsAiLyricsThreshold] = useState(() => parseInt(localStorage.getItem(`${STORAGE_PREFIX}-ai-lyric-threshold`) || "5000"));
  const [txtSettingsAiStyleWordThreshold, setTxtSettingsAiStyleWordThreshold] = useState(() => parseInt(localStorage.getItem(`${STORAGE_PREFIX}-ai-style-word-threshold`) || "60"));
  const [tglSettingsAiTemplatesMode, setTglSettingsAiTemplatesMode] = useState(() => localStorage.getItem(`${STORAGE_PREFIX}-ai-expert-mode`) === "1");
  const [aiDefaultTemplates, setAiDefaultTemplates] = useState({
    music:  localStorage.getItem(`${STORAGE_PREFIX}-ai-default-template-music`)  || null,
    vocal:  localStorage.getItem(`${STORAGE_PREFIX}-ai-default-template-vocal`)  || null,
    lyrics: localStorage.getItem(`${STORAGE_PREFIX}-ai-default-template-lyrics`) || null,
  });
  const [txtSettingsCardsAiSites, setTxtSettingsCardsAiSites]                 = useState(() => localStorage.getItem(`${STORAGE_PREFIX}-ai-sites`) || "udio, suno, producer, tunee");
  const [txtSettingsCardsMusicSites, setTxtSettingsCardsMusicSites]           = useState(() => localStorage.getItem(`${STORAGE_PREFIX}-music-sites`) || "soundcloud");
  const [playerMedia, setPlayerMedia]         = useState(null);
  const [playlist, setPlaylist]               = useState([]);
  const [playlistIndex, setPlaylistIndex]     = useState(0);
  const globalPlayerRef                       = useRef(null);
  const [globalPlaying, setGlobalPlaying]     = useState(false);
  const t = useT(langKey);

  const [environments, setEnvironments]             = useState([]);
  const [activeEnvId, setActiveEnvId]               = useState(() => localStorage.getItem(`${STORAGE_PREFIX}-active-env`) || "");
  useEffect(() => localStorage.setItem(`${STORAGE_PREFIX}-active-env`, activeEnvId), [activeEnvId]);
  const [envPopoverOpen, setEnvPopoverOpen]         = useState(false);
  const [renamingEnvId, setRenamingEnvId]           = useState(null);
  const [renameEnvValue, setRenameEnvValue]         = useState("");
  const [newEnvInput, setNewEnvInput]               = useState(null);
  const [moveEnvMode, setMoveEnvMode]               = useState(null); // null | "folder" | "cards"
  const [moveTargetEnvId, setMoveTargetEnvId]       = useState("");
  const [moveTargetProjectId, setMoveTargetProjectId] = useState("");
  const [moveCardWarning, setMoveCardWarning]   = useState(null); // [{id,name}] or null
  const [batchTranslateOpen, setBatchTranslateOpen]           = useState(false);
  const [batchTranslateSource, setBatchTranslateSource]     = useState(null); // "project" | "selection"
  const [batchTranslateLang, setBatchTranslateLang]           = useState("");
  const [batchTranslateLangSearch, setBatchTranslateLangSearch] = useState("");
  const [batchTranslateLangOpen, setBatchTranslateLangOpen]   = useState(false);
  const [batchTranslateRunning, setBatchTranslateRunning]     = useState(false);
  const [batchTranslateProgress, setBatchTranslateProgress]   = useState(null);
  const batchTranslateCancelRef = useRef(false);
  const batchTranslateDataRef = useRef(null);

  const [updateInfo, setUpdateInfo]         = useState(null);
  useEffect(() => {
    checkForUpdate({ appId: pkg.name, alias: STORAGE_PREFIX, currentVersion: APP_VERSION })
      .then(u => { if (u) setUpdateInfo(u); });
  }, []);

  // A .amlp opened from the desktop — the main process parks the file and focuses this window;
  // we collect it here and run the ordinary import. Checked on launch AND on every window focus,
  // because a double-click while the app is already running arrives as a focus, not a start.
  // /pending-open hands the file over once and forgets it, so re-checking is free.
  useEffect(() => {
    if (!activeEnvId) return;   // wait for the environment list — the import needs a target
    const collect = async () => {
      try {
        const pending = await apiFetch("/pending-open");
        if (pending?.text) await importAmlpText(pending.text, apiFetch, setCards, setProjects, showToast, t, activeEnvId, openProjectId);
      } catch (err) {
        console.error("pending .amlp check failed:", err.message);
      }
    };
    collect();
    window.addEventListener("focus", collect);
    return () => window.removeEventListener("focus", collect);
  }, [activeEnvId, openProjectId]); // eslint-disable-line react-hooks/exhaustive-deps

  // Dropping a .amlp anywhere on the window imports it, into the project you are in.
  // ⚠ CLAUDE: the window-level preventDefault is load-bearing, not tidiness — without it Electron
  // NAVIGATES to a dropped file (a file:// URL sails past the will-navigate guard, which only
  // redirects non-file:// links to the browser) and the app is replaced by the file's contents.
  useEffect(() => {
    const swallow = (e) => { if (e.dataTransfer?.types?.includes("Files")) e.preventDefault(); };
    window.addEventListener("dragover", swallow);
    window.addEventListener("drop", swallow);
    return () => { window.removeEventListener("dragover", swallow); window.removeEventListener("drop", swallow); };
  }, []);

  // Sync tag color overrides every render so CardItems always see current values
  Object.keys(tagColorOverrides).forEach(k => delete tagColorOverrides[k]);
  Object.assign(tagColorOverrides, clrSettingsTags);

  const _rawTypes = buildTypes(clrSettingsCardsColors);
  const types = {
    note:        { ..._rawTypes.note,        label: t("lblDlgSettingsColorNote") },
    music:       { ..._rawTypes.music,       label: t("lblAppTypeMusic") },
    vocal:       { ..._rawTypes.vocal,       label: t("lblAppTypeVocal") },
    SongStateUndefined:  { ..._rawTypes.SongStateUndefined,  label: t("lblAppTypeSong") },
    SongStateCreated: { ..._rawTypes.SongStateCreated, label: t("lblAppTypeSong") },
    SongStatePublished: { ..._rawTypes.SongStatePublished, label: t("lblAppTypeSong") },
  };
  useEffect(() => { localStorage.setItem(`${STORAGE_PREFIX}-proj-sort`, projectSort); apiFetch("/settings", { method: "POST", body: JSON.stringify({ [`${STORAGE_PREFIX}-proj-sort`]: projectSort }) }); }, [projectSort]);
  useEffect(() => { localStorage.setItem(`${STORAGE_PREFIX}-proj-sort-dir`, projectSortDir); apiFetch("/settings", { method: "POST", body: JSON.stringify({ [`${STORAGE_PREFIX}-proj-sort-dir`]: projectSortDir }) }); }, [projectSortDir]);
  useEffect(() => { localStorage.setItem(`${STORAGE_PREFIX}-view`, viewMode); apiFetch("/settings", { method: "POST", body: JSON.stringify({ [`${STORAGE_PREFIX}-view`]: viewMode }) }); }, [viewMode]);
  useEffect(() => { document.documentElement.setAttribute("data-theme", themeName); localStorage.setItem(`${STORAGE_PREFIX}-theme`, themeName); apiFetch("/settings", { method: "POST", body: JSON.stringify({ [`${STORAGE_PREFIX}-theme`]: themeName }) }); }, [themeName]);
  useEffect(() => { localStorage.setItem(`${STORAGE_PREFIX}-sort`, sortBy); apiFetch("/settings", { method: "POST", body: JSON.stringify({ [`${STORAGE_PREFIX}-sort`]: sortBy }) }); }, [sortBy]);
  useEffect(() => { localStorage.setItem(`${STORAGE_PREFIX}-sort-dir`, sortDir); apiFetch("/settings", { method: "POST", body: JSON.stringify({ [`${STORAGE_PREFIX}-sort-dir`]: sortDir }) }); }, [sortDir]);
  useEffect(() => { localStorage.setItem(`${STORAGE_PREFIX}-type-colors`, JSON.stringify(clrSettingsCardsColors)); apiFetch("/settings", { method: "POST", body: JSON.stringify({ [`${STORAGE_PREFIX}-type-colors`]: JSON.stringify(clrSettingsCardsColors) }) }); }, [clrSettingsCardsColors]);
  useEffect(() => { localStorage.setItem(`${STORAGE_PREFIX}-ai-sites`, txtSettingsCardsAiSites); apiFetch("/settings", { method: "POST", body: JSON.stringify({ [`${STORAGE_PREFIX}-ai-sites`]: txtSettingsCardsAiSites }) }); }, [txtSettingsCardsAiSites]);
  useEffect(() => { localStorage.setItem(`${STORAGE_PREFIX}-music-sites`, txtSettingsCardsMusicSites); apiFetch("/settings", { method: "POST", body: JSON.stringify({ [`${STORAGE_PREFIX}-music-sites`]: txtSettingsCardsMusicSites }) }); }, [txtSettingsCardsMusicSites]);
  useEffect(() => { localStorage.setItem(`${STORAGE_PREFIX}-lang`, langKey); apiFetch("/settings", { method: "POST", body: JSON.stringify({ [`${STORAGE_PREFIX}-lang`]: langKey }) }); }, [langKey]);
  useEffect(() => {
    const json = JSON.stringify(clrSettingsTags);
    localStorage.setItem(`${STORAGE_PREFIX}-tag-colors`, json);
    apiFetch("/settings", { method: "POST", body: JSON.stringify({ [`${STORAGE_PREFIX}-tag-colors`]: json }) });
  }, [clrSettingsTags]);

  useEffect(() => { localStorage.setItem(`${STORAGE_PREFIX}-sidebar-width`, String(sidebarWidth)); }, [sidebarWidth]);
  useEffect(() => { localStorage.setItem(`${STORAGE_PREFIX}-link-panel-width`, String(linkPanelWidth)); }, [linkPanelWidth]);

  useEffect(() => {
    const onMove = (e) => {
      if (!isLinkResizingRef.current) return;
      const newW = Math.max(180, Math.min(900, linkResizeStartW.current + e.clientX - linkResizeStartX.current));
      setLinkPanelWidth(newW);
    };
    const onUp = () => {
      if (!isLinkResizingRef.current) return;
      isLinkResizingRef.current = false;
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
    };
    document.addEventListener("mousemove", onMove);
    document.addEventListener("mouseup", onUp);
    return () => { document.removeEventListener("mousemove", onMove); document.removeEventListener("mouseup", onUp); };
  }, []);

  useEffect(() => {
    const onMove = (e) => {
      if (!isResizingRef.current) return;
      const newW = Math.max(160, Math.min(520, resizeStartW.current + e.clientX - resizeStartX.current));
      setSidebarWidth(newW);
    };
    const onUp = () => {
      if (!isResizingRef.current) return;
      isResizingRef.current = false;
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
    };
    document.addEventListener("mousemove", onMove);
    document.addEventListener("mouseup", onUp);
    return () => { document.removeEventListener("mousemove", onMove); document.removeEventListener("mouseup", onUp); };
  }, []);

  useEffect(() => {
    Promise.all([apiFetch("/cards"), apiFetch("/projects"), apiFetch("/settings"), apiFetch("/translations"), apiFetch("/environments")])
      .then(([p, pr, settings, tIdx, envs]) => {
        setCards(p); setProjects(pr); setLoading(false); setTranslationsIndex(tIdx || {});
        setEnvironments(envs || []);
        const storedEnvId = localStorage.getItem(`${STORAGE_PREFIX}-active-env`) || "";
        if (!envs?.find(e => e.id === storedEnvId)) {
          const firstRegular = envs?.find(e => !e.isGlobal);
          setActiveEnvId(firstRegular?.id || envs?.[0]?.id || "");
        }
        // Restore settings from file, falling back to localStorage
        if (settings[`${STORAGE_PREFIX}-gemini-key`])       { setTxtSettingsAiApiKey(settings[`${STORAGE_PREFIX}-gemini-key`]);                         localStorage.setItem(`${STORAGE_PREFIX}-gemini-key`, settings[`${STORAGE_PREFIX}-gemini-key`]); }
        const vocalLimit = settings[`${STORAGE_PREFIX}-ai-limit-vocal`] || settings[`${STORAGE_PREFIX}-ai-limit-vocalist`];
        if (vocalLimit) { setTxtSettingsAiLimitsVocal(parseInt(vocalLimit)); localStorage.setItem(`${STORAGE_PREFIX}-ai-limit-vocal`, vocalLimit); }
        if (settings[`${STORAGE_PREFIX}-ai-limit-music`])    { setTxtSettingsAiLimitsMusic(parseInt(settings[`${STORAGE_PREFIX}-ai-limit-music`]));       localStorage.setItem(`${STORAGE_PREFIX}-ai-limit-music`, settings[`${STORAGE_PREFIX}-ai-limit-music`]); }
        if (settings[`${STORAGE_PREFIX}-ai-limit-style`])    { setAiLimitStyle(parseInt(settings[`${STORAGE_PREFIX}-ai-limit-style`]));       localStorage.setItem(`${STORAGE_PREFIX}-ai-limit-style`, settings[`${STORAGE_PREFIX}-ai-limit-style`]); }
        if (settings[`${STORAGE_PREFIX}-ai-warn-limit`])     { setTxtSettingsAiStyleThreshold(parseInt(settings[`${STORAGE_PREFIX}-ai-warn-limit`]));         localStorage.setItem(`${STORAGE_PREFIX}-ai-warn-limit`, settings[`${STORAGE_PREFIX}-ai-warn-limit`]); }
        if (settings[`${STORAGE_PREFIX}-ai-style-word-threshold`]) { setTxtSettingsAiStyleWordThreshold(parseInt(settings[`${STORAGE_PREFIX}-ai-style-word-threshold`])); localStorage.setItem(`${STORAGE_PREFIX}-ai-style-word-threshold`, settings[`${STORAGE_PREFIX}-ai-style-word-threshold`]); }
        if (settings[`${STORAGE_PREFIX}-theme`])             { setThemeName(settings[`${STORAGE_PREFIX}-theme`]);                             localStorage.setItem(`${STORAGE_PREFIX}-theme`, settings[`${STORAGE_PREFIX}-theme`]); }
        if (settings[`${STORAGE_PREFIX}-lang`])              { setLangKey(settings[`${STORAGE_PREFIX}-lang`]);                                localStorage.setItem(`${STORAGE_PREFIX}-lang`, settings[`${STORAGE_PREFIX}-lang`]); }
        if (settings[`${STORAGE_PREFIX}-view`])              { setViewMode(settings[`${STORAGE_PREFIX}-view`]);                               localStorage.setItem(`${STORAGE_PREFIX}-view`, settings[`${STORAGE_PREFIX}-view`]); }
        if (settings[`${STORAGE_PREFIX}-sort`])              { setSortBy(settings[`${STORAGE_PREFIX}-sort`]);                                 localStorage.setItem(`${STORAGE_PREFIX}-sort`, settings[`${STORAGE_PREFIX}-sort`]); }
        if (settings[`${STORAGE_PREFIX}-sort-dir`])          { setSortDir(settings[`${STORAGE_PREFIX}-sort-dir`]);                           localStorage.setItem(`${STORAGE_PREFIX}-sort-dir`, settings[`${STORAGE_PREFIX}-sort-dir`]); }
        if (settings[`${STORAGE_PREFIX}-proj-sort`])         { setProjectSort(settings[`${STORAGE_PREFIX}-proj-sort`]);                       localStorage.setItem(`${STORAGE_PREFIX}-proj-sort`, settings[`${STORAGE_PREFIX}-proj-sort`]); }
        if (settings[`${STORAGE_PREFIX}-proj-sort-dir`])     { setProjectSortDir(settings[`${STORAGE_PREFIX}-proj-sort-dir`]);               localStorage.setItem(`${STORAGE_PREFIX}-proj-sort-dir`, settings[`${STORAGE_PREFIX}-proj-sort-dir`]); }
        if (settings[`${STORAGE_PREFIX}-type-colors`])       { try { const tc = JSON.parse(settings[`${STORAGE_PREFIX}-type-colors`]); setClrSettingsCardsColors({ ...DEFAULT_TYPE_COLORS, ...tc }); localStorage.setItem(`${STORAGE_PREFIX}-type-colors`, settings[`${STORAGE_PREFIX}-type-colors`]); } catch {} }
        if (settings[`${STORAGE_PREFIX}-tag-colors`])        { try { setClrSettingsTags(JSON.parse(settings[`${STORAGE_PREFIX}-tag-colors`]));   localStorage.setItem(`${STORAGE_PREFIX}-tag-colors`,  settings[`${STORAGE_PREFIX}-tag-colors`]);  } catch {} }
      })
      .catch((err) => { setApiError(err.message); setLoading(false); });
  }, []);

  const refreshLrcExists = (list) => {
    const songs = (list || cards).filter(p => p.type === "song" && p.mediaPath);
    if (!songs.length) { setLrcExists({}); return; }
    Promise.all(songs.map(s =>
      fetch(`${API}/api/check-lrc`, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ mediaPath: s.mediaPath }) })
        .then(r => r.json()).then(j => [s.id, j.exists]).catch(() => [s.id, false])
    )).then(pairs => setLrcExists(Object.fromEntries(pairs)));
  };
  const refreshMediaExists = (list) => {
    const withMedia = (list || cards).filter(p => p.mediaPath);
    if (!withMedia.length) { setMediaExists({}); return; }
    fetch(`${API}/api/check-media-batch`, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ paths: withMedia.map(p => ({ id: p.id, mediaPath: p.mediaPath })) }) })
      .then(r => r.json()).then(j => setMediaExists(j.results || {})).catch(() => setMediaExists({}));
  };
  useEffect(() => { if (!loading) { refreshLrcExists(); refreshMediaExists(); } }, [cards, loading]);
  useEffect(() => {
    const onFocus = () => { if (!loading) { refreshLrcExists(); refreshMediaExists(); } };
    window.addEventListener('focus', onFocus);
    return () => window.removeEventListener('focus', onFocus);
  }, [loading, cards]);

  useEffect(() => {
    if (!docCreateOpen) { setAvailableTranslationLangs([]); return; }
    const ids = activeProjectIds.length ? activeProjectIds : (activeProj ? [activeProj.id] : []);
    apiFetch('/translations/languages', { method: 'POST', body: JSON.stringify({ projectIds: ids }) })
      .then(langs => setAvailableTranslationLangs(langs))
      .catch(() => setAvailableTranslationLangs([]));
  }, [docCreateOpen, activeProjectIds]); // eslint-disable-line

  // Shared toast — the pill lifts clear of the player bar when one is open. Errors still pass
  // a longer ms at the call site so they can be read.
  const { showToast, toast } = useToast({ bottom: playerMedia ? 84 : undefined });

  const isGlobalSearch = search.trim().length > 0;
  const parseSearch = (raw) => {
    const s = raw.trim();
    if (/^n:/i.test(s)) return { mode: "name",   q: s.slice(2).trim().toLowerCase() };
    if (/^s:/i.test(s)) return { mode: "style",  q: s.slice(2).trim().toLowerCase() };
    if (/^l:/i.test(s)) return { mode: "lyrics", q: s.slice(2).trim().toLowerCase() };
    return { mode: "all", q: s.toLowerCase() };
  };
  const matchesSearch = (p) => {
    if (!search.trim()) return true;
    const { mode, q } = parseSearch(search);
    const trText = (translationsIndex[p.id] || "").toLowerCase();
    if (mode === "name")   return p.name.toLowerCase().includes(q);
    if (mode === "style")  return p.style.toLowerCase().includes(q) || (p.desc || "").toLowerCase().includes(q);
    if (mode === "lyrics") return (p.lyrics || "").toLowerCase().includes(q) || trText.includes(q);
    return p.name.toLowerCase().includes(q) || p.style.toLowerCase().includes(q) || (p.desc || "").toLowerCase().includes(q) || (p.lyrics || "").toLowerCase().includes(q) || trText.includes(q);
  };
  const isSongType = (p) => p.type === "song";
  const isNoteType = (p) => p.type === "note";
  const toggleType = (t) => { setActiveType((p) => p === t ? null : t); setSelectedTags([]); };
  const matchesActiveType = (p) => activeType === "song" ? isSongType(p) : activeType === "note" ? isNoteType(p) : p.type === activeType;
  const cfg = activeType ? (activeType === "song" ? types.SongStateUndefined : activeType === "note" ? types.note : types[activeType]) : null;

  // Env-scoped visibility
  const globalEnvId = environments.find(e => e.isGlobal)?.id || "";
  const activeEnvSongIds = new Set(
    cards.filter(p => p.env === activeEnvId && p.type === "song").map(p => p.id)
  );
  const usedGlobalIds = new Set(
    cards
      .filter(p => p.env === globalEnvId && (p.type === "music" || p.type === "vocal"))
      .filter(p => cards.some(s => activeEnvSongIds.has(s.id) && (s.linkedCards || []).includes(p.id)))
      .map(p => p.id)
  );
  const envCards  = activeEnvId ? cards.filter(p => p.env === activeEnvId || usedGlobalIds.has(p.id)) : cards;
  const envProjects = activeEnvId ? projects.filter(p => p.env === activeEnvId) : projects;

  // Special filter predicates - shared by sourceCards, header counts, and linkViewSongs
  const isUntagged    = (p) => !p.tags || p.tags.length === 0;
  const isNoAiLink   = (p) => p.type === 'song' && !(p.urls || []).some(u => txtSettingsCardsAiSites.split(",").map(s => s.trim().toLowerCase()).filter(Boolean).includes((u.label || "").toLowerCase().trim()));
  const isNoMedia     = (p) => p.type === 'song' && !p.mediaPath;
  const isNoLrc       = (p) => p.type === 'song' && cleanLyrics(p.lyrics || "").trim() && p.mediaPath && !lrcExists[p.id];
  const isNoPublished = (p) => p.type === 'song' && getSongState(p, types, txtSettingsCardsAiSites, txtSettingsCardsMusicSites) !== types.SongStatePublished;

  // Filter source based on active project or type
  const sourceCards = (() => {
    if (activeProject === UNLINKED_KEY) {
      const allLinkedIds = new Set(envProjects.flatMap(p => p.cardIds));
      return envCards.filter(p => !allLinkedIds.has(p.id));
    }
    if (activeProject === UNTAGGED_KEY) return envCards.filter(isUntagged);
    let base;
    if (activeProjectIds.length > 0) {
      const allIds = new Set();
      const collect = (id) => {
        const pr = envProjects.find(p => p.id === id);
        if (!pr) return;
        pr.cardIds.forEach(pid => allIds.add(pid));
        envProjects.filter(p => p.parentId === id).forEach(child => collect(child.id));
      };
      activeProjectIds.forEach(collect);
      base = [...allIds].map(id => cards.find(p => p.id === id)).filter(Boolean);
    } else {
      base = envCards;
    }
    if (activeType) base = base.filter(p => matchesActiveType(p));
    if (showWarningNoTagged) base = base.filter(isUntagged);
    if (showWarningNoAiLink) base = base.filter(isNoAiLink);
    if (showWarningNoMedia) base = base.filter(isNoMedia);
    if (showWarningNoLrc) base = base.filter(isNoLrc);
    if (showWarningNoPublished) base = base.filter(isNoPublished);
    return base;
  })();

  const allTags = [...new Set(sourceCards.flatMap((p) => p.tags))].sort();

  const filtered = sourceCards.filter((p) => {
    const matchTags   = !selectedTags.length || selectedTags.every((tg) => p.tags.includes(tg));
    const matchFav       = !showFavOnly || p.favorite;
    return matchesSearch(p) && matchTags && matchFav;
  }).sort((a, b) => {
    const typeRank = (t) => t === "note" ? 0 : t === "music" ? 1 : t === "vocal" ? 2 : 3;
    const typeSort = typeRank(a.type) - typeRank(b.type);
    if (typeSort !== 0) return typeSort;
    const dir = sortDir === "asc" ? 1 : -1;
    if (sortBy === "name") { const sortKey = (p) => p.sortNumber != null ? String(p.sortNumber).padStart(2, "0") + "-" + p.name : p.name; const nc = sortKey(a).localeCompare(sortKey(b)); return nc !== 0 ? dir * nc : (a.version || 1) - (b.version || 1); }
    return dir * (a.date - b.date);
  });

  const urlSchema = [...new Set(filtered.flatMap(p => (p.urls || []).map(u => u.label)))];
  const linkSchema = [...new Set(filtered.flatMap(p => p.linkedCards || []))]
    .map(id => cards.find(p => p.id === id)).filter(Boolean)
    .sort((a, b) => { const tr = (t) => t === "note" ? 0 : t === "music" ? 1 : t === "vocal" ? 2 : 3; const td = tr(a.type) - tr(b.type); return td !== 0 ? td : a.name.localeCompare(b.name); });

  const linkScopeCards = (() => {
    if (activeProject === UNLINKED_KEY) {
      const allLinkedIds = new Set(envProjects.flatMap(p => p.cardIds));
      return envCards.filter(p => !allLinkedIds.has(p.id));
    }
    if (activeProject === UNTAGGED_KEY) return envCards.filter(isUntagged);
    if (activeProjectIds.length > 0) {
      const allIds = new Set();
      const collect = (id) => {
        const pr = envProjects.find(p => p.id === id);
        if (!pr) return;
        pr.cardIds.forEach(pid => allIds.add(pid));
        envProjects.filter(p => p.parentId === id).forEach(child => collect(child.id));
      };
      activeProjectIds.forEach(collect);
      return [...allIds].map(id => cards.find(p => p.id === id)).filter(Boolean);
    }
    return envCards;
  })();
  const linkBaseFilter = (p) =>
    matchesSearch(p)
    && (!selectedTags.length || selectedTags.every(tg => p.tags.includes(tg)))
    && (!showFavOnly || viewMode === "link" || p.favorite);
  const linkSortFn = (a, b) => {
    const dir = sortDir === "asc" ? 1 : -1;
    if (sortBy === "name") { const sk = (x) => x.sortNumber != null ? String(x.sortNumber).padStart(2, "0") + "-" + x.name : x.name; const nc = sk(a).localeCompare(sk(b)); return nc !== 0 ? dir * nc : (a.version || 1) - (b.version || 1); }
    return dir * (a.date - b.date);
  };
  const linkViewSongs = linkScopeCards.filter(p => p.type === "song" && linkBaseFilter(p))
    .filter(p => !showWarningNoTagged || isUntagged(p))
    .filter(p => !showWarningNoAiLink || isNoAiLink(p))
    .filter(p => !showWarningNoMedia || isNoMedia(p))
    .filter(p => !showWarningNoLrc || isNoLrc(p))
    .filter(p => !showWarningNoPublished || isNoPublished(p))
    .sort(linkSortFn);
  const linkViewMVSource = linkMVScope === "global" && globalEnvId
    ? cards.filter(p => p.env === globalEnvId && (p.type === "music" || p.type === "vocal"))
    : linkScopeCards.filter(p => (p.type === "music" || p.type === "vocal") && linkBaseFilter(p));
  const linkViewMV = linkViewMVSource
    .filter(p => !linkMVSearch.trim() || p.name.toLowerCase().includes(linkMVSearch.trim().toLowerCase()))
    .sort((a, b) => { const tr = (t) => t === "note" ? 0 : t === "music" ? 1 : t === "vocal" ? 2 : 3; return tr(a.type) - tr(b.type) || linkSortFn(a, b); });

  const toggleTag = (tag, e) =>
    setSelectedTags((prev) =>
      e?.ctrlKey || e?.metaKey
        ? prev.includes(tag) ? prev.filter((tg) => tg !== tag) : [...prev, tag]
        : prev.length === 1 && prev[0] === tag ? [] : [tag]
    );

  const toggleFav = async (id) => {
    const p = cards.find((x) => x.id === id);
    const updated = await apiFetch(`/cards/${id}`, { method: "PUT", body: JSON.stringify({ favorite: !p.favorite }) });
    setCards((prev) => prev.map((x) => x.id === id ? updated : x));
  };

  const copy = (p, mode = "default") => {
    let text = p.style, toastMsg = t("tstCardCopied"), copyKey = p.id;
    if (mode === "title")   { text = p.sortNumber != null ? `${String(p.sortNumber).padStart(2, "0")}-${p.name}` : p.name; toastMsg = t("tstCardTitleCopied"); copyKey = p.id + "_title"; }
    if (mode === "lyrics")  { text = p.lyrics; toastMsg = t("tstCardLyricsCopied"); copyKey = p.id + "_lyrics"; }
    if (mode === "linked") {
      const linked = (p.linkedCards || []).map((id) => cards.find((x) => x.id === id)).filter(Boolean);
      const parts = [...(p.style?.trim() ? [p.style.trim()] : []), ...linked.map((lp) => lp.style).filter(Boolean)];
      text = parts.join("\n");
      toastMsg = t("tstCardLinkedCopied").replace("{n}", linked.length);
      copyKey = p.id + "_linked";
    }
    navigator.clipboard.writeText(text).then(() => {
      setCopiedId(copyKey); showToast(toastMsg);
      setTimeout(() => setCopiedId(null), 1800);
    });
  };

  // Export the open project (or every song, when none is open) as one .amlp LP file — the
  // inverse of the import, written server-side so the grammar lives in exactly one place.
  const exportAmlp = async () => {
    // What you see is what you export: the songs ticked in the view, or — with nothing ticked —
    // every song currently listed (so the active project, search and filters all apply).
    // Intersecting with `filtered` drops a selection left behind outside the current view.
    const inView = selectedIds.length ? filtered.filter(c => selectedIds.includes(c.id)) : filtered;
    const cardIds = inView.filter(c => c.type === "song").map(c => c.id);
    try {
      const result = await apiFetch("/export-amlp", {
        method: "POST",
        body: JSON.stringify({ cardIds, projectId: openProjectId, title: t("ttlOsdSaveDoc") }),
      });
      if (result.error === "noSongs") return showToast("❌ " + t("msgAppExportAmlpNone"), 6000);
      if (!result.canceled) showToast(`✅ ${result.songs} ${t("msgAppExportAmlpSongs")}`);
    } catch (err) {
      showToast("❌ " + err.message, 6000);
    }
  };

  // Closing always drops the Suno draft — a draft left behind would silently re-fill the next
  // new card with the previous song.
  const closeCardModal = () => { setModalOpen(false); setDraftCard(null); };

  // New card — shared by the header + button and Ctrl+N. A Suno link in the clipboard pre-fills
  // the card; anything else opens it empty.
  const openNewCard = async () => {
    if (draftFetching) return;
    setEditingCard(null);
    setDraftFetching(true);
    const draft = await sunoDraftFromClipboard();
    setDraftFetching(false);
    setDraftCard(draft);
    setModalOpen(true);
  };

  // Ctrl+N / Cmd+N — the same action as the + button. Held back while any dialog is up, so it
  // can never stack a second card dialog over the one being edited.
  useEffect(() => {
    const onKeyDown = (e) => {
      if (!(e.ctrlKey || e.metaKey) || e.shiftKey || e.altKey) return;
      if (e.key !== "n" && e.key !== "N") return;
      if (modalOpen || settingsOpen || projectModalOpen || docCreateOpen || bulkTagOpen || batchTranslateOpen) return;
      e.preventDefault();
      openNewCard();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [modalOpen, settingsOpen, projectModalOpen, docCreateOpen, bulkTagOpen, batchTranslateOpen, draftFetching]); // eslint-disable-line react-hooks/exhaustive-deps

  const handleSave = async (data) => {
    try {
      if (editingCard) {
        const updated = await apiFetch(`/cards/${editingCard.id}`, { method: "PUT", body: JSON.stringify(data) });
        const withUrls = { ...updated, urls: data.urls || [] };
        setCards((prev) => {
          const next = prev.map((p) => p.id === editingCard.id ? withUrls : p);
          const removedTags = (editingCard.tags || []).filter(tag => !(data.tags || []).includes(tag));
          if (removedTags.length > 0) {
            const usedTags = new Set(next.flatMap(p => p.tags || []));
            setClrSettingsTags(tc => {
              const copy = { ...tc };
              let changed = false;
              removedTags.forEach(tag => { if (copy[tag] && !usedTags.has(tag)) { delete copy[tag]; changed = true; } });
              return changed ? copy : tc;
            });
          }
          return next;
        });
        if (data.translations?.length > 0) {
          for (const tr of data.translations) {
            if (tr.deleted) {
              await apiFetch(`/translations/${tr.id}`, { method: "DELETE" });
            } else if (tr.pending) {
              await apiFetch("/translations", { method: "POST", body: JSON.stringify({ cardId: editingCard.id, lang: tr.lang, name: tr.name || "", content: tr.content || "" }) });
            } else {
              await apiFetch(`/translations/${tr.id}`, { method: "PUT", body: JSON.stringify({ name: tr.name || "", content: tr.content || "" }) });
            }
          }
        }
        showToast(t("tstAppCardUpdated"));
      } else {
        const created = await apiFetch("/cards", { method: "POST", body: JSON.stringify({ id: generateId(), date: Date.now(), favorite: false, linkedCards: [], env: activeEnvId, ...data }) });
        // Save urls for new cards
        if (data.urls && data.urls.length > 0) {
          for (const u of data.urls) {
            await apiFetch(`/cards/${created.id}/urls`, { method: "POST", body: JSON.stringify({ href: u.href, label: u.label }) });
          }
        }
        const projectId = activeProjectIds.length > 0 ? activeProjectIds[0] : null;
        const withUrls = { ...created, urls: data.urls || [], project: projectId };
        setCards((prev) => [withUrls, ...prev]);
        if (projectId) {
          const updated = await apiFetch(`/projects/${projectId}/cards`, { method: "POST", body: JSON.stringify({ cardId: created.id }) });
          setProjects((prev) => prev.map((p) => p.id === projectId ? updated : p));
        }
        if (data.translations?.length > 0) {
          for (const tr of data.translations) {
            if (tr.deleted) continue;
            await apiFetch("/translations", { method: "POST", body: JSON.stringify({ cardId: created.id, lang: tr.lang, name: tr.name || "", content: tr.content || "" }) });
          }
        }
        showToast(t("tstAppCardCreated"));
      }
      closeCardModal();
    } catch (err) { showToast("❌ " + err.message); }
  };

  const handleDelete = async (id) => {
    try {
      const toDelete = cards.find(p => p.id === id);
      if (toDelete && (toDelete.type === "music" || toDelete.type === "vocal")) {
        const linkedSongs = cards.filter(p => (p.linkedCards || []).includes(id));
        for (const song of linkedSongs) {
          const merged = [song.style?.trim(), toDelete.style?.trim()].filter(Boolean).join("\n");
          if (merged !== song.style) {
            const updated = await apiFetch(`/cards/${song.id}`, { method: "PUT", body: JSON.stringify({ style: merged }) });
            setCards(prev => prev.map(p => p.id === song.id ? updated : p));
          }
        }
      }
      await apiFetch(`/cards/${id}`, { method: "DELETE" });
      setCards((prev) => prev.filter((p) => p.id !== id));
      setProjects((prev) => prev.map(pr => ({ ...pr, cardIds: pr.cardIds.filter(pid => pid !== id) })));
      setDeleteConfirm(null);
      showToast(t("tstAppCardDeleted"));
    } catch (err) { showToast("❌ " + err.message); }
  };

  const handleBulkDelete = async () => {
    try {
      const mvTypes = cards.filter(p => selectedIds.includes(p.id) && (p.type === "music" || p.type === "vocal"));
      for (const mv of mvTypes) {
        const linkedSongs = cards.filter(p => !selectedIds.includes(p.id) && (p.linkedCards || []).includes(mv.id));
        for (const song of linkedSongs) {
          const merged = [song.style?.trim(), mv.style?.trim()].filter(Boolean).join("\n");
          if (merged !== song.style) {
            const updated = await apiFetch(`/cards/${song.id}`, { method: "PUT", body: JSON.stringify({ style: merged }) });
            setCards(prev => prev.map(p => p.id === song.id ? updated : p));
          }
        }
      }
      await Promise.all(selectedIds.map(id => apiFetch(`/cards/${id}`, { method: "DELETE" })));
      setCards(prev => prev.filter(p => !selectedIds.includes(p.id)));
      setProjects(prev => prev.map(pr => ({ ...pr, cardIds: pr.cardIds.filter(pid => !selectedIds.includes(pid)) })));
      showToast(`${selectedIds.length} ${t("tstAppCardDeleted")}`);
      setSelectedIds([]);
      setBulkDeleteConfirm(false);
    } catch (err) { showToast("❌ " + err.message); }
  };

  const handleBulkTag = async (input, action) => {
    const tags = input.split(",").map(t => t.trim().toLowerCase()).filter(Boolean);
    if (!tags.length) return;
    const targets = cards.filter(p => bulkCardIds.includes(p.id));
    try {
      const toUpdate = targets.filter(p =>
        action === "add" ? tags.some(tg => !p.tags.includes(tg)) : tags.some(tg => p.tags.includes(tg))
      );
      if (!toUpdate.length) { showToast(action === "add" ? "Tags already present on all selected" : "Tags not found on any selected"); return; }
      const updated = await Promise.all(
        toUpdate.map(p => {
          const newTags = action === "add"
            ? [...new Set([...p.tags, ...tags])]
            : p.tags.filter(tg => !tags.includes(tg));
          return apiFetch(`/cards/${p.id}`, { method: "PUT", body: JSON.stringify({ tags: newTags }) });
        })
      );
      setCards(prev => prev.map(p => { const u = updated.find(u => u.id === p.id); return u || p; }));
      const label = tags.length === 1 ? `"${tags[0]}"` : `${tags.length} tags`;
      showToast(`${action === "add" ? "Added" : "Removed"} ${label} ${action === "add" ? "to" : "from"} ${toUpdate.length} card${toUpdate.length > 1 ? "s" : ""}`);
      setBulkTagInput("");
    } catch (err) { showToast("❌ " + err.message); }
  };

  const handleMoveCards = async (confirmed = false) => {
    if (!moveTargetEnvId) return;
    // Check if moving music/vocal that has songs still linked in the current env
    if (!confirmed) {
      const selectionHasSong = selectedIds.some(id => { const p = cards.find(x => x.id === id); return p && (p.type === "song_l" || p.type === "song_e" || p.type === "song"); });
      if (!selectionHasSong) {
        const affectedSongs = cards.filter(s =>
          (s.type === "song_l" || s.type === "song_e" || s.type === "song") &&
          s.env === activeEnvId &&
          selectedIds.some(id => (s.linkedCards || []).includes(id))
        );
        if (affectedSongs.length > 0) {
          setMoveCardWarning(affectedSongs.map(s => ({ id: s.id, name: s.name })));
          return;
        }
      }
    }
    setMoveCardWarning(null);
    try {
      const result = await apiFetch("/move-cards", { method: "POST", body: JSON.stringify({ cardIds: selectedIds, targetEnvId: moveTargetEnvId, targetProjectId: moveTargetProjectId || undefined }) });
      const [freshCards, freshProjects] = await Promise.all([apiFetch("/cards"), apiFetch("/projects")]);
      setCards(freshCards);
      setProjects(freshProjects);
      setSelectedIds([]);
      setMoveEnvMode(null);
      setMoveTargetEnvId("");
      setMoveTargetProjectId("");
      let msg = t("tstAppCardMoved").replace("{n}", result.movedIds?.length || 0);
      if ((result.linkedMovedCount || 0) + (result.duplicatedIds?.length || 0) + (result.skippedIds?.length || 0) > 0) {
        msg += " · " + t("tstAppMoveLinkedInfo").replace("{n}", result.linkedMovedCount || 0).replace("{d}", result.duplicatedIds?.length || 0).replace("{s}", result.skippedIds?.length || 0);
      }
      showToast("✅ " + msg);
    } catch (err) { showToast("❌ " + err.message); }
  };

  const handleMoveFolder = async (projectId) => {
    if (!moveTargetEnvId) return;
    try {
      await apiFetch("/move-folder", { method: "POST", body: JSON.stringify({ projectId, targetEnvId: moveTargetEnvId }) });
      const [freshCards, freshProjects] = await Promise.all([apiFetch("/cards"), apiFetch("/projects")]);
      setCards(freshCards);
      setProjects(freshProjects);
      setMoveEnvMode(null);
      setMoveTargetEnvId("");
      showToast("✅ " + t("tstAppFolderMoved"));
    } catch (err) { showToast("❌ " + err.message); }
  };

  const handleBatchTranslate = async () => {
    if (!txtSettingsAiApiKey || !batchTranslateLang) return;
    const lang = batchTranslateLang;
    const targetLangName = getLangName(lang, 'en');
    const pool = batchTranslateSource === "selection"
      ? cards.filter(p => selectedIds.includes(p.id))
      : sourceCards;
    const projCards = pool.filter(p => p.type === "song" && p.lyrics?.trim());
    const toTranslate = [];
    for (const card of projCards) {
      const existing = await apiFetch(`/translations/${card.id}`);
      if (!existing.some(tr => tr.lang === lang)) toTranslate.push(card);
    }
    if (toTranslate.length === 0) { showToast(t("tstDlgTranslateAllDone")); return; }
    if (!batchTranslateDataRef.current) {
      const [templates, presets, systemBlocks] = await Promise.all([
        apiFetch("/ai-templates?task=translate"),
        apiFetch("/ai-presets?task=translate"),
        apiFetch("/ai-system"),
      ]);
      batchTranslateDataRef.current = { templates, presets, systemBlocks };
    }
    const { templates, presets, systemBlocks } = batchTranslateDataRef.current;
    setBatchTranslateRunning(true);
    batchTranslateCancelRef.current = false;
    let done = 0, errors = 0;
    for (const card of toTranslate) {
      if (batchTranslateCancelRef.current) break;
      setBatchTranslateProgress({ current: done + 1, total: toTranslate.length, cardName: card.name });
      try {
        const srcLangName = card.lang ? getLangName(card.lang, 'en') : "";
        const vars = { SOURCE_LANGUAGE: srcLangName, TARGET_LANGUAGE: targetLangName, SONG_NAME: card.name, LYRICS: card.lyrics };
        const request = buildTaskRequest("translate", vars, templates, presets, systemBlocks);
        const bag = await callAi(request, txtSettingsAiApiKey, getApiPort(), "batch-translate");
        const content = bag.CONTENT?.trim() || (bag._raw || "").trim();
        const trName = bag.NAME?.trim() || "";
        await apiFetch("/translations", { method: "POST", body: JSON.stringify({ cardId: card.id, lang, name: trName, content }) });
        if (bag.SOURCE && !card.lang) {
          const detectedCode = bag.SOURCE.trim();
          if (SONG_LANGUAGES.some(l => l.code === detectedCode)) {
            await apiFetch(`/cards/${card.id}`, { method: "PUT", body: JSON.stringify({ lang: detectedCode }) });
          }
        }
        done++;
      } catch (e) {
        console.error(`[MPL] batch translate error for ${card.name}:`, e);
        errors++;
      }
    }
    setBatchTranslateRunning(false);
    setBatchTranslateProgress(null);
    setBatchTranslateOpen(false);
    setBatchTranslateLang("");
    showToast(`${done} ${t("tstDlgTranslateDone")}` + (errors ? `, ${errors} error(s)` : ""));
  };

  // ── Project actions ──────────────────────────────────────────────────────────
  const handleSaveProject = async (data) => {
    try {
      if (editingProject) {
        const updated = await apiFetch(`/projects/${editingProject.id}`, { method: "PUT", body: JSON.stringify(data) });
        setProjects((prev) => prev.map((p) => p.id === editingProject.id ? updated : p));
        if (updated.relocated > 0) {
          const freshCards = await apiFetch("/cards");
          setCards(freshCards);
          refreshMediaExists(freshCards);
          refreshLrcExists(freshCards);
          showToast(`✅ ${t("tstAppProjectUpdated")} - ${updated.relocated} media relocated`);
        } else {
          showToast(t("tstAppProjectUpdated"));
        }
      } else {
        const created = await apiFetch("/projects", { method: "POST", body: JSON.stringify({ id: generateId(), date: Date.now(), env: activeEnvId, ...data }) });
        setProjects((prev) => [created, ...prev]);
        if (data.parentId) setExpandedProjects(prev => new Set([...prev, data.parentId]));
        showToast(t("tstAppProjectCreated"));
      }
      setProjectModalOpen(false);
      setEditingProject(null);
      setNewFolderParentId(null);
    } catch (err) { showToast("❌ " + err.message); }
  };

  const handleDeleteProject = async (id, withCards) => {
    try {
      await apiFetch(`/projects/${id}${withCards ? "?cards=1" : ""}`, { method: "DELETE" });
      setProjects((prev) => prev.filter((p) => p.id !== id));
      setActiveProjectIds(prev => prev.filter(pid => pid !== id));
      if (withCards) setCards((prev) => prev.filter((c) => c.project !== id));
      setDeleteProjectConfirm(null);
      showToast(t("tstAppProjectDeleted"));
    } catch (err) { showToast("❌ " + err.message); }
  };

  const handleMoveProject = async (projectId, newParentId) => {
    if (projectId === newParentId) return;
    if (newParentId) {
      // Guard against circular reference: check if newParentId is a descendant of projectId
      let cursor = projects.find(p => p.id === newParentId);
      while (cursor) {
        if (cursor.id === projectId) return;
        cursor = projects.find(p => p.id === cursor.parentId);
      }
    }
    try {
      const updated = await apiFetch(`/projects/${projectId}`, { method: "PUT", body: JSON.stringify({ parentId: newParentId }) });
      setProjects(prev => prev.map(p => p.id === projectId ? updated : p));
      if (newParentId) setExpandedProjects(prev => new Set([...prev, newParentId]));
    } catch (err) { showToast("❌ " + err.message); }
  };

  const addToProject = async (projectId, cardId) => {
    try {
      const oldProjectId = cards.find(p => p.id === cardId)?.project || null;
      const updated = await apiFetch(`/projects/${projectId}/cards`, { method: "POST", body: JSON.stringify({ cardId }) });
      setProjects(prev => prev.map(p => {
        if (p.id === projectId) return updated;
        if (oldProjectId && p.id === oldProjectId) return { ...p, cardIds: p.cardIds.filter(id => id !== cardId) };
        return p;
      }));
      setCards(prev => prev.map(p => p.id === cardId ? { ...p, project: projectId } : p));
      showToast(t("tstAppAddedToProject"));
    } catch (err) { showToast("❌ " + err.message); }
  };

  const removeFromProject = async (projectId, cardId) => {
    try {
      const updated = await apiFetch(`/projects/${projectId}/cards/${cardId}`, { method: "DELETE" });
      setProjects(prev => prev.map(p => p.id === projectId ? updated : p));
      setCards(prev => prev.map(p => p.id === cardId ? { ...p, project: null } : p));
      showToast(t("tstAppRemovedFromProject"));
    } catch (err) { showToast("❌ " + err.message); }
  };

  // ── Bulk actions — ONE button per action, two targets. Selection wins; otherwise the
  // active single project. Tag / Translate / Unlink act on the target's CARDS; Delete and
  // Move keep their folder-level meaning on a project (delete the project, move the folder
  // + its cards). Greyed when there is neither a selection nor a single active project. ──
  const bulkOnSelection = selectedIds.length > 0;
  const bulkProject = activeProjectIds.length === 1 ? projects.find(p => p.id === activeProjectIds[0]) : null;
  const bulkEnabled = bulkOnSelection || !!bulkProject;
  // The card set Tag/Translate/Unlink operate on (selection, else all of the active
  // project — sourceCards is the project's cards, recursively including sub-projects,
  // the same set the existing project-Translate uses).
  const bulkCardIds = bulkOnSelection ? selectedIds : (bulkProject ? sourceCards.map(c => c.id) : []);
  // Human scope shown in every bulk-action dialog: "8 selected" or "74 · Project Name".
  const bulkScope = bulkOnSelection
    ? `${bulkCardIds.length} ${t("lblHdrSelected")}`
    : `${bulkCardIds.length} · ${bulkProject?.name || ""}`;

  // Unlink a set of cards from a project (each individually). Used by the unified Unlink —
  // removes bulkCardIds from the active project. Runs after the unlink confirm.
  const handleBulkUnlink = async () => {
    const projectId = activeProjectIds[0];
    if (!projectId) return;
    try {
      await Promise.all(bulkCardIds.map(id => apiFetch(`/projects/${projectId}/cards/${id}`, { method: "DELETE" })));
      setProjects(prev => prev.map(p => p.id === projectId ? { ...p, cardIds: p.cardIds.filter(id => !bulkCardIds.includes(id)) } : p));
      setCards(prev => prev.map(p => bulkCardIds.includes(p.id) ? { ...p, project: null } : p));
      showToast(t("tstAppRemovedFromProject"));
      setUnlinkConfirm(false);
      if (bulkOnSelection) clearSelection();
    } catch (err) { showToast("❌ " + err.message); }
  };

  const handleImportFolder = async (projectId) => {
    try {
      const { path } = await apiFetch("/open-folder-dialog");
      if (!path) return;
      setImportFolderPending({ projectId, path });
    } catch (err) { showToast("❌ " + err.message); }
  };

  const handleSyncMusicUrls = async (projectId) => {
    try {
      const data = await apiFetch(`/projects/${projectId}/sync-music-urls`, { method: "POST" });
      const parts = [];
      if (data.added) parts.push(`${data.added} added`);
      if (data.updated) parts.push(`${data.updated} updated`);
      if (data.skipped) parts.push(`${data.skipped} skipped`);
      showToast(`✅ ${data.matched}/${data.totalSongs} ${t("tipSbrSyncUrls")} - ${parts.join(", ")}`);
      if (data.cards) setCards(prev => prev.map(p => { const u = data.cards.find(dp => dp.id === p.id); return u || p; }));
    } catch (err) { showToast("❌ " + (err.message || t("tipSbrSyncUrls"))); }
  };

  const handleImportFolderConfirm = async (songType) => {
    if (!importFolderPending) return;
    const { projectId, path } = importFolderPending;
    setImportFolderPending(null);
    try {
      const data = await apiFetch("/import-folder", { method: "POST", body: JSON.stringify({ folderPath: path, projectId, songType }) });
      if (data.created === 0) { showToast(t("tstAppImportJSONNothing")); return; }
      await apiFetch(`/projects/${projectId}`, { method: "PUT", body: JSON.stringify({ path }) });
      const [freshCards, freshProjects] = await Promise.all([apiFetch("/cards"), apiFetch("/projects")]);
      setCards(freshCards);
      setProjects(freshProjects);
      setActiveProject(projectId);
      showToast(`✅ ${data.created} song${data.created !== 1 ? "s" : ""} imported`);
    } catch (err) { showToast("❌ " + err.message); }
  };

  // ── Selection helpers ───────────────────────────────────────────────────────
  const toggleSelect = (id) =>
    setSelectedIds(prev => prev.includes(id) ? prev.filter(x => x !== id) : [...prev, id]);
  const selectAll  = () => setSelectedIds(filtered.map(p => p.id));
  const clearSelection = () => { setSelectedIds([]); setBulkTagOpen(false); setBulkTagInput(""); };

  // ── Drag handlers ────────────────────────────────────────────────────────────
  const handleDragStart = (e, cardId) => {
    // If dragging a selected card, drag all selected; otherwise just this one
    const ids = selectedIds.includes(cardId) && selectedIds.length > 1
      ? selectedIds
      : [cardId];
    setDraggingId(cardId);
    e.dataTransfer.setData("cardIds", JSON.stringify(ids));
    e.dataTransfer.setData("cardId", cardId); // backward compat
    e.dataTransfer.effectAllowed = "copy";
    const allLinkable = ids.every(id => { const p = cards.find(x => x.id === id); return p && (p.type === "music" || p.type === "vocal"); });
    if (allLinkable) e.dataTransfer.setData("linkable", "1");
  };

  const handleDragEnd = () => setDraggingId(null);

  // Sort view — move draggedId to before/after targetId within orderedIds, then renumber the
  // whole sequence sortNumber = 1..N and persist in one bulk call.
  const handleSortReorder = async (orderedIds, draggedId, targetId, pos) => {
    if (!draggedId || !targetId || draggedId === targetId) return;
    const list = orderedIds.filter(id => id !== draggedId);
    let ti = list.indexOf(targetId);
    if (ti === -1) return;
    if (pos === "after") ti += 1;
    list.splice(ti, 0, draggedId);
    try {
      const updated = await apiFetch("/cards/reorder", { method: "POST", body: JSON.stringify({ ids: list, renameMedia: renameMediaOnSort }) });
      const byId = new Map(updated.map(u => [u.id, u]));
      setCards(prev => prev.map(p => byId.get(p.id) || p));
    } catch (err) { showToast("❌ " + err.message); }
  };

  const handleLinkCard = async (songId, newIds) => {
    try {
      const song = cards.find(p => p.id === songId);
      if (!song) return;
      const merged = [...new Set([...(song.linkedCards || []), ...newIds])];
      const updated = await apiFetch(`/cards/${songId}`, { method: "PUT", body: JSON.stringify({ ...song, linkedCards: merged }) });
      setCards(prev => prev.map(p => p.id === songId ? updated : p));
      showToast(`✅ Linked ${newIds.length} card${newIds.length !== 1 ? "s" : ""}`);
    } catch (err) { showToast("❌ " + err.message); }
  };

  const handleUnlinkCard = async (songId, removeId) => {
    try {
      const song = cards.find(p => p.id === songId);
      if (!song) return;
      const updated = await apiFetch(`/cards/${songId}`, { method: "PUT", body: JSON.stringify({ ...song, linkedCards: (song.linkedCards || []).filter(id => id !== removeId) }) });
      setCards(prev => prev.map(p => p.id === songId ? updated : p));
    } catch (err) { showToast("❌ " + err.message); }
  };

  const isLinkView      = viewMode === "link";
  const isMediaLinkView = viewMode === "media";
  const isSortView      = viewMode === "sort";
  const isSpecialView   = isLinkView || isMediaLinkView || isSortView;

  // Media view — optionally hide files already linked to a song, to focus on the unlinked ones.
  const isMediaLinked = (file) => linkViewSongs.some(s => s.mediaPath === file.path);
  const mediaLinkedCount = mediaLinkFiles.filter(isMediaLinked).length;
  const visibleMediaFiles = hideLinkedMedia ? mediaLinkFiles.filter(f => !isMediaLinked(f)) : mediaLinkFiles;
  const toggleHideLinkedMedia = () => setHideLinkedMedia(v => { const n = !v; localStorage.setItem(`${STORAGE_PREFIX}-hide-linked-media`, n ? "1" : "0"); return n; });
  const toggleRenameMediaOnSort = () => setRenameMediaOnSort(v => { const n = !v; localStorage.setItem(`${STORAGE_PREFIX}-sort-rename-media`, n ? "1" : "0"); return n; });

  const handleToggleFinalized = async () => {
    if (!activeProj) return;
    try {
      const updated = await apiFetch(`/projects/${activeProj.id}`, { method: "PUT", body: JSON.stringify({ finalized: !activeProj.finalized }) });
      setProjects(prev => prev.map(p => p.id === activeProj.id ? updated : p));
    } catch (err) { showToast("❌ " + err.message); }
  };

  const handleScanMediaFolder = async (forceDialog = false) => {
    let folderPath = (!forceDialog && !mediaLinkFolder && activeProj?.path) ? activeProj.path : null;
    if (!folderPath) {
      const data = await apiFetch("/open-folder-dialog");
      if (!data.path) return;
      folderPath = data.path;
    }
    setMediaLinkFolder(folderPath);
    const result = await apiFetch(`/scan-media-folder?path=${encodeURIComponent(folderPath)}`);
    setMediaLinkFiles(result.files || []);
  };

  const handleSetMediaPath = async (songId, filePath) => {
    const song = cards.find(p => p.id === songId);
    if (!song) return;
    try {
      const updated = await apiFetch(`/cards/${songId}`, { method: "PUT", body: JSON.stringify({ ...song, mediaPath: filePath }) });
      setCards(prev => prev.map(p => p.id === songId ? updated : p));
    } catch (err) { showToast("❌ " + err.message); }
  };

  // Auto-link — pair each unlinked song with the scanned file `NN-<title>.<ext>` whose
  // number AND title both match. Non-destructive: existing links and already-taken files
  // are never touched; among flac/wav twins of the same track the flac wins.
  const handleAutoLinkMedia = async () => {
    // ⚠ CLAUDE: mirrors suno-sync's filename sanitize (files on disk are named from it) — keep in step.
    const sanitizeTitle = (title) => title
      .replace(/\s*[\/\\]\s*/g, " - ")
      .replace(/[:*?"<>|]/g, "")
      .replace(/\s+/g, " ")
      .replace(/[ .]+$/, "")
      .trim();
    const usedPaths = new Set(linkViewSongs.map(s => s.mediaPath).filter(Boolean));
    let linked = 0;
    for (const song of linkViewSongs) {
      if (song.mediaPath || song.sortNumber == null) continue;
      const wantNum = parseInt(song.sortNumber, 10);
      if (isNaN(wantNum)) continue;
      const wantTitle = sanitizeTitle(song.name).toLowerCase();
      const matches = mediaLinkFiles.filter(f => {
        if (usedPaths.has(f.path)) return false;
        const m = f.name.match(/^(\d+)\s*-\s*(.+)\.[^.]+$/);
        return m && parseInt(m[1], 10) === wantNum && m[2].replace(/\s+/g, " ").trim().toLowerCase() === wantTitle;
      });
      if (!matches.length) continue;
      const file = matches.find(f => /\.flac$/i.test(f.name)) || matches[0];
      await handleSetMediaPath(song.id, file.path);
      usedPaths.add(file.path);
      linked++;
    }
    showToast(`✅ ${linked} ${t("tstAppMediaAutoLinked")}`);
  };

  const s = makeStyles();
  const activeProj = activeProjectIds.length === 1 ? envProjects.find(p => p.id === activeProjectIds[0]) : null;
  // A finalized project locks its three special functions — the view is enterable but its
  // interactions are greyed out and inert until finalize is unchecked. (Declared after activeProj.)
  const projLocked = isSpecialView && !!activeProj?.finalized;
  const isUnlinkedView = activeProject === UNLINKED_KEY;
  const isUntaggedView = showWarningNoTagged;
  const isNoLrcView    = showWarningNoLrc;

  // Sort view — the active project's OWN songs (direct membership, sub-projects excluded),
  // unfiltered (search/tags/fav ignored) so renumbering always covers the whole set. Ordered
  // by sortNumber then name, matching the name-sort key used elsewhere.
  const sortViewSongs = activeProj
    ? activeProj.cardIds
        .map(id => cards.find(p => p.id === id))
        .filter(p => p && p.type === "song")
        .sort((a, b) => {
          const sk = (x) => x.sortNumber != null ? String(x.sortNumber).padStart(2, "0") + "-" + x.name : x.name;
          const nc = sk(a).localeCompare(sk(b));
          return nc !== 0 ? nc : (a.version || 1) - (b.version || 1);
        })
    : [];

  // The special views (link / media / sort) are project-scoped: available only when exactly one
  // project is active. If the project selection is lost while in one, fall back to the grid.
  useEffect(() => {
    if (!activeProj && (viewMode === "link" || viewMode === "media" || viewMode === "sort")) setViewMode("grid");
  }, [activeProj?.id, viewMode]); // eslint-disable-line

  // Auto-load project media folder when entering media link view
  useEffect(() => {
    if (!isMediaLinkView) return;
    if (!activeProj?.path) { setMediaLinkFolder(""); setMediaLinkFiles([]); return; }
    const folderPath = activeProj.path;
    setMediaLinkFolder(folderPath);
    apiFetch(`/scan-media-folder?path=${encodeURIComponent(folderPath)}`)
      .then(result => setMediaLinkFiles(result.files || []))
      .catch(() => {});
  }, [isMediaLinkView, activeProj?.path]); // eslint-disable-line react-hooks/exhaustive-deps

  if (apiError) {
    return (
      <div className="empty" style={{ ...s.root, display: "flex", alignItems: "center", justifyContent: "center", flexDirection: "column", gap: 20, padding: 40 }}>
        <style>{buildCss()}</style>
        <Plug />
        <h2 style={{ margin: 0, color: 'var(--text)' }}>{t("msgAppCannotReach")}</h2>
        <p style={{ margin: 0, color: 'var(--text-mute)', textAlign: "center", maxWidth: 400, lineHeight: 1.6 }}>
          <code style={{ background: 'var(--bg-elev)', padding: "2px 8px", borderRadius: 4, color: 'var(--accent)' }}>cd mpl-server && npm start</code>
        </p>
        <p className="hint">{apiError}</p>
        <button className="btn primary" onClick={() => { setApiError(null); setLoading(true); Promise.all([apiFetch("/cards"), apiFetch("/projects")]).then(([p,pr]) => { setCards(p); setProjects(pr); setLoading(false); }).catch((e) => { setApiError(e.message); setLoading(false); }); }}>
          <RotateCcw style={{ marginRight: 6 }} />{t("btnAppRetry")}
        </button>
      </div>
    );
  }

  const getOrphanedSongs = (idsToDelete) =>
    cards.filter(p =>
      !idsToDelete.includes(p.id) &&
      (p.linkedCards || []).some(id => idsToDelete.includes(id))
    );

  // Tag / selection strip — rendered BELOW the gradient identity bar via AppHeader's
  // subBar slot (the .barh-subbar), NOT inside the header row. See GUI Standard:
  // "header buttons → separator → optional sub-bar".
  // ⚠ CLAUDE: pass null (not an empty fragment) when the sub-bar has no content - no tags to
  // show AND nothing selected. AppHeader's guard is `subBar != null`, so an always-truthy `<>…</>`
  // renders an empty .barh-subbar band (the "empty strip on an untagged project" bug). subBar
  // must be null when empty. This is the shared sub-bar rule (mirror it wherever a subBar is passed).
  const tagBar = (allTags.length === 0 && selectedIds.length === 0) ? null : (
    <>
      {allTags.map((tag) => {
        const color = clrSettingsTags[tag] || getTagColor(tag);
        const active = selectedTags.includes(tag);
        return <button key={tag} onClick={(e) => toggleTag(tag, e)} className={`chip${active ? " on" : ""}`} style={{ "--chip": color }}>{tag}</button>;
      })}
      {selectedIds.length > 0 && (
        <div style={{ marginLeft: "auto", display: "flex", alignItems: "center", gap: 6, flexShrink: 0 }}>
          <CheckSquare className="icon-inline" color={'var(--accent)'} />
          <span style={{ fontSize: 12, color: 'var(--accent)', fontWeight: 600 }}>{selectedIds.length} {t("lblHdrSelected")}</span>
          <button className="chip" onClick={selectAll} style={{ color: 'var(--accent)', borderColor: 'var(--accent)' + "44" }}>{t("btnHdrSelectAll")} ({filtered.length})</button>
          <button className="chip" onClick={clearSelection}><X className="icon-inline" />{t("btnHdrClearSelection")}</button>
        </div>
      )}
    </>
  );

  return (
    <div className="app-root" style={s.root}
      onDragOver={(e) => { if (e.dataTransfer?.types?.includes("Files")) { e.preventDefault(); e.dataTransfer.dropEffect = "copy"; } }}
      onDrop={(e) => {
        const file = [...(e.dataTransfer?.files || [])].find(f => /\.amlp$/i.test(f.name));
        if (!file) return;   // internal drags (project rows, cards) carry their own types — leave them alone
        e.preventDefault();
        importFromAmlp(file, apiFetch, setCards, setProjects, showToast, t, activeEnvId, openProjectId);
      }}>
      <style>{buildCss()}</style>

      {/* ── Update banner ── */}
      <UpdateBanner info={updateInfo} appId={pkg.name} lang={langKey} storagePrefix={STORAGE_PREFIX} t={t} onClose={() => setUpdateInfo(null)} />

      {/* Header */}
      <AppHeader appName={APP_NAME} appVersion={APP_VERSION}>
        <div className="search barh-search" style={{ maxWidth: 300 }}>
          <span className="search-glyph"><Search /></span>
          <input className="search-input" placeholder={t("plhHdrSearch")} value={search}
            title={t("tipHdrSearchHelp")}
            onChange={(e) => { setSearch(e.target.value); setSelectedTags([]); }} />
          {isGlobalSearch && <span style={{ fontSize: 10, color: "var(--accent)", whiteSpace: "nowrap", pointerEvents: "none" }}>{filtered.length} result{filtered.length !== 1 ? "s" : ""}</span>}
          {search && <button className="search-clear" onClick={() => setSearch("")}><X className="icon-inline" /></button>}
        </div>
        <span className="count" style={{ background: 'var(--bg-row)', color: 'var(--text-mute)', fontSize: 12, padding: "2px 8px" }}>{filtered.length}</span>
        <div className="barh-grp">
          {/* ⚠ CLAUDE: NO wrapper div around these — they must be DIRECT children of .barh-grp
              or the pill-collapse (border/radius/dividers, all `.barh-grp > *`) can't reach them and
              each renders as a separate rounded button. The dim-when-not-applicable state rides on each
              button's `disabled` (`.btn.icon:disabled`), not a wrapper. */}
          {[["note", types.note], ["music", types.music], ["vocal", types.vocal], ["song", types.SongStateUndefined]].map(([key, typ]) => {
            const active = activeType === key;
            const { Icon } = typ;
            return (
              <button key={key} className={`btn icon${active ? " active" : ""}`} onClick={() => toggleType(key)} title={typ.label}
                disabled={isSpecialView}
                style={active ? { color: "var(--accent)", "--accent": typ.accent } : undefined}>
                <Icon />
              </button>
            );
          })}
          <button className={`btn icon${showFavOnly ? " active" : ""}`} onClick={() => setShowFavOnly(v => !v)} title={t("tipHdrFavorites")}
            style={showFavOnly ? { "--accent": "var(--star)" } : undefined}>
            <Star fill={showFavOnly ? "var(--star)" : "none"} />
          </button>
        </div>
        {(() => {
          const scope = activeProjectIds.length > 0 ? (() => { const allIds = new Set(); const collect = (id) => { const pr = envProjects.find(p => p.id === id); if (!pr) return; pr.cardIds.forEach(pid => allIds.add(pid)); envProjects.filter(p => p.parentId === id).forEach(child => collect(child.id)); }; activeProjectIds.forEach(collect); return [...allIds].map(id => cards.find(p => p.id === id)).filter(Boolean); })() : envCards;
          const clearWarnings = () => { setShowWarningNoTagged(false); setShowWarningNoAiLink(false); setShowWarningNoMedia(false); setShowWarningNoLrc(false); setShowWarningNoPublished(false); };
          const warnBtn = (active, setFn, icon, title, color, count) => (
            <button className={`btn icon${active ? " active" : ""}`} onClick={() => { clearWarnings(); setFn(v => !active); }} title={title} disabled={count === 0 && !active}
              style={active ? { "--accent": color } : undefined}>
              {icon}
            </button>
          );
          return (
            <div className="barh-grp">
              {warnBtn(showWarningNoTagged, setShowWarningNoTagged, <Tag />, t("tipHdrWarningNoTagged"), "#a78bfa", scope.filter(isUntagged).length)}
              {warnBtn(showWarningNoMedia, setShowWarningNoMedia, <Disc />, t("tipHdrWarningNoMedia"), "#f97316", scope.filter(isNoMedia).length)}
              {warnBtn(showWarningNoLrc, setShowWarningNoLrc, <FileMusic />, t("tipHdrWarningNoLrc"), "#22d3ee", scope.filter(isNoLrc).length)}
              {warnBtn(showWarningNoAiLink, setShowWarningNoAiLink, <Music2 />, t("tipHdrWarningNoAiLink"), types.SongStateUndefined.accent, scope.filter(isNoAiLink).length)}
              {warnBtn(showWarningNoPublished, setShowWarningNoPublished, <Globe />, t("tipHdrWarningNoPublished"), types.SongStatePublished.accent, scope.filter(isNoPublished).length)}
            </div>
          );
        })()}
        {/* Standard views — always available */}
        <div className="barh-grp">
          {[["grid", LayoutGrid, "tipHdrGridView"], ["list", LayoutList, "tipHdrListView"]].map(([mode, Icon, tip]) => (
            <button key={mode} className={`btn icon${viewMode === mode ? " active" : ""}`} onClick={() => setViewMode(mode)} title={t(tip)}>
              <Icon />
            </button>
          ))}
        </div>
        {/* Special single-feature views — project-scoped: enabled only when exactly one project is
            active, greyed out (not hidden) otherwise */}
        <div className="barh-grp">
          {[["link", Link2, "tipHdrLinkView"], ["media", Disc, "tipHdrMediaLinkView"], ["sort", ArrowUpDown, "tipHdrSortView"]].map(([mode, Icon, tip]) => (
            <button key={mode} className={`btn icon${viewMode === mode ? " active" : ""}`} disabled={!activeProj}
              style={!activeProj ? { opacity: 0.35, cursor: "not-allowed" } : undefined}
              onClick={() => activeProj && setViewMode(mode)} title={t(tip)}>
              <Icon />
            </button>
          ))}
        </div>
        <div className="barh-grp">
          {["date", "name"].map((key) => {
            const active = sortBy === key;
            // The icon encodes sort direction itself (calendar/AZ arrow up = asc,
            // down = desc), so no separate direction indicator is needed. Inactive it
            // shows the default the button applies on first click (desc).
            const asc = active && sortDir === "asc";
            const Icon = key === "date"
              ? (asc ? CalendarArrowUp : CalendarArrowDown)
              : (asc ? ArrowDownAZ : ArrowDownZA);
            return (
              <button key={key} className={`btn icon${active ? " active" : ""}`} title={key === "date" ? t("tipHdrSortByDate") : t("tipHdrSortByName")} onClick={() => { if (sortBy === key) setSortDir(d => d === "asc" ? "desc" : "asc"); else { setSortBy(key); setSortDir("desc"); } }}>
                <Icon />
              </button>
            );
          })}
        </div>
        <div style={{ display: "flex", gap: 8, alignItems: "center" }}>

          {/* Import/Export group */}
          {/* ⚠ CLAUDE: the dropdown must sit OUTSIDE the .barh-grp weld — the weld has overflow:hidden
              and clips any absolutely-positioned popup. Trigger in the weld, menu in this relative wrapper. */}
          <div style={{ position: "relative" }}>
            <div className="barh-grp">
            <button className="btn icon" onClick={() => jsonRef.current?.click()} title={t("tipHdrImport")}>
              <Download />
            </button>
            <button ref={exportBtnRef} className={`btn icon${exportMenuOpen ? " active" : ""}`}
              onClick={() => setExportMenuOpen(v => !v)}
              title={activeProj ? `${t("tipHdrExportProj")} "${activeProj.name}"` : t("tipHdrExportAll")}>
              <Upload />
            </button>
            </div>
            <Menu anchorRef={exportBtnRef} open={exportMenuOpen} onClose={() => setExportMenuOpen(false)} align="right">
              <MenuItem icon={<Upload />} label={activeProj ? `${t("tipHdrExportProj")} "${activeProj.name}"` : t("tipHdrExportCards")}
                onClick={async () => { const proj = activeProj || null; const ep = proj ? cards.filter(p => proj.cardIds.includes(p.id)) : cards; const epr = proj ? [proj] : projects; await exportToJson(ep, epr, proj?.name || null, t("ttlOsdSaveDoc")); }} />
              <MenuItem icon={<FileMusic />} label={t("btnHdrExportAmlp")} onClick={exportAmlp} />
              <MenuItem icon={<Sparkles />} label={t("tipHdrExportAi")} onClick={() => exportAiToJson(t("ttlOsdSaveDoc"))} />
              <MenuItem icon={<Database />} label={t("btnHdrExportBackup")} onClick={() => exportBackupToJson(cards, projects, environments, t("ttlOsdSaveDoc"))} />
            </Menu>
          </div>

          <input ref={jsonRef} type="file" accept=".json,.amlp" style={{ display: "none" }}
            onChange={(e) => { const file = e.target.files?.[0]; if (file) { if (/\.amlp$/i.test(file.name)) importFromAmlp(file, apiFetch, setCards, setProjects, showToast, t, activeEnvId, openProjectId); else importFromJson(file, apiFetch, setCards, setProjects, showToast, t, activeEnvId, setEnvironments, setImportErrors); } e.target.value = ""; }} />
          {/* New card — when the clipboard holds a Suno link, the card opens pre-filled from it
              (name, number, style, lyrics, date, link); otherwise it opens empty as always. */}
          <button className="btn icon primary" disabled={draftFetching} title={`${t("btnHdrNew")} (Ctrl+N)`}
            onClick={openNewCard}>
            {draftFetching ? <RotateCcw className="spinner" /> : <Plus />}
          </button>
        </div>
        {/* Only the standard source·help·settings group is pushed right; everything else left. */}
        <span className="bar-spacer" />
        <div className="barh-grp">
          <button className="btn icon" onClick={() => window.open(GITHUB_URL, '_blank')} title="GitHub">
            <GithubIcon />
          </button>
          <button className="btn icon" onClick={() => window.open(HELP_URL.replace('/en/p/', `/${langKey.replace(/_/g, '-')}/p/`), '_blank')} title={t("tipHdrHelp")}>
            <HelpCircle />
          </button>
          <button className="btn icon" onClick={() => setSettingsOpen(true)} title={t("tipHdrSettings")}>
            <Settings />
          </button>
        </div>
        {/* Bulk tag modal - rendered at root level, see below */}

        {/* Move Card modal - rendered at root level, see below */}
      </AppHeader>

      {/* .app-main content region (Rule 14): the sidebar+main split. Tagged in place
          with flex-direction:row so .app-main's column default doesn't stack it; the
          player footer is a fixed portal, not a flow child. */}
      <div className="app-main" style={{ display: "flex", flex: 1, minHeight: 0, flexDirection: "row" }}>

        {/* Sidebar */}
        {sidebarOpen && (
          <aside style={{ ...s.sidebar, width: sidebarWidth, minWidth: sidebarWidth, flexDirection: "row", padding: 0 }}
            onDragOver={(e) => {
              if (!e.dataTransfer.types.includes("projectid")) return;
              e.preventDefault();
              e.dataTransfer.dropEffect = "move";
              const row = e.target.closest("[data-proj-id]");
              setDragOverFolderId(row?.dataset?.projId ?? null);
            }}
            onDragLeave={(e) => { if (!e.currentTarget.contains(e.relatedTarget)) setDragOverFolderId(null); }}
            onDrop={(e) => {
              e.preventDefault();
              const draggedId = e.dataTransfer.getData("projectid");
              const targetId = dragOverFolderId;
              setDragOverFolderId(null);
              if (draggedId && targetId && draggedId !== targetId) handleMoveProject(draggedId, targetId);
            }}
          >
            {/* Vertical action bar — shared vertical toolbar (.barv-grp welds of .btn.icon atoms) on the --bar-bgd surface */}
            <div className="barv-tools">
              {(() => {
                // Sidebar action button = the shared toolbar atom (.btn.icon): geometry, hover,
                // active state + border/weld all come from .btn.icon / .barv-grp. Only the badge overlay
                // and the per-button `dim` (greyed + click-disabled) are local.
                const sidebarBtn = (icon, onClick, active, title, dim, badge = null) => (
                  <button onClick={onClick} title={title}
                    className={"btn icon" + (active ? " active" : "")}
                    style={{ position: "relative", ...(dim ? sidebarDim : {}) }}>
                    {icon}
                    {badge && <span style={{ position: "absolute", bottom: 2, right: 2, lineHeight: 1, pointerEvents: "none" }}>{badge}</span>}
                  </button>
                );
                const sidebarSelProj = activeProjectIds.length === 1 ? projects.find(p => p.id === activeProjectIds[0]) : null;
                const sidebarDim = { opacity: 0.3, pointerEvents: "none" };
                const sidebarHasPlaylist = sidebarSelProj?.urls?.some(u => { try { const h = new URL(u.href).hostname.toLowerCase(); return txtSettingsCardsMusicSites.split(",").some(s => h.includes(s.trim().toLowerCase())); } catch (_) { return false; } });
                const sidebarFolderEnvOptions = environments.filter(e => e.id !== activeEnvId && !e.isGlobal);
                const dimSel = !sidebarSelProj;
                return (<>
                  {/* Sort group */}
                  <div className="barv-grp">
                    {sidebarBtn((projectSort === "date" && projectSortDir === "asc") ? <CalendarArrowUp /> : <CalendarArrowDown />, () => { if (projectSort === "date") setProjectSortDir(d => d === "asc" ? "desc" : "asc"); else { setProjectSort("date"); setProjectSortDir("desc"); } }, projectSort === "date", "Sort by date", false)}
                    {sidebarBtn((projectSort === "name" && projectSortDir === "desc") ? <ArrowDownZA /> : <ArrowDownAZ />, () => { if (projectSort === "name") setProjectSortDir(d => d === "asc" ? "desc" : "asc"); else { setProjectSort("name"); setProjectSortDir("asc"); } }, projectSort === "name", "Sort by name", false)}
                  </div>
                  {/* Project management — new folder / import / edit / sync URLs (act on the selected project) */}
                  <div className="barv-grp">
                    {sidebarBtn(<FolderPlus />, () => { const sel = activeProjectIds.length === 1 ? activeProjectIds[0] : null; setNewFolderParentId(sel); setEditingProject(null); setProjectModalOpen(true); }, false, t("tipSbrNewProject"), false)}
                    {sidebarBtn(<FolderOpen />, () => handleImportFolder(sidebarSelProj.id), false, t("tipSbrImportFolder"), dimSel)}
                    {sidebarBtn(<Pencil />, () => { setEditingProject(sidebarSelProj); setProjectModalOpen(true); }, false, t("btnCardEdit"), dimSel)}
                    {sidebarBtn(<RefreshCw />, () => handleSyncMusicUrls(sidebarSelProj.id), false, t("tipSbrSyncUrls"), dimSel || !sidebarHasPlaylist)}
                  </div>
                  {/* Bulk card actions — ONE button each; acts on the selected cards, else all
                      cards of the active project. Greyed when neither. Delete/Move keep their
                      folder meaning on a project (delete the project; move the folder + cards). */}
                  <div className="barv-grp">
                    {sidebarBtn(<Tag />, () => { setBulkTagOpen(true); setBulkTagInput(""); }, bulkTagOpen, t("tipSbrTag"), !bulkEnabled)}
                    {sidebarBtn(<Languages />, () => { setBatchTranslateOpen(v => !v); setBatchTranslateLang(""); setBatchTranslateLangSearch(""); setBatchTranslateSource(bulkOnSelection ? "selection" : "project"); }, batchTranslateOpen, t("tipSbrTranslate"), !bulkEnabled)}
                    {sidebarBtn(<FolderUp />, () => { if (bulkOnSelection) { setMoveEnvMode(m => m === "cards" ? null : "cards"); setMoveTargetEnvId(""); setMoveTargetProjectId(""); } else { setMoveEnvMode(m => m === "folder" ? null : "folder"); setMoveTargetEnvId(""); } }, !!moveEnvMode, t("tipSbrMoveEnv"), !bulkEnabled)}
                    {sidebarBtn(<Unlink />, () => setUnlinkConfirm(true), false, t("tipSbrUnlink"), !(activeProjectIds.length === 1 && bulkCardIds.length > 0))}
                  </div>
                  {/* Delete — a single self-framed danger .btn.icon: deletes the project, or the selected cards. */}
                  <div style={{ ...(!bulkEnabled ? sidebarDim : {}) }}>
                    <button onClick={() => { if (bulkOnSelection) setBulkDeleteConfirm(true); else { setDeleteProjectCards(false); setDeleteProjectConfirm(bulkProject.id); } }} title={t("tipCardDelete")} className="btn icon" style={{ color: "var(--danger)" }}>
                      <Trash2 />
                    </button>
                  </div>
                  {/* Move folder modal - rendered at root level, see below */}
                  {/* Excel + doc */}
                  <div className="barv-grp">
                    {sidebarBtn(<FileSpreadsheet />, () => { const proj = activeProj || null; const ep = proj ? cards.filter(p => proj.cardIds.includes(p.id)) : cards; const epr = proj ? [proj] : projects; exportToExcel(ep, epr, proj?.name || null, t("ttlOsdSaveDoc"), API); }, false, activeProj ? `${t("tipSbrExportExcel")} "${activeProj.name}"` : t("tipSbrExportExcel"), false)}
                    {sidebarBtn(<FilePlus />, () => setDocCreateOpen(true), false, t("tipSbrCreateDoc"), false)}
                  </div>
                  {/* Batch translate modal - rendered at root level, see below */}
                </>);
              })()}
            </div>
            {/* Project list */}
            <div className="pnl" style={{ flex: 1, minWidth: 0, overflowY: "auto", display: "flex", flexDirection: "column", padding: "12px 0" }}>

            {/* Environment selector */}
            {environments.length > 0 && (() => {
              const activeEnv = environments.find(e => e.id === activeEnvId);
              return (
                <div style={{ position: "relative", padding: "0 8px 8px", borderBottom: `1px solid var(--border)`, marginBottom: 6 }}>
                  <button ref={envBtnRef}
                    className={"field-trigger" + (envPopoverOpen ? " open" : "")}
                    onClick={() => setEnvPopoverOpen(v => !v)}>
                    <span style={{ display: "flex", alignItems: "center", gap: 5, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                      {activeEnv?.isGlobal && <Globe className="icon-inline" color={'var(--accent)'} />}
                      {activeEnv?.name || "-"}
                    </span>
                    <ChevronDown className="icon-inline" color={'var(--text-mute)'} />
                  </button>
                  <Popover anchorRef={envBtnRef} open={envPopoverOpen} minWidthAnchor
                    onClose={() => { setEnvPopoverOpen(false); setRenamingEnvId(null); setNewEnvInput(null); }}>
                    {environments.map((env, idx) => (
                      <div key={env.id}>
                        {idx === 1 && <div className="menu-divider" />}
                        {renamingEnvId === env.id ? (
                          <div className="pop-item">
                            <input
                              autoFocus
                              value={renameEnvValue}
                              onChange={e => setRenameEnvValue(e.target.value)}
                              onKeyDown={async e => {
                                if (e.key === "Enter" && renameEnvValue.trim()) {
                                  await apiFetch(`/environments/${env.id}`, { method: "PUT", body: JSON.stringify({ name: renameEnvValue.trim() }) });
                                  setEnvironments(prev => prev.map(x => x.id === env.id ? { ...x, name: renameEnvValue.trim() } : x));
                                  setRenamingEnvId(null);
                                  showToast(t("tstPnlProjectEnvRenamed"));
                                }
                                if (e.key === "Escape") setRenamingEnvId(null);
                              }}
                              onClick={e => e.stopPropagation()}
                              className="input" style={{ flex: 1 }}
                            />
                          </div>
                        ) : (
                          <div className={"pop-item" + (env.id === activeEnvId ? " active" : "")} role="button"
                            onClick={() => { setActiveEnvId(env.id); setActiveProjectIds([]); setActiveProject(null); setEnvPopoverOpen(false); }}>
                            {env.isGlobal && <Globe color={'var(--accent)'} />}
                            {env.id === activeEnvId && <Check color={'var(--accent)'} />}
                            <span>{env.name}</span>
                            {!env.isGlobal && (
                              <button className="btn icon small subtle" style={{ marginLeft: "auto" }} onClick={e => { e.stopPropagation(); setRenamingEnvId(env.id); setRenameEnvValue(env.name); }}
                                title={t("btnCardEdit")}>
                                <TextCursor />
                              </button>
                            )}
                            {!env.isGlobal && (
                              <button className="btn icon small subtle" onClick={async e => {
                                e.stopPropagation();
                                try {
                                  await apiFetch(`/environments/${env.id}`, { method: "DELETE" });
                                  const remaining = environments.filter(x => x.id !== env.id);
                                  setEnvironments(remaining);
                                  if (activeEnvId === env.id) {
                                    const next = remaining.find(x => !x.isGlobal) || remaining[0];
                                    setActiveEnvId(next?.id || "");
                                  }
                                  showToast(t("tstPnlProjectEnvDeleted"));
                                } catch (err) {
                                  showToast("❌ " + (err.message.includes("409") ? t("tstPnlProjectEnvNotEmpty") : err.message));
                                }
                              }} title={t("tipCardDelete")}
                             >
                                <Trash2 />
                              </button>
                            )}
                          </div>
                        )}
                      </div>
                    ))}
                    <div className="menu-divider" />
                    {newEnvInput !== null ? (
                      <div className="pop-item">
                        <input
                          autoFocus
                          value={newEnvInput}
                          onChange={e => setNewEnvInput(e.target.value)}
                          onKeyDown={async e => {
                            if (e.key === "Enter" && newEnvInput.trim()) {
                              try {
                                const created = await apiFetch("/environments", { method: "POST", body: JSON.stringify({ name: newEnvInput.trim() }) });
                                setEnvironments(prev => [...prev, created]);
                                setActiveEnvId(created.id);
                                setNewEnvInput(null);
                                setEnvPopoverOpen(false);
                                showToast(t("tstPnlProjectEnvCreated"));
                              } catch (err) { showToast("❌ " + err.message); }
                            }
                            if (e.key === "Escape") { setNewEnvInput(null); }
                          }}
                          placeholder={t("plhPnlProjectEnvName")}
                          className="input" style={{ flex: 1 }}
                        />
                      </div>
                    ) : (
                      <button className="pop-item" onClick={() => setNewEnvInput("")} style={{ color: 'var(--accent)' }}>
                        <Plus />{t("btnPnlProjectEnvNew")}
                      </button>
                    )}
                  </Popover>
                </div>
              );
            })()}

            {/* All cards entry */}
            <button onClick={() => { setActiveProject(null); setActiveProjectIds([]); setActiveType(null); }} className={"lv-item" + ((!activeProject && !activeProjectIds.length && !activeType) ? " active" : "")}>
              <Folder />
              <span className="lv-item-name" style={{ flex: 1 }}>{t("lblPnlProjectAll")}</span>
              <span className="lv-item-badge">{envCards.length}</span>
            </button>

            {/* Unsorted cards entry — cards in no project. Hidden when empty. */}
            {(() => {
              const allLinkedIds = new Set(envProjects.flatMap(p => p.cardIds));
              const unlinkedCount = envCards.filter(p => !allLinkedIds.has(p.id)).length;
              if (unlinkedCount === 0) return null;
              const isActive = activeProject === UNLINKED_KEY;
              return (
                <button onClick={() => { setActiveProject(UNLINKED_KEY); setActiveProjectIds([]); setActiveType(null); setSelectedTags([]); }} className={"lv-item" + (isActive ? " active" : "")}>
                  <Folder color="var(--warn)" />
                  <span className="lv-item-name" style={{ flex: 1 }}>{t("lblPnlProjectUnsorted")}</span>
                  <span className="lv-item-badge" style={{ background: "color-mix(in srgb, var(--warn) 14%, transparent)", color: "var(--warn)" }}>{unlinkedCount}</span>
                </button>
              );
            })()}

            {/* Untagged cards entry — cards with no tags. Hidden when empty. */}
            {(() => {
              const untaggedCount = envCards.filter(isUntagged).length;
              if (untaggedCount === 0) return null;
              const isActive = activeProject === UNTAGGED_KEY;
              return (
                <button onClick={() => { setActiveProject(UNTAGGED_KEY); setActiveProjectIds([]); setActiveType(null); setSelectedTags([]); }} className={"lv-item" + (isActive ? " active" : "")}>
                  <Tag color="#a78bfa" />
                  <span className="lv-item-name" style={{ flex: 1 }}>{t("lblPnlProjectUntagged")}</span>
                  <span className="lv-item-badge" style={{ background: "#1e1a2e", color: "#a78bfa" }}>{untaggedCount}</span>
                </button>
              );
            })()}


            {envProjects.length > 0 && <div style={{ height: 1, background: 'var(--border)', margin: "6px 8px" }} />}

            {envProjects.length === 0 ? (
              <p className="hint" style={{ padding: "8px 12px" }}>{t("empGLViewNoCardsFound")}</p>
            ) : (() => {
              const sorted = [...envProjects].sort((a, b) => {
                const dir = projectSortDir === "asc" ? 1 : -1;
                if (projectSort === "name") return dir * a.name.localeCompare(b.name);
                return dir * (a.date - b.date);
              });
              const childrenOf = {};
              const roots = [];
              for (const p of sorted) {
                if (p.parentId) { (childrenOf[p.parentId] = childrenOf[p.parentId] || []).push(p); }
                else roots.push(p);
              }
              const collectAllCardIds = (projId) => {
                const p = envProjects.find(p => p.id === projId);
                if (!p) return new Set();
                const ids = new Set(p.cardIds);
                for (const child of (childrenOf[projId] || [])) {
                  for (const id of collectAllCardIds(child.id)) ids.add(id);
                }
                return ids;
              };
              const renderProj = (proj, depth) => {
                const children = childrenOf[proj.id] || [];
                const isExpanded = expandedProjects.has(proj.id);
                return (
                  <React.Fragment key={proj.id}>
                    <div data-proj-id={proj.id} style={{ display: "flex", alignItems: "center", paddingLeft: depth * 10 }}>
                      <div style={{ width: 16, flexShrink: 0, display: "flex", alignItems: "center", justifyContent: "center" }}>
                        {children.length > 0 && (
                          <button className="btn icon small subtle" onClick={() => setExpandedProjects(prev => { const n = new Set(prev); n.has(proj.id) ? n.delete(proj.id) : n.add(proj.id); return n; })}>
                            {isExpanded ? <ChevronDown /> : <ChevronRight />}
                          </button>
                        )}
                      </div>
                      <div style={{ flex: 1, minWidth: 0 }}>
                        <ProjectItem
                          proj={proj}
                          isActive={activeProjectIds.includes(proj.id)}
                          isDropTarget={dragOverFolderId === proj.id}
                          draggingId={draggingId}
                          totalCount={collectAllCardIds(proj.id).size}
                          onDragProjectStart={() => setDraggingProjectId(proj.id)}
                          onDragProjectEnd={() => { setDraggingProjectId(null); setDragOverFolderId(null); }}
                          onSelect={(e) => {
                            setActiveProject(null);
                            setSelectedTags([]);
                            if (e.ctrlKey || e.metaKey) {
                              setActiveProjectIds(prev => prev.includes(proj.id) ? prev.filter(id => id !== proj.id) : [...prev, proj.id]);
                            } else {
                              setActiveProjectIds(prev => prev.length === 1 && prev[0] === proj.id ? [] : [proj.id]);
                              setActiveType(null);
                            }
                          }}
                          onDrop={(ids) => ids.forEach(id => addToProject(proj.id, id))}
                          langKey={langKey}
                        />
                      </div>
                    </div>
                    {isExpanded && children.map(child => renderProj(child, depth + 1))}
                  </React.Fragment>
                );
              };
              return (
                <>
                  {draggingProjectId && (
                    <div
                      onDragOver={e => { e.preventDefault(); setRootDropOver(true); }}
                      onDragLeave={() => setRootDropOver(false)}
                      onDrop={e => { e.preventDefault(); setRootDropOver(false); handleMoveProject(draggingProjectId, null); }}
                      style={{ margin: "0 8px 4px", padding: "5px 8px", borderRadius: 6, border: `1px dashed ${rootDropOver ? 'var(--accent)' : 'var(--border-strong)'}`, background: rootDropOver ? 'var(--accent)' + "22" : "transparent", fontSize: 11, color: rootDropOver ? 'var(--accent)' : 'var(--text-mute)', textAlign: "center", transition: "all 0.15s" }}>
                      {t("btnPnlProjectMoveToRoot")}
                    </div>
                  )}
                  {roots.map(proj => renderProj(proj, 0))}
                </>
              );
            })()}

            {draggingId && (
              <div style={{ margin: "8px 8px 0", padding: "8px", borderRadius: 8, border: `1px dashed var(--border-strong)`, fontSize: 11, color: 'var(--text-mute)', textAlign: "center" }}>
                {t("empGLViewDropOnProject")}
              </div>
            )}
            </div>{/* end project list */}
          </aside>
        )}

        {/* Resize handle */}
        {sidebarOpen && (
          <Splitter
            theme={themeName}
            onResizeStart={(e) => {
              isResizingRef.current = true;
              resizeStartX.current = e.clientX;
              resizeStartW.current = sidebarWidth;
              document.body.style.cursor = "col-resize";
              document.body.style.userSelect = "none";
            }}
          />
        )}

        {/* Sidebar toggle tab */}
        <CollapseToggle open={sidebarOpen} onToggle={() => setSidebarOpen(v => !v)} side="left" title={sidebarOpen ? t("tipPnlProjectHide") : t("tipPnlProjectShow")} />

        {/* Main content */}
        <div style={{ flex: 1, minWidth: 0, minHeight: 0, overflow: "hidden", display: "flex", flexDirection: "column" }}>
          {/* View top-bar (Model B, shared `.view-topbar`) - the CONTENT zone's toolbar: tag filter
              + selection cluster. It scopes the CARDS, so it sits at the top of the CONTENT column
              (right of the sidebar) on the content surface, not the header. Same class in apps-cockpit. */}
          {/* Special views (link/media/sort) hide the tag filter and instead carry the finalize toggle —
              check to lock the project's three functions, uncheck to make the view live again. */}
          {isSpecialView
            ? (activeProj && (
                <div className="view-topbar">
                  <button className="btn small subtle" type="button" onClick={handleToggleFinalized}
                   
                    title={t("tipDlgProjectFinalized")} style={{ color: 'var(--text)' }}>
                    {activeProj.finalized ? <CheckSquare color={'var(--accent)'} /> : <Square color={'var(--text-mute)'} />}
                    <span>{t("lblViewFinalized")}</span>
                  </button>
                  {isSortView && (
                    <button className="btn small subtle" type="button" onClick={toggleRenameMediaOnSort}
                     
                      title={t("tipViewRenameMedia")} style={{ color: 'var(--text)' }}>
                      {renameMediaOnSort ? <CheckSquare color={'var(--accent)'} /> : <Square color={'var(--text-mute)'} />}
                      <span>{t("lblViewRenameMedia")}</span>
                    </button>
                  )}
                </div>
              ))
            : (tagBar && <div className="view-topbar">{tagBar}</div>)}

          {/* Grid / List / Link */}
          {viewMode === "sort" ? (
            <div style={{ flex: 1, display: "flex", flexDirection: "column", minHeight: 0, overflow: "hidden" }}>
              <div style={{ padding: "10px 16px", borderBottom: `1px solid var(--border)`, display: "flex", alignItems: "center", gap: 8, flexShrink: 0 }}>
                <ArrowUpDown className="icon-inline" color={'var(--text-mute)'} />
                <span style={{ fontSize: 12, fontWeight: 600, color: 'var(--text-mute)' }}>{t("lblPnlSortSongs")} ({sortViewSongs.length})</span>
              </div>
              <div style={{ flex: 1, overflowY: "auto", padding: "12px 14px", paddingBottom: playerMedia ? 90 : 60, display: "flex", flexDirection: "column", gap: 8 }}>
                {loading ? (
                  <div className="empty"><RotateCcw className="spinner" /></div>
                ) : sortViewSongs.length === 0 ? (
                  <div className="empty"><Music2 color={'var(--text-mute)'} /><p className="empty-title">{t("empPnlSortNoSongs")}</p></div>
                ) : (
                  sortViewSongs.map((song, i) => (
                    <SongListCard key={song.id} song={song} types={types} anyDragging={!!draggingId} allCards={cards}
                      mode="sort" isDragging={draggingId === song.id} isLast={i === sortViewSongs.length - 1} locked={projLocked}
                      onReorder={(draggedId, targetId, pos) => handleSortReorder(sortViewSongs.map(x => x.id), draggedId, targetId, pos)}
                      onDragStart={(e, id) => { setDraggingId(id); e.dataTransfer.setData("cardId", id); e.dataTransfer.effectAllowed = "move"; }}
                      onDragEnd={handleDragEnd}
                      txtSettingsCardsAiSites={txtSettingsCardsAiSites} txtSettingsCardsMusicSites={txtSettingsCardsMusicSites}
                      langKey={langKey} apiPort={getApiPort()} globalEnvId={globalEnvId} />
                  ))
                )}
              </div>
            </div>
          ) : viewMode === "media" ? (
            <div style={{ flex: 1, display: "flex", minHeight: 0, overflow: "hidden" }}>
              {/* Left panel - song drop targets */}
              <div style={{ width: linkPanelWidth, minWidth: linkPanelWidth, display: "flex", flexDirection: "column", overflow: "hidden" }}>
                <div style={{ padding: "10px 16px", borderBottom: `1px solid var(--border)`, display: "flex", alignItems: "center", gap: 8, flexShrink: 0 }}>
                  <types.SongStateCreated.Icon className="icon-inline" color={types.SongStateCreated.accent} />
                  <span style={{ fontSize: 12, fontWeight: 600, color: 'var(--text-mute)' }}>{t("lblPnlLinkSongs")} ({linkViewSongs.length})</span>
                </div>
                <div style={{ flex: 1, overflowY: "auto", padding: "12px 14px", paddingBottom: playerMedia ? 90 : 60, display: "flex", flexDirection: "column", gap: 8 }}>
                  {loading ? (
                    <div className="empty"><RotateCcw className="spinner" /></div>
                  ) : linkViewSongs.length === 0 ? (
                    <div className="empty"><Music2 color={'var(--text-mute)'} /><p className="empty-title">{t("empPnlLinkNoSongs")}</p></div>
                  ) : (
                    linkViewSongs.map(song => (
                      <SongListCard key={song.id} song={song} types={types} anyDragging={!!dragMediaFile} locked={projLocked}
                        mode="media" onSetMedia={handleSetMediaPath} onClearMedia={(id) => handleSetMediaPath(id, "")}
                        onEdit={(p) => { setEditingCard(p); setModalOpen(true); }}
                        isPlaying={playerMedia?.cardId === song.id && globalPlaying} txtSettingsCardsAiSites={txtSettingsCardsAiSites} txtSettingsCardsMusicSites={txtSettingsCardsMusicSites}
                        apiPort={getApiPort()} langKey={langKey} lrcExists={!!lrcExists[song.id]} mediaExists={mediaExists[song.id] !== false}
                        allCards={cards} clrSettingsTags={clrSettingsTags} onFav={toggleFav} copiedId={copiedId}
                        onCopy={(p, mode) => copy(p, mode)} onDelete={(id) => setDeleteConfirm(id)}
                        onRemoveFromProject={activeProjectIds.length === 1 ? (projId, songId) => removeFromProject(projId, songId) : null}
                        activeProjectId={activeProjectIds.length === 1 ? activeProjectIds[0] : null}
                        linkSchema={linkSchema} globalEnvId={globalEnvId}
                        onPlay={(p) => { if (!p.mediaPath) return; if (playerMedia?.cardId === p.id) { globalPlayerRef.current?.togglePlay(); return; } setPlayerMedia({ path: p.mediaPath, cardName: p.name, cardId: p.id }); setPlaylist([]); }} />
                    ))
                  )}
                </div>
              </div>
              {/* Resize handle */}
              <Splitter theme={themeName}
                onResizeStart={(e) => { isLinkResizingRef.current = true; linkResizeStartX.current = e.clientX; linkResizeStartW.current = linkPanelWidth; document.body.style.cursor = "col-resize"; document.body.style.userSelect = "none"; }} />
              {/* Right panel - media files */}
              <div style={{ flex: 1, minWidth: 0, display: "flex", flexDirection: "column", overflow: "hidden" }}>
                <div style={{ padding: "10px 16px", borderBottom: `1px solid var(--border)`, display: "flex", alignItems: "center", gap: 8, flexShrink: 0 }}>
                  <Disc className="icon-inline" color={'var(--text-mute)'} />
                  <span style={{ fontSize: 12, fontWeight: 600, color: 'var(--text-mute)', flex: 1 }}>
                    {mediaLinkFolder ? mediaLinkFolder.split(/[\\/]/).pop() : t("lblPnlLinkNoFolder")} {mediaLinkFiles.length > 0 ? `(${mediaLinkFiles.length})` : ""}
                  </span>
                  <button onClick={() => handleScanMediaFolder(!!mediaLinkFolder)} title={t("tipPnlLinkChooseFolder")} className="btn icon small">
                    <FolderOpen />
                  </button>
                  {mediaLinkFiles.length > 0 && (
                    <button onClick={handleAutoLinkMedia} disabled={projLocked} title={t("tipPnlMediaAutoLink")} className="btn icon small">
                      <Wand2 />
                    </button>
                  )}
                  {mediaLinkedCount > 0 && (
                    <button onClick={toggleHideLinkedMedia} title={hideLinkedMedia ? t("tipPnlMediaShowLinked") : t("tipPnlMediaHideLinked")}
                      className={`btn icon small${hideLinkedMedia ? " active" : ""}`}>
                      {hideLinkedMedia ? <CircleDashed /> : <Disc />}
                    </button>
                  )}
                </div>
                <div style={{ flex: 1, overflowY: "auto", padding: 12, paddingBottom: playerMedia ? 90 : 60, display: "flex", flexDirection: "column", gap: 6 }}>
                  {!mediaLinkFolder ? (
                    <div className="empty"><Disc color={'var(--text-mute)'} /><p className="empty-title">{t("empPnlLinkBrowseFolder")}</p></div>
                  ) : mediaLinkFiles.length === 0 ? (
                    <div className="empty"><Disc color={'var(--text-mute)'} /><p className="empty-title">{t("empPnlLinkNoAudio")}</p></div>
                  ) : visibleMediaFiles.length === 0 ? (
                    <div className="empty"><Disc color={'var(--text-mute)'} /><p className="empty-title">{t("empPnlMediaAllLinked")}</p></div>
                  ) : (
                    visibleMediaFiles.map(file => {
                      const linkedSong = linkViewSongs.find(s => s.mediaPath === file.path);
                      return (
                        <MediaLinkFileCard key={file.path} file={file} isDragging={dragMediaFile?.path === file.path} locked={projLocked}
                          linkedSong={linkedSong} types={types} txtSettingsCardsAiSites={txtSettingsCardsAiSites} txtSettingsCardsMusicSites={txtSettingsCardsMusicSites}
                          onDragStart={() => setDragMediaFile(file)}
                          onDragEnd={() => setDragMediaFile(null)} />
                      );
                    })
                  )}
                </div>
              </div>
            </div>
          ) : viewMode === "link" ? (
            <div style={{ flex: 1, display: "flex", minHeight: 0, overflow: "hidden" }}>
              {/* Left panel - song drop targets */}
              <div style={{ width: linkPanelWidth, minWidth: linkPanelWidth, display: "flex", flexDirection: "column", overflow: "hidden" }}>
                <div style={{ padding: "10px 16px", borderBottom: `1px solid var(--border)`, display: "flex", alignItems: "center", gap: 8, flexShrink: 0 }}>
                  <types.SongStateCreated.Icon className="icon-inline" color={types.SongStateCreated.accent} />
                  <span style={{ fontSize: 12, fontWeight: 600, color: 'var(--text-mute)' }}>{t("lblPnlLinkSongs")} ({linkViewSongs.length})</span>
                </div>
                <div style={{ flex: 1, overflowY: "auto", padding: "12px 14px", paddingBottom: playerMedia ? 90 : 60, display: "flex", flexDirection: "column", gap: 8 }}>
                  {loading ? (
                    <div className="empty"><RotateCcw className="spinner" /></div>
                  ) : linkViewSongs.length === 0 ? (
                    <div className="empty"><Music2 color={'var(--text-mute)'} /><p className="empty-title">{t("empPnlLinkNoSongs")}</p></div>
                  ) : (
                    linkViewSongs.map(song => (
                      <SongListCard key={song.id} song={song} types={types} anyDragging={!!draggingId} allCards={cards} locked={projLocked}
                        mode="link" onLink={handleLinkCard} onUnlink={handleUnlinkCard}
                        onEdit={(p) => { setEditingCard(p); setModalOpen(true); }}
                        isPlaying={playerMedia?.cardId === song.id && globalPlaying}
                        txtSettingsCardsAiSites={txtSettingsCardsAiSites} txtSettingsCardsMusicSites={txtSettingsCardsMusicSites} langKey={langKey} mediaExists={mediaExists[song.id] !== false}
                        apiPort={getApiPort()} lrcExists={!!lrcExists[song.id]}
                        clrSettingsTags={clrSettingsTags} onFav={toggleFav} copiedId={copiedId}
                        onCopy={(p, mode) => copy(p, mode)} onDelete={(id) => setDeleteConfirm(id)}
                        onRemoveFromProject={activeProjectIds.length === 1 ? (projId, songId) => removeFromProject(projId, songId) : null}
                        activeProjectId={activeProjectIds.length === 1 ? activeProjectIds[0] : null}
                        linkSchema={linkSchema} globalEnvId={globalEnvId}
                        onPlay={(p) => { if (!p.mediaPath) return; if (playerMedia?.cardId === p.id) { globalPlayerRef.current?.togglePlay(); return; } setPlayerMedia({ path: p.mediaPath, cardName: p.name, cardId: p.id }); setPlaylist([]); }} />
                    ))
                  )}
                </div>
              </div>
              {/* Resize handle */}
              <Splitter
                theme={themeName}
                onResizeStart={(e) => {
                  isLinkResizingRef.current = true;
                  linkResizeStartX.current = e.clientX;
                  linkResizeStartW.current = linkPanelWidth;
                  document.body.style.cursor = "col-resize";
                  document.body.style.userSelect = "none";
                }}
              />
              {/* Right panel - music + vocal drag sources */}
              <div style={{ flex: 1, minWidth: 0, display: "flex", flexDirection: "column", overflow: "hidden" }}>
                <div style={{ padding: "10px 16px 8px", borderBottom: `1px solid var(--border)`, display: "flex", flexDirection: "column", gap: 6, flexShrink: 0 }}>
                  <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                    <Link2 className="icon-inline" color={'var(--text-mute)'} />
                    <span style={{ fontSize: 12, fontWeight: 600, color: 'var(--text-mute)', flex: 1 }}>{t("lblPnlLinkMusicVocal")} ({linkViewMV.length})</span>
                    {globalEnvId && (
                      <div className="opt-btns">
                        {[["local", Folder, t("lblPnlLinkCurrentProject")], ["global", Globe, t("lblPnlLinkGlobal")]].map(([val, Icon, label]) => {
                          const active = linkMVScope === val;
                          return <button key={val} className={"opt-btn" + (active ? " active" : "")} onClick={() => setLinkMVScope(val)} title={label}>
                            <Icon className="icon-inline" />{label}
                          </button>;
                        })}
                      </div>
                    )}
                  </div>
                  <input value={linkMVSearch} onChange={e => setLinkMVSearch(e.target.value)}
                    placeholder={t("plhHdrSearchCards")}
                    className="input" style={{ width: "100%" }} />
                </div>
                <div style={{ flex: 1, overflowY: "auto", padding: 12, paddingBottom: playerMedia ? 90 : 60, display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(220px, 1fr))", gap: 10, alignContent: "start" }}>
                  {loading ? (
                    <div className="empty"><RotateCcw className="spinner" /></div>
                  ) : linkViewMV.length === 0 ? (
                    <div className="empty"><Music2 color={'var(--text-mute)'} /><p className="empty-title">{t("empPnlLinkNoMusicVocal")}</p></div>
                  ) : (
                    linkViewMV.map(p => (
                      <LinkViewMVCard key={p.id} card={p} types={types} isDragging={draggingId === p.id} locked={projLocked}
                        linkedByCount={envCards.filter(s => (s.linkedCards || []).includes(p.id)).length}
                        onEdit={(p) => { setEditingCard(p); setModalOpen(true); }}
                        onDragStart={(e, id) => { setDraggingId(id); e.dataTransfer.setData("cardIds", JSON.stringify([id])); e.dataTransfer.setData("cardId", id); e.dataTransfer.setData("linkable", "1"); e.dataTransfer.effectAllowed = "copy"; }}
                        onDragEnd={handleDragEnd} langKey={langKey} />
                    ))
                  )}
                </div>
              </div>
            </div>
          ) : (
            <main style={{ ...(viewMode === "list" ? s.list : s.grid), paddingBottom: playerMedia ? 90 : 60 }}>
              {loading ? (
                <div className="empty"><RotateCcw className="spinner" /><p className="empty-title">{t("msgPnlLinkLoading")}</p></div>
              ) : filtered.length === 0 ? (
                <div className="empty">
                  {isUntaggedView ? <Tag color={'var(--text-mute)'} /> : isUnlinkedView ? <Star color={'var(--text-mute)'} /> : isNoLrcView ? <FileMusic color={'var(--text-mute)'} /> : activeProjectIds.length > 0 ? <FolderOpen color={'var(--text-mute)'} /> : cfg ? <cfg.Icon color={'var(--text-mute)'} /> : <Music2 color={'var(--text-mute)'} />}
                  <p className="empty-title">{isUntaggedView ? t("empGLViewAllHaveTags") : isUnlinkedView ? t("empGLViewAllInProject") : isNoLrcView ? t("empGLViewAllHaveLrc") : activeProjectIds.length > 0 ? t("empGLViewNoCardsProject") : t("empGLViewNoCardsFound")}</p>
                  <p className="empty-sub">{isUntaggedView || isUnlinkedView || isNoLrcView ? "" : activeProjectIds.length > 0 ? t("empGLViewDragToAdd") : t("empGLViewAdjustFilters")}</p>
                </div>
              ) : (
                filtered.map((p) => (
                  <CardItem key={p.id} card={p} cfg={p.type === "song" ? getSongState(p, types, txtSettingsCardsAiSites, txtSettingsCardsMusicSites) : types[p.type]} themeName={themeName}
                    copiedId={copiedId} allCards={envCards} types={types} viewMode={viewMode} langKey={langKey} globalEnvId={globalEnvId}
                    isDragging={draggingId === p.id}
                    anyDragging={!!draggingId}
                    isSelected={selectedIds.includes(p.id)}
                    activeProjectId={activeProjectIds.length === 1 ? activeProjectIds[0] : null}
                    apiPort={getApiPort()} lrcExists={!!lrcExists[p.id]} mediaExists={mediaExists[p.id] !== false}
                    urlSchema={urlSchema} linkSchema={linkSchema}
                    txtSettingsCardsAiSites={txtSettingsCardsAiSites} txtSettingsCardsMusicSites={txtSettingsCardsMusicSites}
                    isPlaying={playerMedia?.cardId === p.id && globalPlaying}
                    clrSettingsTags={clrSettingsTags}
                    onCopy={(p, mode) => copy(p, mode)} onFav={toggleFav}
                    onEdit={(p) => { setEditingCard(p); setModalOpen(true); }}
                    onDelete={(id) => setDeleteConfirm(id)}
                    onDragStart={handleDragStart}
                    onDragEnd={handleDragEnd}
                    onLinkCard={handleLinkCard}
                    onToggleSelect={toggleSelect}
                    onRemoveFromProject={activeProjectIds.length === 1 ? () => removeFromProject(activeProjectIds[0], p.id) : null}
                    onPlay={(playCard) => {
                      if (!playCard.mediaPath) return;
                      if (playerMedia?.cardId === playCard.id) { globalPlayerRef.current?.togglePlay(); return; }
                      const pl = filtered.filter(x => x.mediaPath).map(x => ({ path: x.mediaPath, cardName: x.name, cardId: x.id }));
                      const idx = pl.findIndex(x => x.cardId === playCard.id);
                      setPlaylist(pl);
                      setPlaylistIndex(idx >= 0 ? idx : 0);
                      setPlayerMedia({ path: playCard.mediaPath, cardName: playCard.name, cardId: playCard.id });
                    }}
                  />
                ))
              )}
            </main>
          )}
        </div>
      </div>

      {/* Modals */}
      {modalOpen && (
        <CardModal card={editingCard} draft={draftCard} allCards={cards} langKey={langKey} apiPort={getApiPort()} txtSettingsAiApiKey={txtSettingsAiApiKey} txtSettingsAiLimitsVocal={txtSettingsAiLimitsVocal} txtSettingsAiLimitsMusic={txtSettingsAiLimitsMusic} txtSettingsAiStyleThreshold={txtSettingsAiStyleThreshold} txtSettingsAiLyricsThreshold={txtSettingsAiLyricsThreshold} txtSettingsAiStyleWordThreshold={txtSettingsAiStyleWordThreshold}
          allExistingTags={[...new Set(cards.flatMap((p) => p.tags))].sort()}
          activeProject={activeProjectIds.length === 1 ? activeProjectIds[0] : null} projects={projects}
          txtSettingsCardsAiSites={txtSettingsCardsAiSites} txtSettingsCardsMusicSites={txtSettingsCardsMusicSites} aiDefaultTemplates={aiDefaultTemplates} tglSettingsAiTemplatesMode={tglSettingsAiTemplatesMode}
          onSetDefaultTemplate={(task, id) => {
            setAiDefaultTemplates(prev => ({ ...prev, [task]: id }));
            localStorage.setItem(`${STORAGE_PREFIX}-ai-default-template-${task}`, id || "");
            apiFetch("/settings", { method: "POST", body: JSON.stringify({ [`${STORAGE_PREFIX}-ai-default-template-${task}`]: id || "" }) });
          }}
          globalEnvId={globalEnvId}
          onMakeGlobal={async (id) => {
            try {
              const updated = await apiFetch(`/cards/${id}`, { method: "PUT", body: JSON.stringify({ env: globalEnvId }) });
              setCards(prev => prev.map(p => p.id === id ? updated : p));
              showToast("✅ " + t("tstPnlProjectMadeGlobal"));
            } catch (err) { showToast("❌ " + err.message); }
          }} types={types} onSave={handleSave} onClose={closeCardModal} showToast={showToast}
          onUpdateTranslationsIndex={(cardId, list) => setTranslationsIndex(prev => ({ ...prev, [cardId]: list.map(x => x.content).join("\n") }))} />
      )}

      {/* Move folder/cards to environment - unified modal */}

      {/* Batch translate modal */}
      {batchTranslateOpen && (
        <div className="dl-backdrop" onMouseDown={e => { if (e.target === e.currentTarget && !batchTranslateRunning) { setBatchTranslateOpen(false); setBatchTranslateLang(""); } }}>
          <div className="dlp" style={{ maxWidth: 300 }}>
            {batchTranslateRunning && batchTranslateProgress ? (
              <>
                <Languages color={'var(--accent)'} className="dlp-icon" />
                <p className="dlp-title">{t("lblDlgTranslateProgress")} {batchTranslateProgress.current}/{batchTranslateProgress.total}</p>
                <p className="dlp-sub" style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{batchTranslateProgress.cardName}</p>
                <div className="dlp-foot">
                  <button className="btn subtle" onClick={() => { batchTranslateCancelRef.current = true; }}><X />{t("btnGlbCancel")}</button>
                </div>
              </>
            ) : (
              <>
                <button className="dl-close" onClick={() => { setBatchTranslateOpen(false); setBatchTranslateLang(""); }}><X /></button>
                <Languages color={'var(--accent)'} className="dlp-icon" />
                <p className="dlp-title">{t("ttlDlgTranslate")}</p>
                <div className="dlp-body">
                <div style={{ width: "100%" }}>
                  <span className="dlg-field-label" style={{ marginBottom: 4, display: "block" }}>{t("lblDlgTranslateTarget")}</span>
                  <Combobox
                    autoFocus
                    placeholder={t("lblDlgTranslateTarget")}
                    value={batchTranslateLangOpen ? batchTranslateLangSearch : (batchTranslateLang ? `${getLangName(batchTranslateLang, langKey)} (${batchTranslateLang})` : "")}
                    onChange={setBatchTranslateLangSearch}
                    onFocus={() => setBatchTranslateLangSearch(batchTranslateLang ? getLangName(batchTranslateLang, langKey) : "")}
                    onOpenChange={setBatchTranslateLangOpen}
                    items={(() => { const q = batchTranslateLangSearch.toLowerCase(); return q ? SONG_LANGUAGES.filter(l => l.name.toLowerCase().includes(q) || getLangName(l.code, langKey).toLowerCase().includes(q) || l.code.toLowerCase().includes(q)) : []; })()}
                    itemKey={l => l.code}
                    itemActive={l => l.code === batchTranslateLang}
                    renderItem={l => <>{getLangName(l.code, langKey)} <span className="pop-dim">({l.code})</span></>}
                    onPick={l => { setBatchTranslateLang(l.code); setBatchTranslateLangSearch(""); }}
                  />
                </div>
                </div>
                <div className="dlp-foot">
                  <button className="btn primary" disabled={!batchTranslateLang || !txtSettingsAiApiKey} onClick={handleBatchTranslate}>{t("btnDlgTranslateStart")}</button>
                </div>
              </>
            )}
          </div>
        </div>
      )}

      {/* Move to environment - unified modal (folder or cards) */}
      {moveEnvMode && (() => {
        const isCards = moveEnvMode === "cards";
        const isFolder = moveEnvMode === "folder";
        const folderProj = isFolder && activeProjectIds.length === 1 ? projects.find(p => p.id === activeProjectIds[0]) : null;
        if (isFolder && !folderProj) return null;
        if (isCards && selectedIds.length === 0) return null;
        const selectionHasSong = isCards && selectedIds.some(id => { const p = cards.find(x => x.id === id); return p && (p.type === "song_l" || p.type === "song_e" || p.type === "song"); });
        const envOptions = environments.filter(e => e.id !== activeEnvId && (isCards && selectionHasSong ? !e.isGlobal : !e.isGlobal));
        const buildFolderOptions = (projs) => {
          const byParent = {};
          for (const p of projs) { const k = p.parentId || ""; (byParent[k] = byParent[k] || []).push(p); }
          const result = [];
          const walk = (pid, depth) => { (byParent[pid] || []).sort((a,b) => a.name.localeCompare(b.name)).forEach(p => { result.push({...p, depth}); walk(p.id, depth+1); }); };
          walk("", 0);
          return result;
        };
        const targetFolders = isCards && moveTargetEnvId ? buildFolderOptions(projects.filter(p => p.env === moveTargetEnvId)) : [];
        const closeModal = () => { setMoveEnvMode(null); setMoveTargetEnvId(""); setMoveTargetProjectId(""); setMoveCardWarning(null); };
        return (
          <div className="dl-backdrop" onMouseDown={e => { if (e.target === e.currentTarget) closeModal(); }}>
            <div className="dlp" style={{ maxWidth: isCards ? 380 : 320 }}>
              <button className="dl-close" onClick={closeModal}><X /></button>
              <ArrowUpDown color="#4dc8c8" className="dlp-icon" />
              <p className="dlp-title">{t("ttlDlgMoveEnv")}{isCards ? ` (${selectedIds.length})` : ""}</p>
              <div className="dlp-body">
                <div style={{ display: "flex", gap: 6, alignItems: "center" }}>
                  <select className="select" value={moveTargetEnvId} onChange={e => { setMoveTargetEnvId(e.target.value); setMoveTargetProjectId(""); }} style={{ flex: 1 }}>
                    <option value="">{t("optDlgMoveEnvSelect")}</option>
                    {envOptions.map(e => <option key={e.id} value={e.id}>{e.name}</option>)}
                  </select>
                </div>
                {isCards && moveTargetEnvId && (
                  <div style={{ display: "flex", gap: 6, alignItems: "center" }}>
                    <span className="dlg-field-label" style={{ whiteSpace: "nowrap" }}>{t("lblDlgMoveEnvProject")}</span>
                    <select className="select" value={moveTargetProjectId} onChange={e => setMoveTargetProjectId(e.target.value)} style={{ flex: 1 }}>
                      <option value="">📁 {t("optDlgMoveEnvRoot")}</option>
                      {targetFolders.map(p => (
                        <option key={p.id} value={p.id}>{"\u00a0\u00a0".repeat(p.depth)}{p.depth > 0 ? "↳ " : "📁 "}{p.name}</option>
                      ))}
                    </select>
                  </div>
                )}
              </div>
              {isCards && moveCardWarning && (
                <div style={{ width: "100%", padding: "8px 10px", borderRadius: 8, border: `1px solid color-mix(in srgb, var(--warn) 40%, transparent)`, background: "color-mix(in srgb, var(--warn) 12%, transparent)", display: "flex", flexDirection: "column", gap: 6 }}>
                  <span style={{ fontSize: 12, color: "var(--warn)", fontWeight: 600 }}>⚠ Moving will unlink from {moveCardWarning.length} song{moveCardWarning.length !== 1 ? "s" : ""} in this environment:</span>
                  <span className="hint">{moveCardWarning.map(s => s.name).join(", ")}</span>
                  <span className="hint">{t("msgDlgMoveEnvMergeWarning")}</span>
                  <div style={{ display: "flex", gap: 6 }}>
                    <button className="btn primary" style={{ "--accent": "var(--warn)", "--accent-hov": "color-mix(in srgb, var(--warn) 82%, black)" }} onClick={() => handleMoveCards(true)}>
                      <ArrowUpDown />{t("btnDlgMoveEnvForce")}
                    </button>
                    <button className="btn subtle" onClick={() => setMoveCardWarning(null)}>
                      <X />{t("btnGlbCancel")}
                    </button>
                  </div>
                </div>
              )}
              <div className="dlp-foot">
                <button className="btn primary" disabled={!moveTargetEnvId} onClick={isCards ? handleMoveCards : () => handleMoveFolder(folderProj.id)}
                  style={{ "--accent": "#4dc8c8", "--accent-hov": "#3bb8b8" }}>
                  <ArrowUpDown />{t("btnDlgMoveEnvMove")}
                </button>
              </div>
            </div>
          </div>
        );
      })()}

      {/* Bulk tag modal — targets the selection, or all cards of the active project. */}
      {bulkTagOpen && bulkEnabled && (() => {
        const allTags = [...new Set(cards.flatMap(p => p.tags))].sort();
        const tokens = bulkTagInput.split(",");
        const currentToken = tokens[tokens.length - 1].trim().toLowerCase();
        const enteredTags = tokens.slice(0, -1).map(t => t.trim().toLowerCase()).filter(Boolean);
        const suggestions = currentToken.length > 0
          ? allTags.filter(tg => tg.includes(currentToken) && tg !== currentToken && !enteredTags.includes(tg))
          : [];
        const selectedCards = cards.filter(p => bulkCardIds.includes(p.id));
        const tagsOnAll  = allTags.filter(tg => selectedCards.every(p => p.tags.includes(tg)));
        const tagsOnSome = allTags.filter(tg => selectedCards.some(p => p.tags.includes(tg)) && !tagsOnAll.includes(tg));
        return (
          <div className="dl-backdrop" onMouseDown={e => { if (e.target === e.currentTarget) { setBulkTagOpen(false); setBulkTagInput(""); } }}>
            <div className="dlp" style={{ maxWidth: 340 }}>
              <button className="dl-close" onClick={() => { setBulkTagOpen(false); setBulkTagInput(""); }}><X /></button>
              <Tag color="#a78bfa" className="dlp-icon" />
              <p className="dlp-title">{t("ttlDlgTags")} ({bulkScope})</p>
              <div className="dlp-body">
              <div style={{ width: "100%" }}>
                <Combobox
                  autoFocus
                  placeholder={t("plhDlgTagsInput")}
                  value={bulkTagInput}
                  onChange={setBulkTagInput}
                  onKeyDown={e => {
                    if (e.key === "Enter" && bulkTagInput.trim()) { handleBulkTag(bulkTagInput, "add"); }
                    if (e.key === "Escape") { setBulkTagOpen(false); }
                  }}
                  items={currentToken.length > 0 ? suggestions : []}
                  itemKey={tg => tg}
                  renderItem={tg => tg}
                  onPick={tg => { const prefix = tokens.slice(0, -1).map(t => t.trim()).filter(Boolean).join(", "); setBulkTagInput(prefix ? prefix + ", " + tg + ", " : tg + ", "); }}
                />
              </div>
              {(tagsOnAll.length > 0 || tagsOnSome.length > 0) && (
                <div style={{ display: "flex", flexWrap: "wrap", gap: 5, alignItems: "center", marginTop: 4 }}>
                  <span className="hint" style={{ marginRight: 2 }}>{t("lblDlgTagsQuickRemove")}</span>
                  {tagsOnAll.map(tg => (
                    <button key={tg} className="chip on" onClick={() => handleBulkTag(tg, "remove")} title={`Remove "${tg}" from all selected`}>
                      {tg}<X className="icon-inline" />
                    </button>
                  ))}
                  {tagsOnSome.map(tg => (
                    <button key={tg} className="chip" onClick={() => handleBulkTag(tg, "remove")} title={`Remove "${tg}" from selected cards that have it`}>
                      {tg}<X className="icon-inline" />
                    </button>
                  ))}
                </div>
              )}
              </div>
              <div className="dlp-foot">
                <button className="btn primary" style={{ flex: 1, "--accent": "var(--ok)", "--accent-hov": "color-mix(in srgb, var(--ok) 82%, black)" }} disabled={!bulkTagInput.trim()} onClick={() => handleBulkTag(bulkTagInput, "add")}>
                  <Plus />{t("btnDlgTagsAdd")}
                </button>
                <button className="btn primary" style={{ flex: 1, "--accent": "var(--danger)", "--accent-hov": "color-mix(in srgb, var(--danger) 82%, black)" }} disabled={!bulkTagInput.trim()} onClick={() => handleBulkTag(bulkTagInput, "remove")}>
                  <X />{t("btnDlgTagsRemove")}
                </button>
              </div>
            </div>
          </div>
        );
      })()}

      {deleteConfirm && (() => {
        const orphanedSongs = getOrphanedSongs([deleteConfirm]);
        return (
          <div className="dl-backdrop" onMouseDown={(e) => { if (e.target === e.currentTarget) setDeleteConfirm(null); }}>
            <div className="dlp" onKeyDown={(e) => { if (e.key !== 'Tab') return; const btns = [...e.currentTarget.querySelectorAll('button')]; const first = btns[0]; const last = btns[btns.length - 1]; if (e.shiftKey) { if (document.activeElement === first) { e.preventDefault(); last.focus(); } } else { if (document.activeElement === last) { e.preventDefault(); first.focus(); } } }}>
              <button className="dl-close" onClick={() => setDeleteConfirm(null)}><X /></button>
              <Trash2 color="var(--danger)" className="dlp-icon" />
              <p className="dlp-title">{t("cfmDlgDeleteCardQ")}</p>
              {orphanedSongs.length > 0 ? (
                <div style={{ marginBottom: 12, textAlign: "left", width: "100%" }}>
                  <p className="dlp-sub" style={{ marginBottom: 6, color: "var(--danger)" }}>⚠ {t("msgDlgDeleteLinkedWarning")}</p>
                  <div style={{ maxHeight: 180, overflowY: "auto", display: "flex", flexDirection: "column", gap: 3 }}>
                    {orphanedSongs.map(s2 => (
                      <span key={s2.id} style={{ fontSize: 12, color: 'var(--text)', padding: "2px 0" }}>
                        • {s2.name}<span style={{ fontSize: 10, fontWeight: 700, opacity: 0.6, marginLeft: 4 }}>v{s2.version || 1}</span>
                      </span>
                    ))}
                  </div>
                </div>
              ) : (
                <p className="dlp-sub">{t("cfmDlgDeleteCardSub")}</p>
              )}
              <div className="dlp-foot">
                <button className="btn subtle dlg-confirm-cancel-default" ref={el => { if (el && !el.dataset.didFocus) { el.dataset.didFocus = "1"; el.focus(); } }} onClick={() => setDeleteConfirm(null)}>{t("btnGlbCancel")}</button>
                <button className="btn primary" style={{ "--accent": "var(--danger)", "--accent-hov": "color-mix(in srgb, var(--danger) 82%, black)" }} onClick={() => handleDelete(deleteConfirm)}><Trash2 style={{ marginRight: 6 }} />{t("btnCardDelete")}</button>
              </div>
            </div>
          </div>
        );
      })()}

      {importFolderPending && (
        <div className="dl-backdrop" onMouseDown={(e) => { if (e.target === e.currentTarget) setImportFolderPending(null); }}>
          <div className="dlp">
            <button className="dl-close" onClick={() => setImportFolderPending(null)}><X /></button>
            <FolderOpen color={types.SongStateCreated.accent} className="dlp-icon" />
            <p className="dlp-title">{t("cfmSbrImportSongsTitle")}</p>
            <p className="dlp-sub">{t("cfmSbrImportSongsSub")}</p>
            <div className="dlp-foot">
              <button className="btn subtle" onClick={() => setImportFolderPending(null)}>{t("btnGlbCancel")}</button>
              <button className="btn primary" style={{ "--accent": types.SongStateCreated.accent, "--accent-hov": types.SongStateCreated.accent }}
                onClick={() => handleImportFolderConfirm("song")}>
                <Music2 />{t("btnSbrImportConfirm")}
              </button>
            </div>
          </div>
        </div>
      )}

      {deleteProjectConfirm && (
        <div className="dl-backdrop" onMouseDown={(e) => { if (e.target === e.currentTarget) setDeleteProjectConfirm(null); }}>
          <div className="dlp">
            <button className="dl-close" onClick={() => setDeleteProjectConfirm(null)}><X /></button>
            <Trash2 color="var(--danger)" className="dlp-icon" />
            <p className="dlp-title">{t("cfmDlgDeleteProjectTitle")}</p>
            <div className="dlp-body">
              <label className="check-label">
                <input type="checkbox" className="check-box" checked={deleteProjectCards} onChange={(e) => setDeleteProjectCards(e.target.checked)} />
                <span>{t("lblDlgDeleteProjectCards")}</span>
              </label>
            </div>
            <div className="dlp-foot">
              <button className="btn subtle" onClick={() => setDeleteProjectConfirm(null)}>{t("btnGlbCancel")}</button>
              <button className="btn primary" style={{ "--accent": "var(--danger)", "--accent-hov": "color-mix(in srgb, var(--danger) 82%, black)" }} onClick={() => handleDeleteProject(deleteProjectConfirm, deleteProjectCards)}><Trash2 style={{ marginRight: 6 }} />{t("btnCardDelete")}</button>
            </div>
          </div>
        </div>
      )}

      {/* Unlink confirm — remove the target cards (selection, else all of the project) from the active project. */}
      {unlinkConfirm && (
        <div className="dl-backdrop" onMouseDown={(e) => { if (e.target === e.currentTarget) setUnlinkConfirm(false); }}>
          <div className="dlp">
            <button className="dl-close" onClick={() => setUnlinkConfirm(false)}><X /></button>
            <Unlink color="var(--warn)" className="dlp-icon" />
            <p className="dlp-title">{t("cfmDlgUnlinkTitle")}</p>
            <p className="dlp-sub">{t("cfmDlgUnlinkSub")} — {bulkScope}</p>
            <div className="dlp-foot">
              <button className="btn subtle" onClick={() => setUnlinkConfirm(false)}>{t("btnGlbCancel")}</button>
              <button className="btn primary" style={{ "--accent": "var(--warn)", "--accent-hov": "color-mix(in srgb, var(--warn) 82%, black)" }} onClick={handleBulkUnlink}><Unlink style={{ marginRight: 6 }} />{t("btnHdrUnlink")}</button>
            </div>
          </div>
        </div>
      )}

      {projectModalOpen && (
        <ProjectModal
          project={editingProject} parentId={newFolderParentId} isSubFolder={!!newFolderParentId} themeName={themeName} langKey={langKey} apiPort={getApiPort()}
          onSave={handleSaveProject} onClose={() => { setProjectModalOpen(false); setEditingProject(null); setNewFolderParentId(null); }} />
      )}

      {docCreateOpen && (() => {
        const opts = [
          { key: "original", label: t("btnDlgDocContentLyricsOriginal"), desc: t("msgDlgDocContentLyricsOriginalDesc") },
          { key: "clean",    label: t("btnDlgDocContentLyricsCleaned"),  desc: t("msgDlgDocContentLyricsCleanedDesc") },
        ];
        const fmtOpts = [
          { key: "docx", label: "DOCX" },
          { key: "md",   label: "Markdown" },
          { key: "adoc", label: "AsciiDoc" },
        ];
        const close = () => setDocCreateOpen(false);
        // Songs the export will actually include, given the current filters — mirrors the
        // server-side filtering in /export-docx | /export-md | /export-adoc.
        const ids = activeProjectIds.length ? activeProjectIds : (activeProj ? [activeProj.id] : []);
        let exportSongs = cards.filter(c => c.type === "song" && (ids.length === 0 || ids.includes(c.project)));
        if ((docCreateSettings.filterLyrics || "withLyrics") === "withLyrics") exportSongs = exportSongs.filter(c => hasRealLyrics(c.lyrics));
        if ((docCreateSettings.filterPublished || "all") === "publishedOnly") exportSongs = exportSongs.filter(c => getSongState(c, types, txtSettingsCardsAiSites, txtSettingsCardsMusicSites) === types.SongStatePublished);
        const exportCount = exportSongs.length;
        const doExport = async () => {
          close();
          const fmt = docCreateSettings.docFormat || "docx";
          const endpoint = fmt === "md" ? "/export-md" : fmt === "adoc" ? "/export-adoc" : "/export-docx";
          try {
            const r = await apiFetch(endpoint, { method: "POST", body: JSON.stringify({ projectIds: ids, contentLyrics: docCreateSettings.contentLyrics, contentLangs: docCreateSettings.contentLangs || [], contentImages: docCreateSettings.contentImages !== "without", contentDesc: docCreateSettings.contentDesc !== "without", filterLyrics: docCreateSettings.filterLyrics || "withLyrics", filterPublished: docCreateSettings.filterPublished || "all", uiLang: langKey, translationLabel: t('lblDlgCardTranslation'), instrumentalLabel: t('lblDlgDocLyricsInstrumental'), title: t('ttlOsdSaveDoc') }) });
            if (!r.canceled) showToast("✅ " + (fmt === "md" ? "Markdown" : fmt === "adoc" ? "AsciiDoc" : "Document") + " saved");
          } catch(err) { showToast("❌ " + err.message); }
        };
        return (
          <div className="dl-backdrop" onMouseDown={(e) => { if (e.target === e.currentTarget) close(); }}>
            <div className="dlg" style={{ maxWidth: 360, padding: 28, gap: 20 }} onKeyDown={(e) => { if (e.key !== 'Tab') return; const btns = [...e.currentTarget.querySelectorAll('button')]; const first = btns[0]; const last = btns[btns.length - 1]; if (e.shiftKey) { if (document.activeElement === first) { e.preventDefault(); last.focus(); } } else { if (document.activeElement === last) { e.preventDefault(); first.focus(); } } }}>
              <div style={s.modalHead}>
                <h2 className="dlg-title" style={{ margin: 0 }}><FilePlus />{t("ttlDlgDoc")}</h2>
                <button className="dl-close" onClick={close}><X /></button>
              </div>
              {/* Format option */}
              <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                <span className="dlg-field-label">{t("lblDlgDocFormat")}</span>
                <div className="barh-grp">
                  {fmtOpts.map((opt) => (
                    <button key={opt.key} onClick={() => setDocCreateSettings(prev => ({ ...prev, docFormat: opt.key }))}
                      className={`btn icon${(docCreateSettings.docFormat || "docx") === opt.key ? " active" : ""}`}
                      style={{ flex: 1, justifyContent: "center" }}>
                      {opt.label}
                    </button>
                  ))}
                </div>
              </div>
              {/* Images option */}
              <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                <span className="dlg-field-label">{t("lblDlgDocContentImages")}</span>
                <div className="barh-grp">
                  {[{ key: "with", label: t("btnDlgDocContentWith") }, { key: "without", label: t("btnDlgDocContentWithout") }].map((opt) => (
                    <button key={opt.key} onClick={() => setDocCreateSettings(prev => ({ ...prev, contentImages: opt.key }))}
                      className={`btn icon${(docCreateSettings.contentImages || "with") === opt.key ? " active" : ""}`}
                      style={{ flex: 1, justifyContent: "center" }}>
                      {opt.label}
                    </button>
                  ))}
                </div>
              </div>
              {/* Description option */}
              <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                <span className="dlg-field-label">{t("lblDlgDocContentDesc")}</span>
                <div className="barh-grp">
                  {[{ key: "with", label: t("btnDlgDocContentWith") }, { key: "without", label: t("btnDlgDocContentWithout") }].map((opt) => (
                    <button key={opt.key} onClick={() => setDocCreateSettings(prev => ({ ...prev, contentDesc: opt.key }))}
                      className={`btn icon${(docCreateSettings.contentDesc || "with") === opt.key ? " active" : ""}`}
                      style={{ flex: 1, justifyContent: "center" }}>
                      {opt.label}
                    </button>
                  ))}
                </div>
              </div>
              {/* Published filter option */}
              <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                <span className="dlg-field-label">{t("lblDlgDocFilterPublished")}</span>
                <div className="barh-grp">
                  {[{ key: "publishedOnly", label: t("btnDlgDocFilterPublishedOnly") }, { key: "all", label: t("btnDlgDocFilterAll") }].map((opt) => (
                    <button key={opt.key} onClick={() => setDocCreateSettings(prev => ({ ...prev, filterPublished: opt.key }))}
                      className={`btn icon${(docCreateSettings.filterPublished || "all") === opt.key ? " active" : ""}`}
                      style={{ flex: 1, justifyContent: "center" }}>
                      {opt.label}
                    </button>
                  ))}
                </div>
              </div>
              {/* Lyrics filter option */}
              <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                <span className="dlg-field-label">{t("lblDlgDocFilterLyrics")}</span>
                <div className="barh-grp">
                  {[{ key: "withLyrics", label: t("btnDlgDocFilterWithLyrics") }, { key: "all", label: t("btnDlgDocFilterAll") }].map((opt) => (
                    <button key={opt.key} onClick={() => setDocCreateSettings(prev => ({ ...prev, filterLyrics: opt.key }))}
                      className={`btn icon${(docCreateSettings.filterLyrics || "withLyrics") === opt.key ? " active" : ""}`}
                      style={{ flex: 1, justifyContent: "center" }}>
                      {opt.label}
                    </button>
                  ))}
                </div>
              </div>
              {/* Lyrics content option */}
              <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                <span className="dlg-field-label">{t("lblDlgDocContentLyrics")}</span>
                <div className="barh-grp">
                  {opts.map((opt) => (
                    <button key={opt.key} onClick={() => setDocCreateSettings(prev => ({ ...prev, contentLyrics: opt.key }))}
                      className={`btn icon${docCreateSettings.contentLyrics === opt.key ? " active" : ""}`}
                      title={opt.desc}
                      style={{ flex: 1, justifyContent: "center" }}>
                      {opt.label}
                    </button>
                  ))}
                </div>
              </div>
              {/* Translations option */}
              <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                <span className="dlg-field-label">{t("lblDlgDocContentLangs")}</span>
                <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
                  {availableTranslationLangs.map(lang => {
                    const selected = (docCreateSettings.contentLangs || []).includes(lang);
                    let label = lang;
                    try { label = new Intl.DisplayNames([langKey], { type: 'language' }).of(lang) || lang; } catch (_) {}
                    return (
                      <button key={lang}
                        onClick={() => setDocCreateSettings(prev => ({ ...prev, contentLangs: selected ? (prev.langs || []).filter(l => l !== lang) : [...(prev.langs || []), lang] }))}
                        className={`chip${selected ? " on" : ""}`}>
                        {label}
                      </button>
                    );
                  })}
                </div>
              </div>
              {/* Actions */}
              <div style={{ display: "flex", gap: 10, justifyContent: "flex-end", alignItems: "center" }}>
                <span className={`hint${exportCount === 0 ? " warn" : ""}`} style={{ marginRight: "auto" }}>{exportCount} {t("msgDlgDocSongCount")}</span>
                <button className="btn subtle" ref={el => { if (el && !el.dataset.didFocus) { el.dataset.didFocus = "1"; el.focus(); } }} onClick={close}>{t("btnGlbCancel")}</button>
                <button className="btn primary" disabled={exportCount === 0} onClick={doExport}><FilePlus style={{ marginRight: 6 }} />{t("btnDlgDocCreate")}</button>
              </div>
            </div>
          </div>
        );
      })()}

      {settingsOpen && (() => {
        return (
        <div className="dl-backdrop" onMouseDown={(e) => { if (e.target === e.currentTarget) setSettingsOpen(false); }}>
          <div className="dlg" style={{ maxWidth: 500 }} onKeyDown={(e) => { if (e.key !== 'Tab') return; const els = [...e.currentTarget.querySelectorAll('button, input, select, textarea, [tabindex]:not([tabindex="-1"])')].filter(el => !el.disabled && el.offsetParent !== null); const first = els[0]; const last = els[els.length - 1]; if (e.shiftKey) { if (document.activeElement === first) { e.preventDefault(); last.focus(); } } else { if (document.activeElement === last) { e.preventDefault(); first.focus(); } } }}>

            {/* Header */}
            <div className="dlg-head">
              <h2 className="dlg-title"><Settings />{t("ttlDlgSettings")}</h2>
              <button className="dl-close" ref={el => { if (el && !el.dataset.didFocus) { el.dataset.didFocus = "1"; el.focus(); } }} onClick={() => setSettingsOpen(false)}><X /></button>
            </div>

            {/* Tabs */}
            <div className="tabs" style={{ padding: "0 24px" }}>
              {[{ key: "display", label: t("tabDlgSettingsDisplay"), icon: Sun }, { key: "colors", label: t("tabDlgSettingsCards"), icon: Pipette }, { key: "tags", label: t("tabDlgSettingsTags"), icon: Tag }, { key: "ai", label: t("tabDlgSettingsAi"), icon: Sparkles }, { key: "about", label: t("tabDlgSettingsAbout"), icon: ScrollText }].map(({ key, label, icon: TabIcon }) => {
                const active = tabSettingsActive === key;
                return (
                  <button key={key} onClick={() => setTabSettingsActive(key)} className={`tab${active ? " active" : ""}`}>
                    <TabIcon />{label}
                  </button>
                );
              })}
            </div>

            {/* Tab content - grid stacks all panels in one cell; active panel wins via zIndex+background (DLG-8) */}
            <div className="dlg-body" style={{ flex: 1, display: "grid" }}>

              <div style={{ gridArea: '1/1', visibility: tabSettingsActive === "display" ? 'visible' : 'hidden', zIndex: tabSettingsActive === "display" ? 1 : 0, background: "var(--dlg-bgd)", transition: 'none' }}>
                {/* Language */}
                <div className="dlg-field" style={{ paddingTop: 20 }}>
                  <label className="dlg-field-label">{t("lblDlgSettingsDisplayLang")}</label>
                  <select value={langKey} onChange={(e) => setLangKey(e.target.value)} className="select">
                    {LANGUAGES.map(({ key, label }) => (
                      <option key={key} value={key}>{label}</option>
                    ))}
                  </select>
                </div>

                {/* Theme */}
                <div className="dlg-field divider">
                  <label className="dlg-field-label">{t("lblDlgSettingsDisplayTheme")}</label>
                  <div className="opt-btns">
                    {[{ key: "dark", Icon: Moon, i18n: "btnDlgSettingsDisplayThemeDark" }, { key: "light", Icon: Sun, i18n: "btnDlgSettingsDisplayThemeLight" }].map(({ key, Icon: TIcon, i18n }) => { const label = t(i18n);
                      const active = themeName === key;
                      return (
                        <button key={key} onClick={() => setThemeName(key)} className={`opt-btn${active ? " active" : ""}`}>
                          <TIcon /><span>{label}</span>
                        </button>
                      );
                    })}
                  </div>
                </div>

              </div>

              <div style={{ gridArea: '1/1', visibility: tabSettingsActive === "colors" ? 'visible' : 'hidden', zIndex: tabSettingsActive === "colors" ? 1 : 0, background: "var(--dlg-bgd)", transition: 'none' }}>
                <div className="dlg-field" style={{ paddingTop: 20, gap: 20 }}>
                  <label className="dlg-field-label">{t("lblDlgSettingsCardsColors")}</label>
                  <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
                    {Object.entries(types).map(([key, typ]) => {
                      const { Icon: TIcon } = typ;
                      return (
                        <div key={key} style={{ display: "flex", flexDirection: "column", gap: 8, padding: "4px 0" }}>
                          <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
                            <div style={{ display: "flex", alignItems: "center", gap: 7, width: 120, fontSize: 13, color: typ.accent, fontWeight: 600, flexShrink: 0 }}>
                              <TIcon className="icon-inline" color={typ.accent} />{t(TYPE_LABEL_KEYS[key] || key)}
                            </div>
                            <ColorPicker color={clrSettingsCardsColors[key]} onChange={(c) => setClrSettingsCardsColors(prev => ({ ...prev, [key]: c }))} cancelLabel={t("btnGlbCancel")} applyLabel={t("btnDlgCardApply")} pickTitle={t("tipGlbPickFromScreen")} />
                            <button className="btn subtle" onClick={() => setClrSettingsCardsColors(prev => ({ ...prev, [key]: DEFAULT_TYPE_COLORS[key] }))} style={{ marginLeft: "auto" }}>{t("btnDlgSettingsCardsReset")}</button>
                          </div>
                        </div>
                      );
                    })}
                  </div>
                  <div className="dlg-field">
                    <label className="dlg-field-label">{t("lblDlgSettingsCardsAiSites")}</label>
                    <input className="input" value={txtSettingsCardsAiSites} onChange={e => setTxtSettingsCardsAiSites(e.target.value)} placeholder="udio, suno, producer, tunee" />
                  </div>
                  <div className="dlg-field">
                    <label className="dlg-field-label">{t("lblDlgSettingsCardsMusicSites")}</label>
                    <input className="input" value={txtSettingsCardsMusicSites} onChange={e => setTxtSettingsCardsMusicSites(e.target.value)} placeholder="soundcloud" />
                  </div>
                </div>
              </div>

              <div style={{ gridArea: '1/1', visibility: tabSettingsActive === "tags" ? 'visible' : 'hidden', zIndex: tabSettingsActive === "tags" ? 1 : 0, background: "var(--dlg-bgd)", transition: 'none' }}>
                <TagColorsTab s={s} t={t} clrSettingsTags={clrSettingsTags} setClrSettingsTags={setClrSettingsTags} allTags={[...new Set(cards.flatMap(p => p.tags))].sort()} />
              </div>

              <div className="dlg-about" style={{ gridArea: '1/1', visibility: tabSettingsActive === "about" ? 'visible' : 'hidden', zIndex: tabSettingsActive === "about" ? 1 : 0, background: "var(--dlg-bgd)", transition: 'none' }}>
                <img src={yaiolLogo} alt="Yaiol" style={{ width: 120, height: "auto", flexShrink: 0 }} />
                <div style={{ display: "flex", flexDirection: "column", gap: 6, minWidth: 0 }}>
                  <div className="dlg-about-id">{APP_NAME} <b>v{APP_VERSION}</b> by yaiol</div>
                  <div className="dlg-about-desc">{t("msgDlgSettingsAboutDesc")}</div>
                  <div className="dlg-about-version">{t("msgDlgSettingsAboutLocalDb")}</div>
                </div>
              </div>

              <div style={{ gridArea: '1/1', visibility: tabSettingsActive === "ai" ? 'visible' : 'hidden', zIndex: tabSettingsActive === "ai" ? 1 : 0, background: "var(--dlg-bgd)", transition: 'none' }}>
                {/* API Key */}
                <div className="dlg-field" style={{ paddingTop: 20 }}>
                  <label className="dlg-field-label" style={{ display: "flex", alignItems: "center", gap: 6 }}><Sparkles className="icon-inline" />{t("lblDlgSettingsAiApiKey")}</label>
                  <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                    <div style={{ display: "flex", gap: 8 }}>
                      <input type="password" className="input" placeholder={t("plhDlgSettingsAiApiKey")}
                        value={txtSettingsAiApiKey}
                        onChange={(e) => { setTxtSettingsAiApiKey(e.target.value); localStorage.setItem(`${STORAGE_PREFIX}-gemini-key`, e.target.value); apiFetch("/settings", { method: "POST", body: JSON.stringify({ [`${STORAGE_PREFIX}-gemini-key`]: e.target.value }) }); }}
                        style={{ flex: 1, fontFamily: "monospace" }}
                      />
                      {txtSettingsAiApiKey && (
                        <button onClick={() => { setTxtSettingsAiApiKey(""); localStorage.removeItem(`${STORAGE_PREFIX}-gemini-key`); apiFetch("/settings", { method: "POST", body: JSON.stringify({ [`${STORAGE_PREFIX}-gemini-key`]: "" }) }); }} className="btn">{t("btnDlgSettingsAiApiKeyClear")}</button>
                      )}
                    </div>
                    {txtSettingsAiApiKey
                      ? <p className="hint" style={{ color: "var(--ok)" }}>{t("msgDlgSettingsAiApiKeyEnabled")}</p>
                      : <p className="hint">{t("msgDlgSettingsAiApiKeyGetKey")}</p>
                    }
                  </div>
                </div>

                {/* Default interface mode */}
                <div className="dlg-field" style={{ borderTop: `1px solid var(--border)`, paddingTop: 16 }}>
                  <label className="dlg-field-label">{t("lblDlgSettingsAiTemplates")}</label>
                  <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 12 }}>
                    <span className="hint" style={{ flex: 1 }}>{t("lblDlgSettingsAiTemplatesMode")} - {t("btnDlgSettingsAiTemplatesModeSimple")} / {t("btnDlgSettingsAiTemplatesModeExpert")}</span>
                    <div className="barh-grp">
                      {[{ v: false, label: t("btnDlgSettingsAiTemplatesModeSimple") }, { v: true, label: t("btnDlgSettingsAiTemplatesModeExpert") }].map((opt) => (
                        <button key={String(opt.v)} onClick={() => { setTglSettingsAiTemplatesMode(opt.v); localStorage.setItem(`${STORAGE_PREFIX}-ai-expert-mode`, opt.v ? "1" : "0"); apiFetch("/settings", { method: "POST", body: JSON.stringify({ [`${STORAGE_PREFIX}-ai-expert-mode`]: opt.v ? "1" : "0" }) }); }}
                          className={`btn${tglSettingsAiTemplatesMode === opt.v ? " active" : ""}`}>
                          {opt.label}
                        </button>
                      ))}
                    </div>
                  </div>
                </div>

                {/* Character limits */}
                <div className="dlg-field" style={{ borderTop: `1px solid var(--border)`, paddingTop: 16 }}>
                  <label className="dlg-field-label">{t("lblDlgSettingsAiLimits")}</label>
                  <div style={{ display: "flex", gap: 12 }}>
                    {[
                      [t("lblDlgSettingsAiLimitsMusic"),    txtSettingsAiLimitsMusic,    v => { setTxtSettingsAiLimitsMusic(v);    localStorage.setItem(`${STORAGE_PREFIX}-ai-limit-music`,    v); apiFetch("/settings", { method: "POST", body: JSON.stringify({ [`${STORAGE_PREFIX}-ai-limit-music`]:    v }) }); }, 100, 3000, 50],
                      [t("lblDlgSettingsAiLimitsVocal"), txtSettingsAiLimitsVocal, v => { setTxtSettingsAiLimitsVocal(v); localStorage.setItem(`${STORAGE_PREFIX}-ai-limit-vocal`, v); apiFetch("/settings", { method: "POST", body: JSON.stringify({ [`${STORAGE_PREFIX}-ai-limit-vocal`]: v }) }); }, 100, 3000, 50],
                    ].map(([label, val, save, min, max, step]) => (
                      <div key={label} style={{ flexShrink: 0 }}>
                        <label className="hint" style={{ display: "block", marginBottom: 6 }}>{label}</label>
                        <NumberField min={min} max={max} step={step} value={val} width={90} onChange={save} />
                      </div>
                    ))}
                  </div>
                </div>

                {/* Warn limit */}
                <div className="dlg-field" style={{ paddingTop: 16 }}>
                  <label className="dlg-field-label">{t("lblDlgSettingsAiStyleThreshold")}</label>
                  <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
                    <NumberField min={100} max={5000} step={50} value={txtSettingsAiStyleThreshold} width={90}
                      onChange={v => { setTxtSettingsAiStyleThreshold(v); localStorage.setItem(`${STORAGE_PREFIX}-ai-warn-limit`, v); apiFetch("/settings", { method: "POST", body: JSON.stringify({ [`${STORAGE_PREFIX}-ai-warn-limit`]: v }) }); }} />
                    <p className="hint">{t("msgDlgSettingsAiStyleThresholdDesc")}</p>
                  </div>
                </div>

                {/* Style word threshold */}
                <div className="dlg-field" style={{ paddingTop: 16 }}>
                  <label className="dlg-field-label">{t("lblDlgSettingsAiStyleWordThreshold")}</label>
                  <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
                    <NumberField min={10} max={1000} step={10} value={txtSettingsAiStyleWordThreshold} width={90}
                      onChange={v => { setTxtSettingsAiStyleWordThreshold(v); localStorage.setItem(`${STORAGE_PREFIX}-ai-style-word-threshold`, v); apiFetch("/settings", { method: "POST", body: JSON.stringify({ [`${STORAGE_PREFIX}-ai-style-word-threshold`]: v }) }); }} />
                    <p className="hint">{t("msgDlgSettingsAiStyleWordThresholdDesc")}</p>
                  </div>
                </div>

                {/* Song lyric threshold */}
                <div className="dlg-field" style={{ paddingTop: 16 }}>
                  <label className="dlg-field-label">{t("lblDlgSettingsAiLyricsThreshold")}</label>
                  <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
                    <NumberField min={500} max={20000} step={50} value={txtSettingsAiLyricsThreshold} width={90}
                      onChange={v => { setTxtSettingsAiLyricsThreshold(v); localStorage.setItem(`${STORAGE_PREFIX}-ai-lyric-threshold`, v); apiFetch("/settings", { method: "POST", body: JSON.stringify({ [`${STORAGE_PREFIX}-ai-lyric-threshold`]: v }) }); }} />
                    <p className="hint">{t("msgDlgSettingsAiLyricsThresholdDesc")}</p>
                  </div>
                </div>

                {/* History */}
                <div className="dlg-field" style={{ paddingTop: 16 }}>
                  <label className="dlg-field-label">{t("lblDlgSettingsAiHistory")}</label>
                  <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
                    <p className="hint">{JSON.parse(localStorage.getItem(`${STORAGE_PREFIX}-ai-history`) || "[]").length} {t("lblDlgSettingsAiHistoryCount")}</p>
                    <button className="btn subtle" onClick={() => { localStorage.removeItem(`${STORAGE_PREFIX}-ai-history`); showToast(t("tstDlgSettingsAiHistoryCleared")); }}>{t("btnDlgSettingsAiHistoryClear")}</button>
                  </div>
                </div>
              </div>

            </div>
          </div>
        </div>
        );
      })()}

      {bulkDeleteConfirm && (() => {
        const orphanedSongs = getOrphanedSongs(selectedIds);
        return (
          <div className="dl-backdrop" onMouseDown={(e) => { if (e.target === e.currentTarget) setBulkDeleteConfirm(false); }}>
            <div className="dlp" onKeyDown={(e) => { if (e.key !== 'Tab') return; const btns = [...e.currentTarget.querySelectorAll('button')]; const first = btns[0]; const last = btns[btns.length - 1]; if (e.shiftKey) { if (document.activeElement === first) { e.preventDefault(); last.focus(); } } else { if (document.activeElement === last) { e.preventDefault(); first.focus(); } } }}>
              <button className="dl-close" onClick={() => setBulkDeleteConfirm(false)}><X /></button>
              <Trash2 color="var(--danger)" className="dlp-icon" />
              <p className="dlp-title">{t("btnCardDelete")} {selectedIds.length} {t("cfmDlgDeleteSelected")}</p>
              {orphanedSongs.length > 0 ? (
                <div style={{ marginBottom: 12, textAlign: "left", width: "100%" }}>
                  <p className="dlp-sub" style={{ marginBottom: 6, color: "var(--danger)" }}>⚠ {t("msgDlgDeleteLinkedWarning")}</p>
                  <div style={{ maxHeight: 180, overflowY: "auto", display: "flex", flexDirection: "column", gap: 3 }}>
                    {orphanedSongs.map(s2 => (
                      <span key={s2.id} style={{ fontSize: 12, color: 'var(--text)', padding: "2px 0" }}>
                        • {s2.name}<span style={{ fontSize: 10, fontWeight: 700, opacity: 0.6, marginLeft: 4 }}>v{s2.version || 1}</span>
                      </span>
                    ))}
                  </div>
                </div>
              ) : (
                <p className="dlp-sub">{t("cfmDlgDeleteCardSub")}</p>
              )}
              <div className="dlp-foot">
                <button className="btn subtle dlg-confirm-cancel-default" ref={el => { if (el && !el.dataset.didFocus) { el.dataset.didFocus = "1"; el.focus(); } }} onClick={() => setBulkDeleteConfirm(false)}>{t("btnGlbCancel")}</button>
                <button className="btn primary" style={{ "--accent": "var(--danger)", "--accent-hov": "color-mix(in srgb, var(--danger) 82%, black)" }} onClick={handleBulkDelete}><Trash2 style={{ marginRight: 6 }} />{t("btnDlgDeleteAll")}</button>
              </div>
            </div>
          </div>
        );
      })()}

      <GlobalPlayer
        ref={globalPlayerRef}
        media={playerMedia} apiPort={getApiPort()} langKey={langKey}
        playlistIndex={playlistIndex} playlistLength={playlist.length}
        hasPrev={playlistIndex > 0}
        hasNext={playlistIndex < playlist.length - 1}
        onPrev={() => { const i = playlistIndex - 1; setPlaylistIndex(i); setPlayerMedia(playlist[i]); }}
        onNext={() => { const i = playlistIndex + 1; setPlaylistIndex(i); setPlayerMedia(playlist[i]); }}
        onJumpTo={(i) => { setPlaylistIndex(i); setPlayerMedia(playlist[i]); }}
        onPlayingChange={setGlobalPlaying}
        onEnded={() => { setPlayerMedia(null); setGlobalPlaying(false); }}
        onClose={() => { setPlayerMedia(null); setPlaylist([]); setGlobalPlaying(false); }}
      />
      {importErrors && (
        <div className="dl-backdrop" onMouseDown={(e) => { if (e.target === e.currentTarget) setImportErrors(null); }}>
          <div className="dlg" style={{ maxWidth: 520 }}>
            <div style={{ ...s.modalHead, padding: "16px 20px 14px" }}>
              <h2 className="dlg-title" style={{ margin: 0 }}><AlertTriangle color="var(--danger)" />{t("ttlAppImportJSONWarnings")}</h2>
              <button className="dl-close" ref={el => { if (el && !el.dataset.didFocus) { el.dataset.didFocus = "1"; el.focus(); } }} onClick={() => setImportErrors(null)}><X /></button>
            </div>
            <div style={{ overflowY: "auto", maxHeight: 360, padding: "12px 20px", display: "flex", flexDirection: "column", gap: 6 }}>
              {importErrors.map((msg, i) => (
                <div key={i} style={{ fontSize: 12, color: 'var(--text)', fontFamily: "'JetBrains Mono', monospace", background: 'var(--bg-input)', border: `1px solid var(--border)`, borderRadius: 6, padding: "6px 10px" }}>{msg}</div>
              ))}
            </div>
            <div style={{ padding: "12px 20px", borderTop: `1px solid var(--border)`, display: "flex", justifyContent: "flex-end" }}>
              <button className="btn subtle" onClick={() => setImportErrors(null)}>{t("btnGlbCancel")}</button>
            </div>
          </div>
        </div>
      )}
      {toast}
    </div>
  );
}

// ── ProjectItem ────────────────────────────────────────────────────────────────
function ProjectItem({ proj, isActive, isDropTarget, draggingId, langKey, totalCount, onSelect, onDrop, onDragProjectStart, onDragProjectEnd }) {
  const [dragOver, setDragOver] = useState(false);
  const s = makeStyles();

  return (
    <div style={{ display: "flex", alignItems: "center", margin: "1px 4px", position: "relative" }}
      onDragOver={(e) => { if (draggingId) { e.preventDefault(); setDragOver(true); } }}
      onDragLeave={(e) => { if (!e.currentTarget.contains(e.relatedTarget)) setDragOver(false); }}
      onDrop={(e) => {
        e.preventDefault(); setDragOver(false);
        try { const ids = JSON.parse(e.dataTransfer.getData("cardIds")); if (ids?.length) { onDrop(ids); return; } } catch(err) {}
        const id = e.dataTransfer.getData("cardId"); if (id) onDrop([id]);
      }}
    >
      <div
        draggable
        onDragStart={(e) => {
          e.dataTransfer.setData("projectid", proj.id);
          e.dataTransfer.effectAllowed = "move";
          const ghost = document.createElement("div");
          ghost.innerText = "📁 " + proj.name;
          ghost.style.cssText = `position:fixed;top:-200px;left:0;padding:6px 14px;background:#1a1a2e;color:${proj.color};border:1px solid ${proj.color};border-radius:8px;font-size:13px;font-family:inherit;white-space:nowrap;`;
          document.body.appendChild(ghost);
          e.dataTransfer.setDragImage(ghost, 10, 20);
          setTimeout(() => { if (document.body.contains(ghost)) document.body.removeChild(ghost); }, 200);
          setTimeout(() => onDragProjectStart(), 0);
        }}
        onDragEnd={() => onDragProjectEnd()}
        style={{ cursor: "grab", padding: "0 4px", color: 'var(--text-mute)', display: "flex", alignItems: "center", flexShrink: 0, fontSize: 10 }}
      >⠿</div>
      {/* Row / selected state from the SHARED .lv-item; the folder ICON keeps its per-folder
          color; only the drag/drop affordances stay inline (not part of the list spec). */}
      <button onClick={(e) => onSelect(e)} className={"lv-item" + (isActive ? " active" : "")} style={{ margin: 0, flex: 1, minWidth: 0, ...(isDropTarget ? { background: 'var(--accent)' + "22", borderColor: 'var(--accent)', outline: `2px dashed var(--accent)` } : dragOver ? { background: 'var(--bg-hov)', borderColor: proj.color + "66", outline: `2px dashed ${proj.color}` } : {}) }}>
        <Folder color={proj.color} />
        <span className="lv-item-name" style={{ flex: 1 }}>{proj.name}{proj.version > 1 && <span style={{ fontSize: 9, fontWeight: 700, color: proj.color, border: `1px solid ${proj.color}55`, borderRadius: 4, padding: '1px 4px', marginLeft: 4, opacity: 0.85 }}>v{proj.version}</span>}</span>
        <span className="lv-item-badge">{totalCount ?? proj.cardIds.length}</span>
      </button>
    </div>
  );
}

// ── TagColorsTab ───────────────────────────────────────────────────────────────
function TagColorsTab({ s, t, clrSettingsTags, setClrSettingsTags, allTags }) {
  const [input, setInput] = useState("");
  const [pendingColor, setPendingColor] = useState(accentDefault());
  const suggestions = input.trim() ? allTags.filter(tg => tg.toLowerCase().includes(input.toLowerCase()) && !clrSettingsTags[tg]) : [];

  const add = () => {
    const tag = input.trim().toLowerCase();
    if (!tag) return;
    setClrSettingsTags(prev => ({ ...prev, [tag]: pendingColor }));
    setInput(""); setPendingColor(accentDefault());
  };

  const remove = (tag) => setClrSettingsTags(prev => { const n = { ...prev }; delete n[tag]; return n; });

  const update = (tag, color) => setClrSettingsTags(prev => ({ ...prev, [tag]: color }));

  const overrides = Object.entries(clrSettingsTags).sort((a, b) => a[0].localeCompare(b[0]));

  return (
    <div style={{ paddingTop: 20 }}>
      <div className="dlg-field">
        <label className="dlg-field-label" style={{ display: "flex", alignItems: "center", gap: 6 }}><Tag className="icon-inline" />{t("ttlDlgSettingsTagColors")}</label>
        <p className="hint" style={{ marginBottom: 12 }}>{t("msgDlgSettingsTagColorsDesc")}</p>
        {/* Add row */}
        <div style={{ display: "flex", gap: 8, alignItems: "center", marginBottom: 12, position: "relative" }}>
          <div style={{ flex: 1 }}>
            <Combobox
              placeholder={t("plhDlgSettingsTagColor")}
              value={input}
              onChange={setInput}
              onKeyDown={(e) => { if (e.key === "Enter") add(); }}
              items={suggestions.slice(0, 10)}
              itemKey={tg => tg}
              renderItem={tg => tg}
              onPick={setInput}
            />
          </div>
          <ColorPicker color={pendingColor} onChange={setPendingColor} cancelLabel={t("btnGlbCancel")} applyLabel={t("btnDlgCardApply")} pickTitle={t("tipGlbPickFromScreen")} />
          <button className="btn primary" onClick={add}>{t("btnDlgSettingsTagColorAdd")}</button>
        </div>
        {/* Override list - maxHeight cap prevents the grid cell from growing unboundedly (DLG-8 Part 2) */}
        <div style={{ maxHeight: 260, overflowY: "auto" }}>
        {overrides.length === 0
          ? <p className="hint">{t("msgDlgSettingsTagColorEmpty")}</p>
          : <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
              {overrides.map(([tag, color]) => (
                <div key={tag} style={{ display: "flex", alignItems: "center", gap: 8, padding: "5px 10px", borderRadius: 7, border: `1px solid var(--border-strong)`, background: 'var(--bg-input)' }}>
                  <span style={{ flex: 1, fontSize: 12, color, fontWeight: 600 }}>{tag}</span>
                  <ColorPicker color={color} onChange={(c) => update(tag, c)} cancelLabel={t("btnGlbCancel")} applyLabel={t("btnDlgCardApply")} pickTitle={t("tipGlbPickFromScreen")} />
                  <button className="btn icon small subtle" onClick={() => remove(tag)} title={t("tipDlgSettingsTagColorRemove")}><X /></button>
                </div>
              ))}
            </div>
        }
        </div>
      </div>
    </div>
  );
}

// ── ProjectModal ───────────────────────────────────────────────────────────────
function ProjectModal({ project, parentId, isSubFolder, langKey, apiPort, onSave, onClose }) {
  const t = useT(langKey);
  const [name, setName]   = useState(project?.name || "");
  const [color, setColor] = useState(project?.color || accentDefault());
  const [dateVal, setDateVal] = useState(() => project?.date ? new Date(project.date).toISOString().slice(0, 10) : new Date().toISOString().slice(0, 10));
  const [projPath, setProjPath] = useState(project?.path || "");
  const [finalized, setFinalized] = useState(project?.finalized || false);
  const [urls, setUrls] = useState(project?.urls || []);
  const [newUrlLabel, setNewUrlLabel] = useState("");
  const [newUrlHref, setNewUrlHref] = useState("");
  const [error, setError] = useState("");
  const s = makeStyles();

  const browsePath = async () => {
    const data = await fetch(`http://localhost:${apiPort}/open-folder-dialog`).then(r => r.json());
    if (data.path) setProjPath(data.path);
  };

  const handleAddUrl = async () => {
    if (!newUrlHref.trim()) return;
    const href = newUrlHref.trim().startsWith("http") ? newUrlHref.trim() : "https://" + newUrlHref.trim();
    let autoLabel = newUrlLabel.trim();
    if (!autoLabel) {
      try { autoLabel = new URL(href).hostname.replace("www.", "").replace(/\.(com|ai)$/, ""); } catch { autoLabel = "Link"; }
    }
    if (project?.id) {
      const res = await fetch(`http://localhost:${apiPort}/projects/${project.id}/urls`, {
        method: "POST", headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ href, label: autoLabel })
      });
      const data = await res.json();
      setUrls(data.urls || []);
    } else {
      setUrls(prev => [...prev, { id: Date.now().toString(), label: autoLabel, href }]);
    }
    setNewUrlHref(""); setNewUrlLabel("");
  };

  const handleRemoveUrl = async (urlId) => {
    if (project?.id) {
      const res = await fetch(`http://localhost:${apiPort}/projects/${project.id}/urls/${urlId}`, { method: "DELETE" });
      const data = await res.json();
      setUrls(data.urls || []);
    } else {
      setUrls(prev => prev.filter(u => u.id !== urlId));
    }
  };

  const submit = () => {
    if (!name.trim()) { setError("Name is required"); return; }
    const date = dateVal ? new Date(dateVal).getTime() : project?.date;
    const resolvedParentId = project ? (project.parentId ?? null) : (parentId ?? null);
    onSave({ name: name.trim(), color, date, parentId: resolvedParentId, path: projPath, urls, finalized });
  };

  return (
    <div className="dl-backdrop" onMouseDown={(e) => { if (e.target === e.currentTarget) onClose(); }}>
      <div className="dlg" style={{ maxWidth: 480, padding: 28, gap: 20 }} onKeyDown={(e) => { if (e.key !== 'Tab') return; const els = [...e.currentTarget.querySelectorAll('button, input, select, textarea, [tabindex]:not([tabindex="-1"])')].filter(el => !el.disabled); const first = els[0]; const last = els[els.length - 1]; if (e.shiftKey) { if (document.activeElement === first) { e.preventDefault(); last.focus(); } } else { if (document.activeElement === last) { e.preventDefault(); first.focus(); } } }}>
        <div style={s.modalHead}>
          <h2 className="dlg-title" style={{ margin: 0 }}>
            <FolderPlus color={color} />{project ? t("ttlDlgProjectEdit") : isSubFolder ? "New Sub-folder" : t("ttlDlgProjectNew")}
          </h2>
          <button className="dl-close" onClick={onClose}><X /></button>
        </div>
        <div className="dlg-field">
          <label className="dlg-field-label">{t("lblDlgProjectName")}</label>
          <input className="input" style={error ? s.errBorder : undefined}
            placeholder={t("plhDlgProjectName")} value={name}
            onChange={(e) => { setName(e.target.value); setError(""); }} autoFocus />
          {error && <span style={s.errMsg}>{error}</span>}
        </div>
        <div className="dlg-field">
          <label className="dlg-field-label">{t("lblDlgProjectColor")}</label>
          <ColorPicker color={color} onChange={setColor} cancelLabel={t("btnGlbCancel")} applyLabel={t("btnDlgCardApply")} pickTitle={t("tipGlbPickFromScreen")} />
        </div>
        <div className="dlg-field">
          <label className="dlg-field-label" style={{ color: color, display: "flex", alignItems: "center", gap: 5 }}><Globe className="icon-inline" />{t("lblDlgCardExternalLinks")} <span style={{ color: 'var(--text-mute)', textTransform: "none", letterSpacing: 0, fontWeight: 400 }}>({t("msgDlgCardOptional")})</span></label>
          <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            {urls.map((u) => (
              <UrlRow key={u.id} url={u} accent={color} onRemove={handleRemoveUrl} />
            ))}
            <div style={{ display: "flex", gap: 6 }}>
              <input className="input" style={{ width: 80, flexShrink: 0 }} placeholder={t("plhDlgCardUrlLabel")} value={newUrlLabel} onChange={(e) => setNewUrlLabel(e.target.value)} />
              <input className="input" style={{ flex: 1 }} placeholder="https://…" value={newUrlHref} onChange={(e) => setNewUrlHref(e.target.value)} onKeyDown={(e) => { if (e.key === "Enter") { e.preventDefault(); handleAddUrl(); } }} />
              <button className="btn primary" onClick={handleAddUrl} disabled={!newUrlHref.trim()} style={{ "--accent": color, "--accent-hov": color, flexShrink: 0 }}>{t("btnDlgCardAdd")}</button>
            </div>
          </div>
        </div>
        <div className="dlg-field">
          <label className="dlg-field-label">{t("lblDlgProjectPath")}</label>
          <div style={{ display: "flex", gap: 6 }}>
            <input className="input" style={{ flex: 1 }}
              placeholder={t("plhDlgProjectPath")} value={projPath}
              onChange={(e) => setProjPath(e.target.value)} />
            <button className="btn subtle" style={{ flexShrink: 0 }} onClick={browsePath}>
              <FolderOpen />
            </button>
          </div>
        </div>
        <div style={{ display: "flex", justifyContent: "flex-end", flexDirection: "column", gap: 10 }}>
          {project && (
            <div style={{ display: "flex", gap: 12, alignItems: "center", width: "100%" }}>
              <DatePicker value={dateVal} onChange={setDateVal} locale={langKey} todayLabel={t('btnGlbToday')} clearLabel={t('btnGlbClear')} />
              <span style={{ fontWeight: 700, color: color, fontSize: 11 }}>v{project.version || 1}</span>
              <button className="btn small subtle" type="button" onClick={() => setFinalized(f => !f)} title={t("tipDlgProjectFinalized")}
                style={{ color: 'var(--text)' }}>
                {finalized ? <CheckSquare color={color} /> : <Square color={'var(--text-mute)'} />}
                <span>{t("lblDlgProjectFinalized")}</span>
              </button>
            </div>
          )}
          <div style={{ display: "flex", gap: 10, width: "100%", justifyContent: "flex-end" }}>
            <button className="btn subtle" onClick={onClose}>{t("btnGlbCancel")}</button>
            <button className="btn primary" style={{ "--accent": color, "--accent-hov": color }} onClick={submit}>{project ? t("btnDlgProjectSave") : t("btnDlgProjectCreate")}</button>
          </div>
        </div>
      </div>
    </div>
  );
}

// ── GlobalPlayer ───────────────────────────────────────────────────────────────
const GlobalPlayer = forwardRef(function GlobalPlayer({ media, apiPort, langKey, playlistIndex, playlistLength, hasPrev, hasNext, onPrev, onNext, onJumpTo, onEnded, onClose, onPlayingChange }, ref) {
  const t = useT(langKey);
  const audioRef = useRef(null);
  const [playing, setPlaying] = useState(false);
  const setPlayingSync = (val) => { setPlaying(val); onPlayingChange?.(val); };
  const [currentTime, setCurrentTime] = useState(0);
  const [duration, setDuration] = useState(0);
  const [volume, setVolume] = useState(() => parseFloat(localStorage.getItem(`${STORAGE_PREFIX}-volume`) ?? "1"));
  const [muted, setMuted] = useState(false);
  const [shuffle, setShuffle] = useState(() => localStorage.getItem(`${STORAGE_PREFIX}-shuffle`) === "1");
  // repeat: "off" | "all" | "one"
  const [repeat, setRepeat] = useState(() => localStorage.getItem(`${STORAGE_PREFIX}-repeat`) || "off");
  const port = apiPort || "3001";
  const streamUrl = media ? `http://localhost:${port}/stream?path=${encodeURIComponent(media.path)}` : null;

  useEffect(() => {
    if (!audioRef.current || !streamUrl) return;
    audioRef.current.load();
    audioRef.current.play().then(() => setPlayingSync(true)).catch(() => setPlayingSync(false));
    setCurrentTime(0);
    setDuration(0);
  }, [streamUrl]);

  useEffect(() => {
    if (!audioRef.current) return;
    audioRef.current.volume = muted ? 0 : volume;
  }, [volume, muted]);

  const togglePlay = () => {
    if (!audioRef.current) return;
    if (playing) { audioRef.current.pause(); setPlayingSync(false); }
    else { audioRef.current.play().then(() => setPlayingSync(true)).catch(() => {}); }
  };

  useImperativeHandle(ref, () => ({ togglePlay }));

  const handleSeek = (e) => {
    if (!audioRef.current || !duration) return;
    const rect = e.currentTarget.getBoundingClientRect();
    const ratio = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
    audioRef.current.currentTime = ratio * duration;
  };

  const handleVolume = (e) => {
    const v = parseFloat(e.target.value);
    setVolume(v);
    setMuted(v === 0);
    localStorage.setItem(`${STORAGE_PREFIX}-volume`, v);
  };

  const toggleMute = () => setMuted(m => !m);

  const toggleShuffle = () => setShuffle(s => {
    const next = !s;
    localStorage.setItem(`${STORAGE_PREFIX}-shuffle`, next ? "1" : "0");
    return next;
  });

  const cycleRepeat = () => setRepeat(r => {
    const next = r === "off" ? "all" : r === "all" ? "one" : "off";
    localStorage.setItem(`${STORAGE_PREFIX}-repeat`, next);
    return next;
  });

  const randomIndex = () => {
    if (playlistLength <= 1) return 0;
    let next;
    do { next = Math.floor(Math.random() * playlistLength); } while (next === playlistIndex);
    return next;
  };

  const handleEnded = () => {
    setPlayingSync(false);
    if (repeat === "one") {
      audioRef.current.currentTime = 0;
      audioRef.current.play().then(() => setPlayingSync(true)).catch(() => {});
      return;
    }
    if (shuffle && playlistLength > 1) { onJumpTo && onJumpTo(randomIndex()); return; }
    if (playlistIndex < playlistLength - 1) { onNext && onNext(); return; }
    if (repeat === "all" && playlistLength > 0) { onJumpTo && onJumpTo(0); return; }
    onEnded && onEnded();
  };

  const handlePrev = () => {
    if (shuffle && playlistLength > 1) { onJumpTo && onJumpTo(randomIndex()); }
    else if (hasPrev) { onPrev && onPrev(); }
  };

  const handleNext = () => {
    if (shuffle && playlistLength > 1) { onJumpTo && onJumpTo(randomIndex()); }
    else if (hasNext) { onNext && onNext(); }
  };

  const fmt = (s) => {
    if (!s || isNaN(s)) return "0:00";
    const m = Math.floor(s / 60);
    const ss = Math.floor(s % 60);
    return `${m}:${ss.toString().padStart(2, "0")}`;
  };

  const canNav = shuffle ? playlistLength > 1 : null;
  const VolumeIcon = muted || volume === 0 ? VolumeX : volume < 0.5 ? Volume1 : Volume2;
  const RepeatIcon = repeat === "one" ? Repeat2 : Repeat;

  if (!media) return null;

  return createPortal(
    <div className="barh-footer" style={{ position: "fixed", bottom: 0, left: 0, right: 0, zIndex: 150 }}> {/* shared canonical footer bar — ui-app.css */}
      <audio
        ref={audioRef}
        src={streamUrl}
        onTimeUpdate={() => setCurrentTime(audioRef.current?.currentTime || 0)}
        onLoadedMetadata={() => setDuration(audioRef.current?.duration || 0)}
        onEnded={handleEnded}
      />
      {/* Shuffle */}
      <button className={"btn icon small subtle" + (shuffle ? " active" : "")} onClick={toggleShuffle} style={{ position: "relative" }} title={shuffle ? t("tipFtrShuffleOn") : t("tipFtrShuffleOff")}>
        <Shuffle className="icon-inline" />
        {shuffle && <span style={{ position: "absolute", bottom: 1, right: 2, width: 4, height: 4, borderRadius: "50%", background: 'var(--accent)' }} />}
      </button>
      <button className="btn icon small subtle" onClick={handlePrev} title={t("tipFtrPrevious")}>
        <ChevronLeft />
      </button>
      <button className="btn icon primary" onClick={togglePlay}>
        {playing ? <Pause /> : <Play />}
      </button>
      <button className="btn icon small subtle" onClick={() => { if (audioRef.current) { audioRef.current.pause(); audioRef.current.currentTime = 0; } setPlayingSync(false); }} title={t("tipFtrStop")}
        style={{ color: "var(--danger)" }}>
        <Square />
      </button>
      <button className="btn icon small subtle" onClick={handleNext} title={t("tipFtrNext")}>
        <ChevronRight />
      </button>
      {/* Repeat */}
      <button className={"btn icon small subtle" + (repeat !== "off" ? " active" : "")} onClick={cycleRepeat} style={{ position: "relative" }} title={repeat === "off" ? t("tipFtrRepeatOff") : repeat === "all" ? t("tipFtrRepeatAll") : t("tipFtrRepeatOne")}>
        <RepeatIcon className="icon-inline" />
        {repeat === "one" && <span style={{ position: "absolute", top: 1, right: 0, fontSize: 8, fontWeight: 700, color: 'var(--accent)', lineHeight: 1 }}>1</span>}
        {repeat !== "off" && repeat !== "one" && <span style={{ position: "absolute", bottom: 1, right: 2, width: 4, height: 4, borderRadius: "50%", background: 'var(--accent)' }} />}
      </button>
      <span style={{ fontSize: 12, fontWeight: 600, color: 'var(--text)', flexShrink: 0, maxWidth: 200, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }} title={media.cardName}>
        {media.cardName}
      </span>
      <input type="range" min={0} max={duration || 1} step={0.1}
        value={currentTime}
        onChange={e => { const v = parseFloat(e.target.value); if (audioRef.current) audioRef.current.currentTime = v; setCurrentTime(v); }}
        style={{ flex: 1, height: 4, accentColor: 'var(--accent)', cursor: "pointer" }} />
      <span className="hint" style={{ flexShrink: 0, fontVariantNumeric: "tabular-nums" }}>
        {fmt(currentTime)} / {fmt(duration)}
      </span>
      <div className="barh-sep" />
      <button className="btn icon small subtle" onClick={toggleMute} title={muted ? t("tipFtrUnmute") : t("tipFtrMute")}>
        <VolumeIcon />
      </button>
      <input
        type="range" min="0" max="1" step="0.02"
        value={muted ? 0 : volume}
        onChange={handleVolume}
        style={{ width: 72, height: 4, accentColor: 'var(--accent)', cursor: "pointer", flexShrink: 0 }}
        title={`${t("tipFtrVolume")}: ${Math.round((muted ? 0 : volume) * 100)}%`}
      />
      <button className="btn icon small subtle" onClick={onClose}>
        <X />
      </button>
    </div>,
    document.body
  );
});

// ── Play button helper - single source of truth for media-missing state ───────
function playBtnProps(mediaExists, isPlaying, accentColor, t) {
  const missing = !mediaExists;
  return {
    icon: missing ? <AlertTriangle /> : isPlaying ? <Pause /> : <Play />,
    color: missing ? "var(--danger)" : accentColor,
    borderColor: missing ? "color-mix(in srgb, var(--danger) 27%, transparent)" : `${accentColor}44`,
    title: missing ? t("tipCardMediaMissing") : isPlaying ? t("tipCardPause") : t("tipCardPlay"),
    disabled: missing,
  };
}

// ── Lettered copy glyphs — a lucide "copy" icon with a T / S / L badge, one per copyable field ──
// (title / style / lyrics). Single base so the three share one geometry; only the letter varies.
const CopyLetter = ({ letter }) => (
  <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <rect width="14" height="14" x="8" y="8" rx="2" ry="2" />
    <path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2" />
    <text x="11.7" y="18.8" fontSize="10.67" fontFamily="Arial, sans-serif" fill="currentColor" strokeWidth="0.6">{letter}</text>
  </svg>
);
const CopyTitle  = () => <CopyLetter letter="T" />;
const CopyStyle  = () => <CopyLetter letter="S" />;
const CopyLyrics = () => <CopyLetter letter="L" />;

// ── SongListCard - single source for song rows across the list view and the three special views ──
// mode: "list" = full card (checkbox + toolbar + lyrics/tags). "link" / "media" / "sort" = stripped
// single-feature card — number + title + version + the feature's own control only, no generic buttons.
function SongListCard({ song, types, langKey, apiPort, lrcExists, mediaExists, isPlaying, txtSettingsCardsAiSites, txtSettingsCardsMusicSites, clrSettingsTags,
  onEdit, onPlay, onFav, onCopy, onDelete, onToggleSelect, onRemoveFromProject, copiedId,
  isSelected, activeProjectId, allCards, linkSchema, globalEnvId,
  isDragging, anyDragging, mode,
  // list-only
  onDragStart, onDragEnd, linkDropHandlers, displayText,
  // link-only
  onLink, onUnlink,
  // media-only
  onSetMedia, onClearMedia,
  // sort-only
  onReorder, isLast,
  // special views — greyed + inert when the project is finalized (locked)
  locked }) {

  const isSpecial = mode !== "list"; // link / media / sort — stripped single-feature card

  const t = useT(langKey);
  const s = makeStyles();
  const isSong = song.type === "song";
  const isNote = song.type === "note";
  const cfg = isSong ? getSongState(song, types, txtSettingsCardsAiSites, txtSettingsCardsMusicSites) : (types[song.type] || types.SongStateUndefined);
  const isGlobalCard = globalEnvId && song.env === globalEnvId;
  const linkedCards = (song.linkedCards || []).map((id) => (allCards || []).find((p) => p.id === id)).filter(Boolean)
    .sort((a, b) => { const tr = (t) => t === "note" ? 0 : t === "music" ? 1 : t === "vocal" ? 2 : 3; const td = tr(a.type) - tr(b.type); return td !== 0 ? td : a.name.localeCompare(b.name); });
  const isLinkedSong = isSong && linkedCards.length > 0;
  const linkedByCount = (song.type === 'music' || song.type === 'vocal')
    ? (allCards || []).filter(p => (p.linkedCards || []).includes(song.id)).length : 0;
  const tagColor = (tag) => (clrSettingsTags && clrSettingsTags[tag]) ? clrSettingsTags[tag] : getTagColor(tag);
  const [over, setOver] = useState(false);
  const [sortInsert, setSortInsert] = useState(null); // "before" | "after" | null — sort-view drop indicator
  // Insert side for a sort drop: "before" for every card, "after" only in the last card's bottom half.
  const sortDropPos = (clientY, r) => (isLast && clientY >= r.top + r.height / 2) ? "after" : "before";
  const filename = song.mediaPath ? song.mediaPath.split(/[\\/]/).pop() : null;

  const linkedBadges = isSong && linkedCards.length > 0
    ? <div style={{ display: "flex", gap: 4, flexShrink: 0 }}>
        {linkedCards.map(lp => { const lc = types[lp.type]; return (
          <span key={lp.id} title={lp.name + " v" + (lp.version || 1)} style={{ fontSize: 10, border: `1px solid ${lc.accent}44`, borderRadius: 4, padding: "2px 6px", color: lc.accent, background: lc.accentDim, display: "inline-flex", alignItems: "center", gap: 3, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", maxWidth: 120 }}>{lp.name}<span style={{ opacity: 0.65, fontWeight: 700, flexShrink: 0 }}>v{lp.version || 1}</span></span>
        ); })}
      </div>
    : !isSong && linkedByCount > 0
      ? <div style={{ display: "flex", gap: 4, alignItems: "center", flexShrink: 0 }}>
          <Link2 className="icon-inline" color={'var(--text-mute)'} /><span style={{ fontSize: 10, border: `1px solid ${types.SongStateUndefined?.accent}44`, borderRadius: 4, padding: "2px 6px", color: types.SongStateUndefined?.accent, background: types.SongStateUndefined?.accentDim }}>{linkedByCount}</span>
        </div>
      : null;

  // Drag/drop handlers per mode
  const dropHandlers = mode === "link" ? {
    onDragOver: (e) => { if (!e.dataTransfer.types.includes("linkable")) return; e.preventDefault(); setOver(true); },
    onDragLeave: () => setOver(false),
    onDrop: (e) => { if (!e.dataTransfer.types.includes("linkable")) return; e.preventDefault(); setOver(false); try { const ids = JSON.parse(e.dataTransfer.getData("cardIds")); if (ids?.length) { onLink(song.id, ids); return; } } catch {} const id = e.dataTransfer.getData("cardId"); if (id) onLink(song.id, [id]); },
  } : mode === "media" ? {
    onDragOver: (e) => { if (!e.dataTransfer.types.includes("mediafile")) return; e.preventDefault(); setOver(true); },
    onDragLeave: () => setOver(false),
    onDrop: (e) => { if (!e.dataTransfer.types.includes("mediafile")) return; e.preventDefault(); setOver(false); const fp = e.dataTransfer.getData("mediafile"); if (fp) onSetMedia(song.id, fp); },
  } : mode === "sort" ? {
    draggable: true,
    onDragStart: (e) => onDragStart && onDragStart(e, song.id),
    onDragEnd,
    // One indicator per gap: always "before" this card, except the last card's bottom half → "after"
    // (so the end of the list is reachable). Avoids the two-position flicker in every inner gap.
    onDragOver: (e) => { e.preventDefault(); e.dataTransfer.dropEffect = "move"; setSortInsert(sortDropPos(e.clientY, e.currentTarget.getBoundingClientRect())); },
    onDragLeave: (e) => { if (!e.currentTarget.contains(e.relatedTarget)) setSortInsert(null); },
    onDrop: (e) => { e.preventDefault(); const pos = sortDropPos(e.clientY, e.currentTarget.getBoundingClientRect()); const id = e.dataTransfer.getData("cardId"); setSortInsert(null); if (id) onReorder && onReorder(id, song.id, pos); },
  } : {
    draggable: true,
    onDragStart: onDragStart ? (e) => onDragStart(e, song.id) : undefined,
    onDragEnd,
    ...(linkDropHandlers || {}),
  };

  const overStyle = over && anyDragging ? { borderColor: cfg.accent, boxShadow: `0 0 0 2px ${cfg.accent}33` } : {};

  // Mode-specific placeholder (only the part that varies)
  const modePlaceholder = mode === "link" ? (
    linkedCards.length === 0 ? (
      <div style={{ border: `1px dashed ${anyDragging ? cfg.accent + "88" : 'var(--border)'}`, borderRadius: 6, padding: "6px 12px", textAlign: "center", fontSize: 11, color: anyDragging ? cfg.accent : 'var(--text-mute)', transition: "all 0.2s", flex: 1, minWidth: 0 }}>
        {anyDragging ? t("lblPnlLinkDrop") : t("lblPnlLinkNoCards")}
      </div>
    ) : (
      <div style={{ display: "flex", flexWrap: "wrap", gap: 5, flex: 1, minWidth: 0 }}>
        {linkedCards.map(lp => {
          const lc = types[lp.type];
          const { Icon: LIcon } = lc;
          return (
            <span key={lp.id} style={{ fontSize: 11, border: `1px solid ${lc.accent}44`, borderRadius: 4, padding: "3px 6px 3px 8px", color: lc.accent, background: lc.accentDim, display: "inline-flex", alignItems: "center", gap: 4 }}>
              <LIcon className="icon-inline" />{lp.name}<span style={{ opacity: 0.65, fontWeight: 700 }}>v{lp.version || 1}</span>
              <button className="btn icon small subtle" onClick={() => onUnlink(song.id, lp.id)} title={t("tipPnlLinkUnlink")} style={{ color: lc.accent + "99" }}>
                <X />
              </button>
            </span>
          );
        })}
      </div>
    )
  ) : mode === "media" ? (
    filename ? (
      <div style={{ display: "flex", alignItems: "center", gap: 6, border: `1px solid ${cfg.accent}44`, borderRadius: 6, padding: "4px 10px", background: cfg.accentDim, flex: 1, minWidth: 0 }}>
        <Disc className="icon-inline" color={cfg.accent} style={{ flexShrink: 0 }} />
        <span style={{ fontSize: 11, color: cfg.accent, flex: 1, minWidth: 0, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }} title={song.mediaPath}>{filename}</span>
        <button className="btn icon small subtle" onClick={() => onClearMedia(song.id)} title={t("tipPnlLinkRemoveMedia")} style={{ color: cfg.accent + "99" }}><X /></button>
      </div>
    ) : (
      <div style={{ border: `1px dashed ${anyDragging ? cfg.accent + "88" : 'var(--border)'}`, borderRadius: 6, padding: "6px 12px", textAlign: "center", fontSize: 11, color: anyDragging ? cfg.accent : 'var(--text-mute)', transition: "all 0.2s", flex: 1, minWidth: 0 }}>
        {anyDragging ? t("lblPnlLinkDropMedia") : t("lblPnlLinkNoMedia")}
      </div>
    )
  ) : mode === "sort" ? null : (
    displayText ? <p style={{ ...s.cardBody, margin: 0, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", fontSize: 11, opacity: 0.7, flex: 1, minWidth: 0 }}>{displayText}</p> : null
  );

  // Second line: feature placeholder always; tags + linked badges only on the full list card
  const secondLine = (
    <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
      {modePlaceholder}
      {!isSpecial && song.tags?.length > 0 && <div style={{ display: "flex", gap: 4, flexShrink: 0 }}>
        {song.tags.slice(0, 3).map((tag) => { const c = tagColor(tag); return <span key={tag} style={{ ...s.inlineTag, color: c, borderColor: c + "55", fontSize: 10 }}>{tag}</span>; })}
        {song.tags.length > 3 && <span className="hint">+{song.tags.length - 3}</span>}
      </div>}
      {!isSpecial && linkedBadges}
    </div>
  );

  return (
    <div
      style={{ ...s.card, "--card-accent": cfg.accent, opacity: isDragging ? 0.5 : 1, cursor: "grab", flexDirection: "row", alignItems: "center", gap: 12, padding: "10px 16px", ...(isSelected ? { borderColor: 'var(--accent)' + "88", background: 'var(--bg-elev)' } : {}), ...overStyle, ...(mode === "sort" ? { position: "relative" } : {}), ...(locked ? { pointerEvents: "none", opacity: 0.45 } : {}) }}
      className="card-item"
      {...(locked ? {} : dropHandlers)}
    >
      {/* Sort drop line — centered in the 8px gap so it reads as a clear line BETWEEN the two cards */}
      {mode === "sort" && sortInsert && (
        <div style={{ position: "absolute", left: 0, right: 0, height: 3, borderRadius: 2, background: 'var(--accent)', pointerEvents: "none", zIndex: 2, ...(sortInsert === "before" ? { top: -5 } : { bottom: -5 }) }} />
      )}
      {mode === "list" && (
        <button className="btn icon small subtle" style={{ color: isSelected ? 'var(--accent)' : 'var(--text-mute)' }} onClick={(e) => { e.stopPropagation(); onToggleSelect && onToggleSelect(song.id); }}>
          {isSelected ? <CheckSquare /> : <Square />}
        </button>
      )}

      {/* Left: edit, play, lrc, name+version, lang + second line below */}
      <div style={{ display: "flex", flexDirection: "column", flex: 1, minWidth: 0, gap: 2 }}>
        <div style={{ display: "flex", alignItems: "center", gap: 7 }}>
          {!isSpecial && <>
            <button className="btn icon small" onClick={() => onEdit(song)} title={t("tipCardEdit")}>
              <Pencil />
            </button>
            {(() => { const pb = playBtnProps(mediaExists, isPlaying, cfg.accent, t); return (
              <button className="btn icon small" style={{ visibility: song.mediaPath ? "visible" : "hidden", color: pb.color, borderColor: pb.borderColor, background: isPlaying ? cfg.accentDim : "transparent" }} onClick={() => !pb.disabled && onPlay && onPlay(song)} title={pb.title} tabIndex={song.mediaPath ? 0 : -1}>
                {pb.icon}
              </button>
            ); })()}
            <button className="btn icon small" style={{ visibility: isSong && song.mediaPath && cleanLyrics(song.lyrics || "").trim() ? "visible" : "hidden", color: lrcExists ? cfg.accent : 'var(--text-mute)', borderColor: lrcExists ? `${cfg.accent}44` : `var(--text-mute)44` }}
              onClick={() => fetch(`http://localhost:${apiPort}/launch-lrc-editor`, {
                method: 'POST', headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ mediaPath: song.mediaPath, lyrics: song.lyrics ? cleanLyrics(song.lyrics) : undefined }),
              })}
              title={t("tipCardOpenLrcEditor")}
              tabIndex={isSong && song.mediaPath ? 0 : -1}>
              <Disc />
            </button>
            <button className="btn icon small subtle" style={{ color: song.favorite ? "var(--star)" : 'var(--text-mute)' }} onClick={() => onFav && onFav(song.id)}><Star fill={song.favorite ? "var(--star)" : "none"} /></button>
          </>}
          <h3 style={{ ...s.cardTitle, margin: 0, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", fontSize: 15, color: cfg.accent, flex: 1, minWidth: 0, display: "flex", alignItems: "center", gap: 4 }}>{song.sortNumber != null ? `${String(song.sortNumber).padStart(2, "0")}-${song.name}` : song.name}<span style={{ fontSize: 9, fontWeight: 700, color: cfg.accent, border: `1px solid ${cfg.accent}55`, borderRadius: 4, padding: '1px 5px', opacity: 0.85, flexShrink: 0 }}>v{song.version || 1}</span></h3>
          {!isSpecial && song.lang && <span style={{ fontSize: 9, fontWeight: 700, color: 'var(--text-mute)', border: `1px solid var(--border)`, borderRadius: 4, padding: '1px 5px', opacity: 0.85, flexShrink: 0 }}>{song.lang}</span>}
        </div>
        {mode !== "sort" && secondLine}
      </div>
      {/* Right side - two rows, 4 slots each, aligned — full list card only, stripped in the special views */}
      {!isSpecial && (() => {
        const aiList    = (txtSettingsCardsAiSites    || "udio, suno, producer, tunee").split(",").map(s => s.trim().toLowerCase()).filter(Boolean);
        const musicList = (txtSettingsCardsMusicSites || "soundcloud").split(",").map(s => s.trim().toLowerCase()).filter(Boolean);
        const aiUrl    = (song.urls || []).find(u => aiList.includes((u.label || "").toLowerCase().trim()));
        const musicUrl = (song.urls || []).find(u => musicList.includes((u.label || "").toLowerCase().trim()));
        const otherUrl = (song.urls || []).find(u => { const lbl = (u.label || "").toLowerCase().trim(); return !aiList.includes(lbl) && !musicList.includes(lbl); });
        const emptySlot = <span style={{ visibility: "hidden" }} />;
        return (
          <div style={{ display: "grid", gridTemplateColumns: "repeat(5, 1fr)", gap: 4, flexShrink: 0, alignSelf: "start" }}>
            {/* Row 1: copy title, copy style, copy lyrics, unlink, delete */}
            <button className="btn icon small" style={{ ...(copiedId === song.id + "_title" ? { background: "color-mix(in srgb, var(--ok) 14%, transparent)", color: "var(--ok)", borderColor: "var(--ok)" } : {}) }}
              onClick={() => onCopy && onCopy(song, "title")}
              title={t("tipCardCopyTitle")}>
              {copiedId === song.id + "_title" ? <Check /> : <CopyTitle />}
            </button>
            {!isNote ? <button className="btn icon small" style={{ ...(copiedId === (isLinkedSong ? song.id + "_linked" : song.id) ? { background: "color-mix(in srgb, var(--ok) 14%, transparent)", color: "var(--ok)", borderColor: "var(--ok)" } : {}), visibility: (isSong ? (song.style?.trim() || linkedCards.some(lp => lp.style?.trim())) : song.style?.trim()) ? "visible" : "hidden" }}
              onClick={() => onCopy && onCopy(song, isLinkedSong ? "linked" : undefined)}
              title={t("tipCardCopyStyle")}
              tabIndex={(isSong ? (song.style?.trim() || linkedCards.some(lp => lp.style?.trim())) : song.style?.trim()) ? 0 : -1}>
              {copiedId === (isLinkedSong ? song.id + "_linked" : song.id) ? <Check /> : <CopyStyle />}
            </button> : emptySlot}
            {!isNote ? <button className="btn icon small" style={{ ...(copiedId === song.id + "_lyrics" ? { background: "color-mix(in srgb, var(--ok) 14%, transparent)", color: "var(--ok)", borderColor: "var(--ok)" } : {}), visibility: isSong && song.lyrics ? "visible" : "hidden" }}
              onClick={() => onCopy && onCopy(song, "lyrics")}
              title={t("tipCardCopyLyrics")}
              tabIndex={isSong && song.lyrics ? 0 : -1}>
              {copiedId === song.id + "_lyrics" ? <Check /> : <CopyLyrics />}
            </button> : emptySlot}
            {onRemoveFromProject
              ? <button className="btn icon small" style={{ color: "var(--warn)", borderColor: "var(--warn)" }} onClick={() => onRemoveFromProject(activeProjectId, song.id)} title={t("tipCardUnlink")}>
                  <X />
                </button>
              : emptySlot}
            <button className="btn icon small" style={{ color: "var(--danger)", borderColor: "var(--danger)" }} onClick={() => onDelete && onDelete(song.id)} title={t("tipCardDelete")}>
              <Trash2 />
            </button>
            {/* Row 2: globe, ai url, music url, other url */}
            {isGlobalCard
              ? <button className="btn icon small" style={{ cursor: "default" }} title={t("lblCardGlobalResource")}><Globe color={'var(--accent)'} /></button>
              : emptySlot}
            {aiUrl
              ? <button title={aiUrl.label + ": " + aiUrl.href} className="btn icon small" style={{ borderColor: types.SongStateCreated?.accent + "66", color: types.SongStateCreated?.accent, background: types.SongStateCreated?.accentDim }} onClick={(e) => { e.stopPropagation(); window.open(aiUrl.href, "_blank"); }}>
                  <span style={{ fontWeight: 700, lineHeight: 1 }}>{(aiUrl.label || "?")[0].toUpperCase()}</span>
                </button>
              : emptySlot}
            {musicUrl
              ? <button title={musicUrl.label + ": " + musicUrl.href} className="btn icon small" style={{ borderColor: types.SongStatePublished?.accent + "66", color: types.SongStatePublished?.accent, background: types.SongStatePublished?.accentDim }} onClick={(e) => { e.stopPropagation(); window.open(musicUrl.href, "_blank"); }}>
                  <span style={{ fontWeight: 700, lineHeight: 1 }}>{(musicUrl.label || "?")[0].toUpperCase()}</span>
                </button>
              : emptySlot}
            {otherUrl
              ? <button title={otherUrl.label + ": " + otherUrl.href} className="btn icon small" style={{ borderColor: 'var(--text-mute)' + "66", color: 'var(--text-mute)', background: "transparent" }} onClick={(e) => { e.stopPropagation(); window.open(otherUrl.href, "_blank"); }}>
                  <span style={{ fontWeight: 700, lineHeight: 1 }}>{(otherUrl.label || "?")[0].toUpperCase()}</span>
                </button>
              : emptySlot}
            {emptySlot}
          </div>
        );
      })()}
    </div>
  );
}

// ── CardItem ─────────────────────────────────────────────────────────────────
function CardItem({ card, cfg, copiedId, allCards, isDragging, anyDragging, isSelected, activeProjectId, types, viewMode, langKey, urlSchema, linkSchema, apiPort, lrcExists, mediaExists, isPlaying, clrSettingsTags, txtSettingsCardsAiSites, txtSettingsCardsMusicSites, globalEnvId, onCopy, onFav, onEdit, onDelete, onDragStart, onDragEnd, onLinkCard, onToggleSelect, onRemoveFromProject, onPlay }) {
  const t = useT(langKey);
  if (!types) types = buildTypes(DEFAULT_TYPE_COLORS);
  cfg = cfg || types.music;
  const [expanded, setExpanded] = useState(false);
  const [linkOver, setLinkOver] = useState(false);
  const s = makeStyles();
  const tagColor = (tag) => (clrSettingsTags && clrSettingsTags[tag]) ? clrSettingsTags[tag] : getTagColor(tag);
  const port = apiPort || localStorage.getItem(`${STORAGE_PREFIX}-api-port`) || "3001";
  const streamUrl = card.mediaPath ? `http://localhost:${port}/stream?path=${encodeURIComponent(card.mediaPath)}` : null;
  const { Icon } = cfg;
  const isList = viewMode === "list";
  const isNote = card.type === "note";
  const displayText = isNote ? (card.desc || "") : (card.style || card.lyrics || "");
  const preview = displayText.length > 160 ? displayText.slice(0, 160) + "…" : displayText;
  const linkedCards = (card.linkedCards || []).map((id) => allCards.find((p) => p.id === id)).filter(Boolean)
    .sort((a, b) => { const tr = (t) => t === "note" ? 0 : t === "music" ? 1 : t === "vocal" ? 2 : 3; const td = tr(a.type) - tr(b.type); return td !== 0 ? td : a.name.localeCompare(b.name); });

  const urlButtons = card.urls && card.urls.length > 0
    ? <div style={{ display: "flex", gap: 3, flexShrink: 0 }}>
        {card.urls.map((u) => (
          <button key={u.id}
            title={u.label + ": " + u.href}
            className="btn small"
            style={{ color: cfg.accent, borderColor: `${cfg.accent}44`, background: cfg.accentDim }}
            onClick={(e) => { e.stopPropagation(); window.open(u.href, "_blank"); }}
          >
            {isList
              ? <span style={{ fontWeight: 700, lineHeight: 1 }}>{(u.label || "?")[0].toUpperCase()}</span>
              : <><ExternalLink /><span style={{ fontWeight: 600 }}>{u.label}</span></>
            }
          </button>
        ))}
      </div>
    : null;
  const linkedByCount = (card.type === 'music' || card.type === 'vocal')
    ? allCards.filter(p => (p.linkedCards || []).includes(card.id)).length
    : 0;

  const isSong = card.type === "song";
  const isLinkedSong = isSong && linkedCards.length > 0;
  const isGlobalCard = !isSong && globalEnvId && card.env === globalEnvId;
  const linkDropHandlers = isSong ? {
    onDragOver: (e) => { if (!e.dataTransfer.types.includes("linkable")) return; e.preventDefault(); setLinkOver(true); },
    onDragLeave: () => setLinkOver(false),
    onDrop: (e) => {
      if (!e.dataTransfer.types.includes("linkable")) return;
      e.preventDefault(); setLinkOver(false);
      try { const ids = JSON.parse(e.dataTransfer.getData("cardIds")); if (ids?.length) { onLinkCard(card.id, ids); return; } } catch {}
      const id = e.dataTransfer.getData("cardId"); if (id) onLinkCard(card.id, [id]);
    }
  } : {};
  const actionButtons = (
    <>
      {/* Title/copy button - always available (every card has a name) */}
      <button className="btn icon small" style={{ ...(copiedId === card.id + "_title" ? { background: "color-mix(in srgb, var(--ok) 14%, transparent)", color: "var(--ok)", borderColor: "var(--ok)" } : {}) }}
        onClick={() => onCopy(card, "title")}
        title={t("tipCardCopyTitle")}>
        {copiedId === card.id + "_title" ? <Check /> : <CopyTitle />}
      </button>
      {/* Style/copy button - hidden for note type; invisible placeholder when style is empty */}
      {!isNote && <button className="btn icon small" style={{ ...(copiedId === (isLinkedSong ? card.id + "_linked" : card.id) ? { background: "color-mix(in srgb, var(--ok) 14%, transparent)", color: "var(--ok)", borderColor: "var(--ok)" } : {}), visibility: (isSong ? (card.style?.trim() || linkedCards.some(lp => lp.style?.trim())) : card.style?.trim()) ? "visible" : "hidden" }}
        onClick={() => onCopy(card, isLinkedSong ? "linked" : undefined)}
        title={t("tipCardCopyStyle")}
        tabIndex={(isSong ? (card.style?.trim() || linkedCards.some(lp => lp.style?.trim())) : card.style?.trim()) ? 0 : -1}>
        {copiedId === (isLinkedSong ? card.id + "_linked" : card.id) ? <Check /> : <CopyStyle />}
      </button>}
      {/* Lyrics button - visible for song with lyrics, invisible placeholder for music/vocal, hidden for note */}
      {!isNote && <button className="btn icon small" style={{ ...(copiedId === card.id + "_lyrics" ? { background: "color-mix(in srgb, var(--ok) 14%, transparent)", color: "var(--ok)", borderColor: "var(--ok)" } : {}), visibility: isSong && card.lyrics ? "visible" : "hidden" }}
        onClick={() => onCopy(card, "lyrics")}
        title={t("tipCardCopyLyrics")}
        tabIndex={isSong && card.lyrics ? 0 : -1}>
        {copiedId === card.id + "_lyrics" ? <Check /> : <CopyLyrics />}
      </button>}
      {onRemoveFromProject && (
        <button className="btn icon small" style={{ color: "var(--warn)", borderColor: "var(--warn)" }} onClick={onRemoveFromProject} title={t("tipCardUnlink")}>
          <X />
        </button>
      )}
      <button className="btn icon small" style={{ color: "var(--danger)", borderColor: "var(--danger)" }} onClick={() => onDelete(card.id)} title={t("tipCardDelete")}>
        <Trash2 />
      </button>
    </>
  );

  // ── List layout ──────────────────────────────────────────────────────────────
  if (isList) {
    return (
      <SongListCard song={card} types={types} langKey={langKey} apiPort={apiPort}
        lrcExists={lrcExists} mediaExists={mediaExists} isPlaying={isPlaying} txtSettingsCardsAiSites={txtSettingsCardsAiSites} txtSettingsCardsMusicSites={txtSettingsCardsMusicSites}
        clrSettingsTags={clrSettingsTags} onEdit={onEdit} onPlay={onPlay} onFav={onFav} onCopy={onCopy} onDelete={onDelete}
        onToggleSelect={onToggleSelect} onRemoveFromProject={onRemoveFromProject} copiedId={copiedId}
        isSelected={isSelected} activeProjectId={activeProjectId} allCards={allCards}
        linkSchema={linkSchema} globalEnvId={globalEnvId} isDragging={isDragging} anyDragging={anyDragging}
        mode="list" displayText={displayText}
        onDragStart={onDragStart} onDragEnd={onDragEnd} linkDropHandlers={linkDropHandlers} />
    );
  }

  // ── Grid layout (default) ────────────────────────────────────────────────────
  return (
    <div
      style={{ ...s.card, "--card-accent": cfg.accent, opacity: isDragging ? 0.5 : 1, cursor: "grab", ...(isSelected ? { borderColor: 'var(--accent)' + "88", boxShadow: `0 0 0 2px ${'var(--accent)' + "22"}` } : {}), ...(linkOver && anyDragging ? { borderColor: cfg.accent, boxShadow: `0 0 0 2px ${cfg.accent}44` } : {}) }}
      className="card-item"
      draggable
      onDragStart={(e) => onDragStart(e, card.id)}
      onDragEnd={onDragEnd}
      {...linkDropHandlers}
    >
      <div style={{ display: "flex", gap: 6, flex: 1, minHeight: 0 }}>
        {/* Left: checkbox + content */}
        <div style={{ display: "flex", flex: 1, minWidth: 0, gap: 6 }}>
          <button className="btn icon small subtle" style={{ color: isSelected ? 'var(--accent)' : 'var(--text-mute)' }}>
            {isSelected ? <CheckSquare /> : <Square />}
          </button>
          <div style={{ flex: 1, minWidth: 0 }}>
            <h3 style={{ ...s.cardTitle, color: cfg.accent, margin: "0 0 8px", display: "flex", alignItems: "center", gap: 4, flexWrap: "wrap" }}>{card.sortNumber != null ? `${String(card.sortNumber).padStart(2, "0")}-${card.name}` : card.name}<span style={{ fontSize: 9, fontWeight: 700, color: cfg.accent, border: `1px solid ${cfg.accent}55`, borderRadius: 4, padding: '1px 5px', marginLeft: 4, opacity: 0.85, flexShrink: 0 }}>v{card.version || 1}</span>{card.lang && <span style={{ fontSize: 9, fontWeight: 700, color: 'var(--text-mute)', border: `1px solid var(--border)`, borderRadius: 4, padding: '1px 5px', opacity: 0.85, flexShrink: 0 }}>{card.lang}</span>}{isGlobalCard && <Globe className="icon-inline" color={'var(--accent)'} title={t("lblCardGlobalResource")} style={{ marginLeft: 2, flexShrink: 0 }} />}</h3>
            <p style={{ ...s.cardBody, ...(expanded ? {} : { display: "-webkit-box", WebkitLineClamp: 8, WebkitBoxOrient: "vertical", overflow: "hidden" }) }}>
              {expanded ? displayText : preview}
              {displayText.length > 160 && (
                <button className="btn icon small subtle" onClick={() => setExpanded((v) => !v)} style={{ color: cfg.accent }}>
                  {expanded ? <ChevronUp /> : <ChevronDown />}
                </button>
              )}
            </p>
            {card.tags.length > 0 && (
              <div style={s.cardTags}>
                {card.tags.map((tag) => { const c = tagColor(tag); return <span key={tag} style={{ ...s.inlineTag, color: c, borderColor: c + "55" }}>{tag}</span>; })}
              </div>
            )}
            {!isNote && linkedCards.length > 0 && (
              <div style={{ display: "flex", flexWrap: "wrap", gap: 6, borderTop: `1px solid var(--border)`, paddingTop: 10, alignItems: "center" }}>
                <Link2 className="icon-inline" color={'var(--text-mute)'} />
                {linkedCards.map((lp) => {
                  const lc = types[lp.type];
                  const { Icon: LIcon } = lc;
                  return <span key={lp.id} style={{ fontSize: 11, border: `1px solid ${lc.accent}44`, borderRadius: 4, padding: "2px 8px", color: lc.accent, background: lc.accentDim, display: "inline-flex", alignItems: "center", gap: 4 }}><LIcon className="icon-inline" />{lp.name}<span style={{ opacity: 0.65, fontWeight: 700 }}>v{lp.version || 1}</span></span>;
                })}
              </div>
            )}
            {!isNote && !isSong && linkedByCount > 0 && (
              <div style={{ display: "flex", gap: 6, borderTop: `1px solid var(--border)`, paddingTop: 10, alignItems: "center" }}>
                <Link2 className="icon-inline" color={'var(--text-mute)'} />
                <span style={{ fontSize: 10, border: `1px solid ${types.SongStateUndefined?.accent}44`, borderRadius: 4, padding: "2px 6px", color: types.SongStateUndefined?.accent, background: types.SongStateUndefined?.accentDim }}>{linkedByCount}</span>
              </div>
            )}
          </div>
        </div>
        {/* Right: star + url icons stacked */}
        <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: 2, flexShrink: 0 }}>
          <button className="btn icon small subtle" style={{ color: card.favorite ? "var(--star)" : 'var(--text-mute)' }} onClick={() => onFav(card.id)}>
            <Star fill={card.favorite ? "var(--star)" : "none"} />
          </button>
          {(() => {
            const aiList    = (txtSettingsCardsAiSites    || "udio, suno, producer, tunee").split(",").map(s => s.trim().toLowerCase()).filter(Boolean);
            const musicList = (txtSettingsCardsMusicSites || "soundcloud").split(",").map(s => s.trim().toLowerCase()).filter(Boolean);
            const aiUrl    = (card.urls || []).find(u => aiList.includes((u.label || "").toLowerCase().trim()));
            const musicUrl = (card.urls || []).find(u => musicList.includes((u.label || "").toLowerCase().trim()));
            const otherUrl = (card.urls || []).find(u => { const lbl = (u.label || "").toLowerCase().trim(); return !aiList.includes(lbl) && !musicList.includes(lbl); });
            return <>
              {aiUrl    && <button title={aiUrl.label    + ": " + aiUrl.href}    className="btn icon small" style={{ borderColor: types.SongStateCreated?.accent + "66", color: types.SongStateCreated?.accent, background: types.SongStateCreated?.accentDim }} onClick={(e) => { e.stopPropagation(); window.open(aiUrl.href,    "_blank"); }}><span style={{ fontWeight: 700, lineHeight: 1 }}>{(aiUrl.label    || "?")[0].toUpperCase()}</span></button>}
              {musicUrl && <button title={musicUrl.label + ": " + musicUrl.href} className="btn icon small" style={{ borderColor: types.SongStatePublished?.accent + "66", color: types.SongStatePublished?.accent, background: types.SongStatePublished?.accentDim }} onClick={(e) => { e.stopPropagation(); window.open(musicUrl.href, "_blank"); }}><span style={{ fontWeight: 700, lineHeight: 1 }}>{(musicUrl.label || "?")[0].toUpperCase()}</span></button>}
              {otherUrl && <button title={otherUrl.label + ": " + otherUrl.href} className="btn icon small" style={{ borderColor: 'var(--text-mute)' + "66", color: 'var(--text-mute)', background: "transparent" }} onClick={(e) => { e.stopPropagation(); window.open(otherUrl.href, "_blank"); }}><span style={{ fontWeight: 700, lineHeight: 1 }}>{(otherUrl.label || "?")[0].toUpperCase()}</span></button>}
            </>;
          })()}
        </div>
      </div>
      <div style={{ ...s.cardActions, justifyContent: "space-between" }}>
        <div style={{ display: "flex", gap: 8 }}>
          <button className="btn icon small" onClick={() => onEdit(card)} title={t("tipCardEdit")}>
            <Pencil />
          </button>
          {(() => { const pb = playBtnProps(mediaExists, isPlaying, cfg.accent, t); return (
            <button className="btn icon small" style={{ visibility: streamUrl ? "visible" : "hidden", color: pb.color, borderColor: pb.borderColor, background: isPlaying ? cfg.accentDim : "transparent" }} onClick={() => !pb.disabled && onPlay && onPlay(card)} title={pb.title} tabIndex={streamUrl ? 0 : -1}>
              {pb.icon}
            </button>
          ); })()}
          <button className="btn icon small" style={{ visibility: isSong && card.mediaPath && cleanLyrics(card.lyrics || "").trim() ? "visible" : "hidden", color: lrcExists ? cfg.accent : 'var(--text-mute)', borderColor: lrcExists ? `${cfg.accent}44` : `var(--text-mute)44` }}
            onClick={() => fetch(`http://localhost:${apiPort}/launch-lrc-editor`, {
              method: 'POST', headers: { 'Content-Type': 'application/json' },
              body: JSON.stringify({ mediaPath: card.mediaPath, lyrics: card.lyrics ? cleanLyrics(card.lyrics) : undefined }),
            })}
            title={t("tipCardOpenLrcEditor")}
            tabIndex={isSong && card.mediaPath ? 0 : -1}>
            <Disc />
          </button>
        </div>
        <div style={{ display: "flex", gap: 8 }}>
          {actionButtons}
        </div>
      </div>
    </div>
  );
}

// ── MediaLinkFileCard ─────────────────────────────────────────────────────────
function MediaLinkFileCard({ file, isDragging, linkedSong, types, txtSettingsCardsAiSites, txtSettingsCardsMusicSites, locked }) {
  const cfg = linkedSong ? getSongState(linkedSong, types, txtSettingsCardsAiSites, txtSettingsCardsMusicSites) : null;
  return (
    <div
      style={{ background: 'var(--bg-elev)', border: `1px solid var(--border)`, borderRadius: 8, padding: "8px 12px", display: "flex", alignItems: "center", gap: 8, cursor: "grab", opacity: isDragging ? 0.45 : 1, transition: "opacity 0.15s", userSelect: "none", ...(locked ? { pointerEvents: "none", opacity: 0.45 } : {}) }}
      draggable={!locked}
      onDragStart={locked ? undefined : (e) => { e.dataTransfer.setData("mediafile", file.path); e.dataTransfer.effectAllowed = "copy"; }}
    >
      <Disc className="icon-inline" color={'var(--text-mute)'} style={{ flexShrink: 0 }} />
      <span style={{ fontSize: 12, color: 'var(--text)', flex: 1, minWidth: 0, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }} title={file.path}>{file.name}</span>
      {linkedSong && cfg && (
        <span style={{ fontSize: 10, border: `1px solid ${cfg.accent}44`, borderRadius: 4, padding: "2px 6px", color: cfg.accent, background: cfg.accentDim, flexShrink: 0, maxWidth: 140, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}
          title={linkedSong.name}>
          {linkedSong.sortNumber != null ? `${String(linkedSong.sortNumber).padStart(2, "0")}-` : ""}{linkedSong.name}
        </span>
      )}
    </div>
  );
}

// ── LinkViewMVCard ────────────────────────────────────────────────────────────
function LinkViewMVCard({ card, types, isDragging, linkedByCount, onEdit, onDragStart, onDragEnd, langKey, locked }) {
  const t = useT(langKey);
  const cfg = types[card.type];
  const { Icon } = cfg;
  const songCfg = types.SongStateUndefined;
  const preview = (card.style || "").length > 100 ? (card.style || "").slice(0, 100) + "…" : (card.style || "");
  return (
    <div
      style={{ background: 'var(--bg-elev)', border: `1px solid var(--border)`, borderRadius: 10, padding: "12px 14px", display: "flex", flexDirection: "column", gap: 6, cursor: "grab", opacity: isDragging ? 0.45 : 1, transition: "opacity 0.15s", userSelect: "none", ...(locked ? { pointerEvents: "none", opacity: 0.45 } : {}) }}
      draggable={!locked}
      onDragStart={(e) => onDragStart(e, card.id)}
      onDragEnd={onDragEnd}
    >
      <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
        <Icon className="icon-inline" color={cfg.accent} />
        <span style={{ fontSize: 13, fontWeight: 700, color: cfg.accent, flex: 1, minWidth: 0, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{card.name}</span>
        <span style={{ fontSize: 10, color: cfg.accent, border: `1px solid ${cfg.accent}44`, borderRadius: 4, padding: "1px 5px", opacity: 0.8, flexShrink: 0 }}>v{card.version || 1}</span>
        <button className="btn icon small subtle" onClick={() => onEdit(card)} title={t("tipCardEdit")}>
          <Pencil />
        </button>
      </div>
      {preview && (
        <p style={{ margin: 0, fontSize: 11, color: 'var(--text-dim)', lineHeight: 1.5, overflow: "hidden", display: "-webkit-box", WebkitLineClamp: 3, WebkitBoxOrient: "vertical" }}>
          {preview}
        </p>
      )}
      {linkedByCount > 0 && (
        <div style={{ display: "flex", gap: 6, borderTop: `1px solid var(--border)`, paddingTop: 6, alignItems: "center" }}>
          <Link2 className="icon-inline" color={'var(--text-mute)'} />
          <span style={{ fontSize: 10, border: `1px solid ${songCfg?.accent}44`, borderRadius: 4, padding: "2px 6px", color: songCfg?.accent, background: songCfg?.accentDim, display: "inline-flex", alignItems: "center", gap: 3 }}>
            {linkedByCount}
          </span>
        </div>
      )}
    </div>
  );
}

// ── AI Engine v2 - multi-variable output ──────────────────────────────────────

// ⚠ CLAUDE: Universal parser - ONE parser for all AI responses.
// Detects UPPER_CASE_KEY: at start of line (inline value or next-line value both work).
// Content like "Tiken Jah: Fakoly" never matches because "Tiken" is not ALL_CAPS.
function parseResponse(raw) {
  const clean = raw.replace(/^```[^\n]*\n?/gm, "").replace(/```$/gm, "").trim();
  const lines = clean.split("\n");
  const bag = {};
  let curKey = null;
  let curLines = [];
  for (const line of lines) {
    const m = line.match(/^([A-Z][A-Z0-9_]*):\s*(.*)$/);
    if (m) {
      if (curKey) bag[curKey] = curLines.join("\n").trim();
      curKey = m[1];
      curLines = m[2] ? [m[2]] : [];
    } else if (curKey) {
      curLines.push(line);
    }
  }
  if (curKey) bag[curKey] = curLines.join("\n").trim();
  return bag;
}

async function callAi(request, apiKey, apiPort, label) {
  const resp = await fetch(`http://localhost:${apiPort}/gemini/generate`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ request, apiKey, raw: true, maxTokens: 8192 }),
  });
  if (!resp.ok) {
    const err = await resp.json().catch(() => ({ error: "Request failed" }));
    throw new Error(err.error || "Request failed");
  }
  const { text } = await resp.json();
  const bag = parseResponse(text);
  bag._raw = text;
  const filename = (label || "ai-call") + ".txt";
  fetch(`http://localhost:${apiPort}/debug-write`, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ filename, content: "=== REQUEST ===\n" + request + "\n\n=== RESPONSE ===\n" + (text || "") }) }).catch(() => {});
  return bag;
}

// Build a complete AI request for a given task using templates + presets + system blocks.
// Used by the wizard to assemble AI requests outside AiGenerateModal.
function buildTaskRequest(task, vars, templates, presets, systemBlocks) {
  const tpl = templates.find(t => t.task === task);
  if (!tpl) return "";
  const parts = [];
  for (const b of tpl.blocks) {
    if (!b.enabled) continue;
    const preset = presets.find(p => p.id === b.presetId);
    if (!preset?.content?.trim()) continue;
    const prefix = b.prefix || `${b.block.toUpperCase()}: `;
    parts.push(`${prefix}${preset.content.trim()}`);
  }
  for (const sys of systemBlocks) {
    const prefix = sys.prefix || `${sys.block.toUpperCase()}: `;
    const taskContent = sys.tasks?.[task]?.content;
    if (taskContent) parts.push(`${prefix}${taskContent}`);
    const taskVars = sys.tasks?.[task]?.vars;
    if (taskVars) {
      const lines = taskVars.map(v => `${v.name}: ${vars[v.name] || v.example || ""}`);
      parts.push(`${prefix}${lines.join("\n")}`);
    }
  }
  return parts.join("\n");
}

// Shared external-link row — label + url + open / copy / remove. The ONE definition used by
// every modal that lists a card's URLs (both card layouts + the project links), so the row
// never drifts. Accent + remove handler come from the caller; open/copy are self-contained.
function UrlRow({ url, accent, onRemove }) {
  const [copied, setCopied] = useState(false);
  const iconBtn = { background: "none", border: "none", cursor: "pointer", color: 'var(--text-mute)', padding: 2, display: "flex", flexShrink: 0 };
  return (
    <div style={{ display: "flex", alignItems: "center", gap: 8, padding: "6px 10px", borderRadius: 8, border: "1px solid var(--border)", background: "var(--dlg-inp)" }}>
      <span style={{ fontSize: 12, fontWeight: 600, color: accent, minWidth: 50, flexShrink: 0 }}>{url.label}</span>
      <span className="hint" style={{ flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{url.href}</span>
      <button className="btn icon small subtle" onClick={() => window.open(url.href, "_blank")}><ExternalLink /></button>
      <button className="btn icon small subtle" onClick={() => { navigator.clipboard.writeText(url.href); setCopied(true); setTimeout(() => setCopied(false), 1200); }}>{copied ? <Check color="var(--ok)" /> : <Copy />}</button>
      <button className="btn icon small subtle" onClick={() => onRemove(url.id)}><X /></button>
    </div>
  );
}

// ── CardModal ────────────────────────────────────────────────────────────────
function AiGenerateModal({ task, txtSettingsAiApiKey, apiPort, cardId, aiTweaks, defaultTemplateId, defaultExpertMode, initLimit, initLimitWords, langKey, onGenerate, onSaveTweaks, onSetDefault, onClose, showToast }) {
  const t = useT(langKey);
  const port = apiPort || localStorage.getItem(`${STORAGE_PREFIX}-api-port`) || "3001";
  const DEFAULT_BLOCKS_FOR_TASK = { music: ["role", "style", "rules"], vocal: ["role", "style", "rules"], lyrics: ["role", "structure", "style", "rules"] };
  const defaultBlocks = DEFAULT_BLOCKS_FOR_TASK[task] || ["role", "rules", "style"];
  const saved = aiTweaks?.[task] || {};

  const [templates, setTemplates]           = useState([]);
  const [presets, setPresets]               = useState({});
  const [selectedTemplateId, setSelectedTemplateId] = useState(saved.templateId || null);
  const [expertMode, setExpertMode]         = useState(defaultExpertMode || false);
  const [blockStates, setBlockStates]       = useState({});
  const [blockOrder, setBlockOrder]         = useState(defaultBlocks);
  const [blockMeta, setBlockMeta]           = useState({});
  const [systemBlocks, setSystemBlocks]     = useState([]);
  const [expandedBlocks, setExpandedBlocks] = useState(() => Object.fromEntries(defaultBlocks.map(b => [b, true])));
  const [userRequest, setUserRequest]       = useState(saved.request || "");
  const [loading, setLoading]               = useState(false);
  const [dataLoaded, setDataLoaded]         = useState(false);
  const [limit, setLimit]                   = useState(saved.limit ?? initLimit ?? 700);
  const [localDefaultId, setLocalDefaultId]           = useState(defaultTemplateId || null);
  const [savedTemplateBlocks, setSavedTemplateBlocks] = useState({});
  const [tplInputMode, setTplInputMode]     = useState(null); // "create" | "rename" | null
  const [tplInputValue, setTplInputValue]   = useState("");
  const [pendingDelete, setPendingDelete]   = useState(false);
  const [presetInputBlock, setPresetInputBlock]       = useState(null);
  const [presetInputMode, setPresetInputMode]         = useState(null); // "create" | "rename"
  const [presetInputValue, setPresetInputValue]       = useState("");
  const [presetPendingDelete, setPresetPendingDelete] = useState(null);
  const [varValues, setVarValues]                     = useState({});
  const prevLengthModeRef = useRef(null);

  const snapshotTplBlocks = (states, order) => Object.fromEntries((order || blockOrder).map(b => [b, { presetId: states[b]?.presetId || null, enabled: states[b]?.enabled !== false }]));

  const buildFromTemplate = (tpl, grouped) => {
    const tplBlocks = tpl.blocks || [];
    const order = tplBlocks.length ? tplBlocks.map(x => x.block) : defaultBlocks;
    const meta = Object.fromEntries(tplBlocks.map(x => [x.block, { name: x.name, prefix: x.prefix, hidden: x.hidden || false }]));
    const states = {};
    for (const b of order) {
      const def = tplBlocks.find(x => x.block === b);
      const pid = def?.presetId || null;
      const preset = pid ? (grouped[b] || []).find(p => p.id === pid) : null;
      const fallback = !preset ? ((grouped[b] || []).find(p => p.isDefaultPick) || (grouped[b] || [])[0] || null) : null;
      states[b] = { presetId: preset?.id || fallback?.id || null, content: preset?.content || fallback?.content || "", enabled: def?.enabled !== false };
    }
    return { states, order, meta };
  };

  useEffect(() => {
    Promise.all([
      fetch(`http://localhost:${port}/ai-templates?task=${task}`).then(r => r.json()),
      fetch(`http://localhost:${port}/ai-presets?task=${task}`).then(r => r.json()),
      fetch(`http://localhost:${port}/ai-system`).then(r => r.json()),
    ]).then(([tplData, presetData, systemData]) => {
      const grouped = {};
      for (const p of presetData) { if (!grouped[p.block]) grouped[p.block] = []; grouped[p.block].push(p); }
      setPresets(grouped);
      setTemplates(tplData);
      setSystemBlocks(systemData);
      const initId = (saved.templateId && tplData.find(t => t.id === saved.templateId))
        ? saved.templateId
        : (defaultTemplateId && tplData.find(t => t.id === defaultTemplateId))
          ? defaultTemplateId
          : (tplData[0]?.id || null);
      setSelectedTemplateId(initId);
      const tpl = tplData.find(t => t.id === initId);
      const { states: fromTpl, order, meta } = tpl ? buildFromTemplate(tpl, grouped) : { states: {}, order: defaultBlocks, meta: {} };
      setBlockOrder(order);
      setBlockMeta(meta);
      setExpandedBlocks(Object.fromEntries(order.map(b => [b, true])));
      if (saved.blocks && Object.keys(saved.blocks).length > 0) {
        const merged = {};
        for (const b of order) merged[b] = { ...fromTpl[b], content: saved.blocks[b] ?? fromTpl[b]?.content ?? "" };
        setBlockStates(merged);
      } else {
        setBlockStates(fromTpl);
      }
      setSavedTemplateBlocks(snapshotTplBlocks(fromTpl, order));
      setDataLoaded(true);
    }).catch(console.error);
  }, []); // run once on mount

  const handleTemplateChange = (id) => {
    setSelectedTemplateId(id);
    const tpl = templates.find(t => t.id === id);
    if (tpl) {
      const { states, order, meta } = buildFromTemplate(tpl, presets);
      setBlockOrder(order);
      setBlockMeta(meta);
      setExpandedBlocks(Object.fromEntries(order.map(b => [b, true])));
      setBlockStates(states);
      setSavedTemplateBlocks(snapshotTplBlocks(states, order));
    }
  };

  const handlePresetChange = (block, pid) => {
    const preset = pid ? (presets[block] || []).find(p => p.id === pid) : null;
    setBlockStates(prev => ({ ...prev, [block]: { ...prev[block], presetId: pid, content: preset?.content || "" } }));
  };

  const handleBlockToggle = (block) => setBlockStates(prev => ({ ...prev, [block]: { ...prev[block], enabled: !prev[block].enabled } }));

  const isModified = (block) => {
    const state = blockStates[block];
    if (!state) return false;
    const preset = state.presetId ? (presets[block] || []).find(p => p.id === state.presetId) : null;
    return preset ? state.content !== preset.content : state.content !== "";
  };

  const VAR_RE = /\[([^:\]]+):(num|txt|ckb):([^\]]*)\]/g;

  const allVars = (() => {
    const seen = new Set();
    const result = [];
    for (const b of blockOrder) {
      const s = blockStates[b];
      if (!s?.enabled || !s?.content) continue;
      let m;
      const re = new RegExp(VAR_RE.source, "g");
      while ((m = re.exec(s.content)) !== null) {
        const name = m[1];
        if (!seen.has(name)) {
          seen.add(name);
          result.push({ name, type: m[2], defaultVal: m[3] });
        }
      }
    }
    return result;
  })();

  // ⚠ CLAUDE: promptLengthMode is derived from template content - drives UI label/step and resolveVars replacements
  const promptLengthMode = (() => {
    const sources = [];
    for (const b of blockOrder) { const s = blockStates[b]; if (s?.enabled && s?.content) sources.push(s.content); }
    for (const sys of systemBlocks) { const tc = sys.tasks?.[task]?.content; if (tc) sources.push(tc); }
    const all = sources.join("\n");
    if (/\{prompt-chars-(max|interval)\}/.test(all)) return "chars";
    if (/\{prompt-words-(max|interval)\}/.test(all)) return "words";
    return null;
  })();

  useEffect(() => {
    if (!dataLoaded || promptLengthMode == null) return;
    const prev = prevLengthModeRef.current;
    prevLengthModeRef.current = promptLengthMode;
    if (prev == null) {
      // first resolve - honour saved limit; otherwise seed word default in words mode
      if (saved.limit != null) return;
      if (promptLengthMode === "words" && initLimitWords != null) setLimit(initLimitWords);
      return;
    }
    if (prev !== promptLengthMode) {
      if (promptLengthMode === "words" && initLimitWords != null) setLimit(initLimitWords);
      else if (promptLengthMode === "chars" && initLimit != null) setLimit(initLimit);
    }
  }, [dataLoaded, promptLengthMode]);

  const resolveVars = (content) => {
    content = content.replace(/\{prompt-chars-max\}/g, String(limit));
    content = content.replace(/\{prompt-chars-interval\}/g, `${limit - 50}-${limit}`);
    content = content.replace(/\{prompt-words-max\}/g, String(limit));
    content = content.replace(/\{prompt-words-interval\}/g, `${limit - 10}-${limit}`);
    const withVars = content.replace(new RegExp(VAR_RE.source, "g"), (_, name, type, def) => {
      const val = varValues[name];
      if (type === "ckb") return val === false ? "" : def;
      return val !== undefined && val !== "" ? val : def;
    });
    return withVars;
  };

  const assembleRequest = () => {
    const parts = [];
    for (const b of blockOrder) {
      const s = blockStates[b];
      if (s?.enabled && s?.content?.trim()) {
        const prefix = blockMeta[b]?.prefix || `${b.toUpperCase()}: `;
        parts.push(`${prefix}${resolveVars(s.content.trim())}`);
      }
    }
    // Append system blocks in JSON order
    for (const sys of systemBlocks) {
      const prefix = sys.prefix || `${sys.block.toUpperCase()}: `;
      const taskContent = sys.tasks?.[task]?.content ?? null;
      const taskVars = sys.tasks?.[task]?.vars ?? null;
      if (taskContent !== null) {
        parts.push(`${prefix}${resolveVars(taskContent)}`);
      } else if (taskVars) {
        const lines = taskVars.map(v =>
          `${v.name}: ${v.name === "USER_REQUEST" ? userRequest.trim() : (varValues[v.name] || v.example || "")}`
        );
        parts.push(`${prefix}${lines.join("\n")}`);
      } else if (!sys.tasks) {
        if (userRequest.trim()) parts.push(`${prefix}${userRequest.trim()}`);
      }
    }
    return parts.join("\n");
  };

  const buildTweaks = () => {
    const tweakBlocks = {};
    for (const b of blockOrder) { const s = blockStates[b]; if (s?.content) tweakBlocks[b] = s.content; }
    return { ...(aiTweaks || {}), [task]: { templateId: selectedTemplateId, blocks: tweakBlocks, request: userRequest, limit } };
  };

  const handleCopy = () => {
    if (!userRequest.trim()) return;
    onSaveTweaks(buildTweaks());
    navigator.clipboard.writeText(assembleRequest());
    showToast(t("tstDlgAiCardCopied"));
  };

  const handleGenerate = async () => {
    if (!userRequest.trim() || !txtSettingsAiApiKey) return;
    onSaveTweaks(buildTweaks());
    setLoading(true);
    try {
      const bag = await callAi(assembleRequest(), txtSettingsAiApiKey, port, task);
      const suggestions = bag.NAME
        ? bag.NAME.split("\n").map(l => l.replace(/^\d+\.\s*/, "").trim()).filter(Boolean).slice(0, 5)
        : [];
      const text = (bag.CONTENT ? bag.CONTENT.trim() : (bag._raw || "").trim());
      onGenerate(text, suggestions);
      onClose();
    } catch (err) {
      showToast(t("tstAppAiError") + ": " + err.message);
    } finally {
      setLoading(false);
    }
  };

  const handleSaveAsNew = (block) => {
    setPresetInputValue("");
    setPresetInputMode("create");
    setPresetInputBlock(block);
  };

  const handleRenamePreset = (block) => {
    const preset = (presets[block] || []).find(p => p.id === blockStates[block]?.presetId);
    if (!preset || preset.isDefault) return;
    setPresetInputValue(preset.name);
    setPresetInputMode("rename");
    setPresetInputBlock(block);
    setExpandedBlocks(prev => ({ ...prev, [block]: true }));
  };

  const handlePresetInputConfirm = async (block) => {
    const name = presetInputValue.trim();
    if (!name) return;
    if (presetInputMode === "create") {
      const res = await fetch(`http://localhost:${port}/ai-presets`, {
        method: "POST", headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ task, block, name, content: blockStates[block]?.content || "" }),
      });
      const np = await res.json();
      setPresets(prev => ({ ...prev, [block]: [...(prev[block] || []), np] }));
      setBlockStates(prev => ({ ...prev, [block]: { ...prev[block], presetId: np.id } }));
    } else if (presetInputMode === "rename") {
      const presetId = blockStates[block]?.presetId;
      await fetch(`http://localhost:${port}/ai-presets/${presetId}`, {
        method: "PUT", headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ name }),
      });
      setPresets(prev => ({ ...prev, [block]: (prev[block] || []).map(p => p.id === presetId ? { ...p, name } : p) }));
    }
    setPresetInputBlock(null);
    setPresetInputMode(null);
    setPresetInputValue("");
    showToast(t("tstDlgAiPresetSaved"));
  };

  const handleDeletePreset = (block) => {
    setPresetPendingDelete(block);
    setExpandedBlocks(prev => ({ ...prev, [block]: true }));
  };

  const handlePresetDeleteConfirm = async (block) => {
    const presetId = blockStates[block]?.presetId;
    if (!presetId) return;
    const res = await fetch(`http://localhost:${port}/ai-presets/${presetId}`, { method: "DELETE" });
    if (!res.ok) { const err = await res.json(); showToast(err.error || "Error"); setPresetPendingDelete(null); return; }
    const presetData = await fetch(`http://localhost:${port}/ai-presets?task=${task}`).then(r => r.json());
    const grouped = {};
    for (const p of presetData) { if (!grouped[p.block]) grouped[p.block] = []; grouped[p.block].push(p); }
    setPresets(grouped);
    const next = (grouped[block] || [])[0] || null;
    setBlockStates(prev => ({ ...prev, [block]: { ...prev[block], presetId: next?.id || null, content: next?.content || "" } }));
    setPresetPendingDelete(null);
    showToast(t("tstDlgAiPresetDeleted"));
  };

  const handleOverwrite = async (block) => {
    const state = blockStates[block];
    if (!state?.presetId) return;
    const preset = (presets[block] || []).find(p => p.id === state.presetId);
    if (preset?.isDefault) return;
    await fetch(`http://localhost:${port}/ai-presets/${state.presetId}`, {
      method: "PUT", headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ content: state.content }),
    });
    setPresets(prev => ({ ...prev, [block]: prev[block].map(p => p.id === state.presetId ? { ...p, content: state.content } : p) }));
    showToast(t("tstDlgAiPresetSaved"));
  };

  const currentTpl = templates.find(tpl => tpl.id === selectedTemplateId) || null;
  const isCurrentDefault = localDefaultId === selectedTemplateId;
  const templateModified = !currentTpl?.isDefault && blockOrder.some(b =>
    (blockStates[b]?.presetId || null) !== (savedTemplateBlocks[b]?.presetId || null) ||
    (blockStates[b]?.enabled !== false) !== (savedTemplateBlocks[b]?.enabled !== false)
  );

  const reloadTemplates = async () => {
    const data = await fetch(`http://localhost:${port}/ai-templates?task=${task}`).then(r => r.json());
    setTemplates(data);
    return data;
  };

  const handleSaveAsTemplate = () => {
    setTplInputValue("");
    setTplInputMode("create");
  };

  const handleRenameTemplate = () => {
    if (!selectedTemplateId || currentTpl?.isDefault) return;
    setTplInputValue(currentTpl?.name || "");
    setTplInputMode("rename");
  };

  const handleDeleteTemplate = () => {
    if (!selectedTemplateId || currentTpl?.isDefault) return;
    setPendingDelete(true);
  };

  const handleTplInputConfirm = async () => {
    const name = tplInputValue.trim();
    if (!name) return;
    if (tplInputMode === "create") {
      const source = templates[0];
      const sourceBlocks = source?.blocks || [];
      const blockArr = sourceBlocks.length
        ? sourceBlocks.map(b => ({ block: b.block, name: b.name, prefix: b.prefix, presetId: null, enabled: true }))
        : defaultBlocks.map(id => ({ block: id, name: id, prefix: id.toUpperCase(), presetId: null, enabled: true }));
      const res = await fetch(`http://localhost:${port}/ai-templates`, {
        method: "POST", headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ task, name, blocks: blockArr }),
      });
      const nt = await res.json();
      await reloadTemplates();
      setSelectedTemplateId(nt.id);
      const { states, order, meta } = buildFromTemplate(nt, presets);
      setBlockOrder(order);
      setBlockMeta(meta);
      setExpandedBlocks(Object.fromEntries(order.map(b => [b, true])));
      setBlockStates(states);
      setSavedTemplateBlocks(snapshotTplBlocks(states, order));
      showToast(t("tstDlgAiTemplateSaved"));
    } else if (tplInputMode === "rename") {
      await fetch(`http://localhost:${port}/ai-templates/${selectedTemplateId}`, {
        method: "PUT", headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ name }),
      });
      await reloadTemplates();
      showToast(t("tstDlgAiTemplateSaved"));
    }
    setTplInputMode(null);
    setTplInputValue("");
  };

  const handleDeleteConfirm = async () => {
    await fetch(`http://localhost:${port}/ai-templates/${selectedTemplateId}`, { method: "DELETE" });
    if (localDefaultId === selectedTemplateId) { onSetDefault(task, null); setLocalDefaultId(null); }
    const data = await reloadTemplates();
    const next = data[0] || null;
    setSelectedTemplateId(next?.id || null);
    if (next) {
      const { states, order, meta } = buildFromTemplate(next, presets);
      setBlockOrder(order);
      setBlockMeta(meta);
      setExpandedBlocks(Object.fromEntries(order.map(b => [b, true])));
      setBlockStates(states);
      setSavedTemplateBlocks(snapshotTplBlocks(states, order));
    }
    setPendingDelete(false);
    showToast(t("tstDlgAiTemplateDeleted"));
  };

  const handleOverwriteTemplate = async () => {
    if (!selectedTemplateId || currentTpl?.isDefault) return;
    await Promise.all(blockOrder.map(b =>
      fetch(`http://localhost:${port}/ai-templates/${selectedTemplateId}/blocks/${encodeURIComponent(b)}`, {
        method: "PUT", headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ presetId: blockStates[b]?.presetId || null, enabled: blockStates[b]?.enabled !== false, name: blockMeta[b]?.name, prefix: blockMeta[b]?.prefix }),
      })
    ));
    setSavedTemplateBlocks(snapshotTplBlocks(blockStates));
    await reloadTemplates();
    showToast(t("tstDlgAiTemplateSaved"));
  };

  const handleSetDefault = () => {
    if (!selectedTemplateId) return;
    onSetDefault(task, selectedTemplateId);
    setLocalDefaultId(selectedTemplateId);
  };

  const handleRemoveBlock = async (block) => {
    if (!selectedTemplateId || currentTpl?.isDefault) return;
    await fetch(`http://localhost:${port}/ai-templates/${selectedTemplateId}/blocks/${encodeURIComponent(block)}`, { method: "DELETE" });
    const newOrder = blockOrder.filter(b => b !== block);
    setBlockOrder(newOrder);
    setBlockStates(prev => { const next = { ...prev }; delete next[block]; return next; });
    setBlockMeta(prev => { const next = { ...prev }; delete next[block]; return next; });
    setExpandedBlocks(prev => { const next = { ...prev }; delete next[block]; return next; });
    setSavedTemplateBlocks(snapshotTplBlocks(blockStates, newOrder));
    await reloadTemplates();
  };

  const handleAddBlock = async (block) => {
    if (!selectedTemplateId || currentTpl?.isDefault) return;
    const firstPreset = (presets[block] || [])[0] || null;
    const blockName = BLOCK_LABELS[block] ?? block;
    const prefix = block.toUpperCase();
    await fetch(`http://localhost:${port}/ai-templates/${selectedTemplateId}/blocks/${encodeURIComponent(block)}`, {
      method: "PUT", headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ presetId: firstPreset?.id || null, enabled: true, name: blockName, prefix }),
    });
    const newOrder = [...blockOrder, block];
    setBlockOrder(newOrder);
    setBlockStates(prev => ({ ...prev, [block]: { presetId: firstPreset?.id || null, content: firstPreset?.content || "", enabled: true } }));
    setBlockMeta(prev => ({ ...prev, [block]: { name: blockName, prefix } }));
    setExpandedBlocks(prev => ({ ...prev, [block]: true }));
    setSavedTemplateBlocks(snapshotTplBlocks({ ...blockStates, [block]: { presetId: firstPreset?.id || null, enabled: true } }, newOrder));
    await reloadTemplates();
  };

  const BLOCK_LABELS = { role: t("lblDlgAiRole"), structure: t("lblDlgAiStructure"), rules: t("lblDlgAiRules"), style: t("lblDlgCardStyle") };

  return (
    <div className="dl-backdrop" style={{ zIndex: 2000 }} onMouseDown={e => { if (e.target === e.currentTarget) { onSaveTweaks(buildTweaks()); onClose(); } }}>
      <div className="dlg" style={{ width: 520, maxWidth: "95vw" }} onKeyDown={(e) => { if (e.key !== 'Tab') return; const els = [...e.currentTarget.querySelectorAll('button, input, select, textarea, [tabindex]:not([tabindex="-1"])')].filter(el => !el.disabled && el.offsetParent !== null); const first = els[0]; const last = els[els.length - 1]; if (e.shiftKey) { if (document.activeElement === first) { e.preventDefault(); last.focus(); } } else { if (document.activeElement === last) { e.preventDefault(); first.focus(); } } }}>
        <div className="dlg-body" style={{ display: "flex", flexDirection: "column", gap: 16 }}>
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
          <h3 className="dlg-title" style={{ margin: 0 }}><Sparkles color={'var(--accent)'} />{t("lblDlgSettingsAiTemplates")}</h3>
          <div style={{ display: "flex", gap: 6, alignItems: "center" }}>
            <button className="btn" onClick={() => setExpertMode(v => !v)}>{expertMode ? t("btnDlgSettingsAiTemplatesModeSimple") : t("btnDlgSettingsAiTemplatesModeExpert")}</button>
            <button className="dl-close" onClick={() => { onSaveTweaks(buildTweaks()); onClose(); }}><X /></button>
          </div>
        </div>

        <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
          <label className="dlg-field-label" style={{ flexShrink: 0 }}>{t("lblDlgSettingsAiTemplatesMode")}</label>
          <select value={selectedTemplateId || ""} onChange={e => handleTemplateChange(e.target.value)}
            className="select" style={{ flex: 1 }}>
            {templates.map(tpl => <option key={tpl.id} value={tpl.id}>{tpl.name}</option>)}
          </select>
          <button className="btn icon small subtle" onClick={handleSaveAsTemplate} title={t("tipDlgAiSaveAsTemplate")}
           >
            <Plus />
          </button>
          <button className="btn icon small subtle" onClick={handleRenameTemplate} title={t("tipDlgAiRenameTemplate")} disabled={currentTpl?.isDefault}
            style={{ color: currentTpl?.isDefault ? 'var(--text-mute)' : 'var(--text-mute)' }}>
            <TextCursor />
          </button>
          <button className="btn icon small subtle" onClick={handleOverwriteTemplate} title={t("tipDlgAiOverwrite")} disabled={!templateModified}
            style={{ color: templateModified ? 'var(--text-mute)' : 'var(--text-mute)' }}>
            <Save />
          </button>
          <button className="btn icon small subtle" onClick={handleDeleteTemplate} title={t("tipDlgAiDeleteTemplate")} disabled={currentTpl?.isDefault}
            style={{ color: currentTpl?.isDefault ? 'var(--text-mute)' : 'var(--text-mute)' }}>
            <Trash2 />
          </button>
          <button className="btn icon small subtle" onClick={handleSetDefault} title={t("tipDlgAiSetDefault")}
            style={{ color: isCurrentDefault ? "var(--star)" : 'var(--text-mute)' }}>
            <Star fill={isCurrentDefault ? "var(--star)" : "none"} />
          </button>
        </div>

        {tplInputMode && (
          <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
            <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
              <input autoFocus value={tplInputValue} onChange={e => setTplInputValue(e.target.value)}
                onKeyDown={e => { if (e.key === "Enter") handleTplInputConfirm(); if (e.key === "Escape") { setTplInputMode(null); setTplInputValue(""); } }}
                className="input" style={{ flex: 1, borderColor: 'var(--accent)' }} />
              <button className="btn icon small subtle" onClick={handleTplInputConfirm} disabled={!tplInputValue.trim()}
                style={{ color: tplInputValue.trim() ? "var(--ok)" : 'var(--text-mute)' }}>
                <Check />
              </button>
              <button className="btn icon small subtle" onClick={() => { setTplInputMode(null); setTplInputValue(""); }}
               >
                <X />
              </button>
            </div>
          </div>
        )}

        {pendingDelete && (
          <div style={{ display: "flex", alignItems: "center", gap: 8, padding: "6px 10px", background: "color-mix(in srgb, var(--danger) 13%, transparent)", borderRadius: 8, border: "1px solid color-mix(in srgb, var(--danger) 40%, transparent)" }}>
            <span style={{ fontSize: 12, color: 'var(--text)', flex: 1 }}>{t("cfmDlgAiDeleteTemplateQ")}</span>
            <button className="btn primary" style={{ "--accent": "var(--danger)", "--accent-hov": "color-mix(in srgb, var(--danger) 82%, black)" }} onClick={handleDeleteConfirm}>{t("btnCardDelete")}</button>
            <button className="btn subtle" onClick={() => setPendingDelete(false)}>{t("btnGlbCancel")}</button>
          </div>
        )}

        {expertMode && dataLoaded && (
          <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
            <div className="dlg-field-label" style={{ marginBottom: 2 }}>{t("ttlDlgAiBlocks")}</div>
            {blockOrder.map(block => {
              const state = blockStates[block] || { presetId: null, content: "", enabled: true };
              const blockPresets = presets[block] || [];
              const modified = isModified(block);
              const expanded = expandedBlocks[block] || false;
              const currentPreset = state.presetId ? blockPresets.find(p => p.id === state.presetId) : null;
              const canOverwrite = modified && currentPreset && !currentPreset.isDefault;
              return (
                <div key={block} style={{ border: `1px solid var(--border-strong)`, borderRadius: 8, overflow: "hidden", opacity: state.enabled ? 1 : 0.5 }}>
                  <div style={{ display: "flex", alignItems: "center", gap: 6, padding: "6px 10px", background: 'var(--bg-input)' }}>
                    <button className="btn icon small subtle" onClick={() => setExpandedBlocks(prev => ({ ...prev, [block]: !expanded }))} title={t("tipDlgAiTweakBlock")}
                      style={{ color: expanded ? 'var(--accent)' : 'var(--text-mute)' }}>
                      <ChevronDown style={{ transform: expanded ? "rotate(180deg)" : "none", transition: "transform 0.15s" }} />
                    </button>
                    <span className="hint" style={{ flexShrink: 0 }}>{blockMeta[block]?.name ?? BLOCK_LABELS[block] ?? block}</span>
                    <select value={state.presetId || ""} onChange={e => handlePresetChange(block, e.target.value || null)}
                      disabled={!state.enabled} className="select" style={{ flex: 1 }}>
                      {blockPresets.map(p => <option key={p.id} value={p.id}>{p.name}</option>)}
                    </select>
                    <button className="btn icon small subtle" onClick={() => { handleSaveAsNew(block); setExpandedBlocks(prev => ({ ...prev, [block]: true })); }} title={t("tipDlgAiSaveAsNew")}
                     >
                      <Plus />
                    </button>
                    <button className="btn icon small subtle" onClick={() => handleRenamePreset(block)} title={t("tipDlgAiRenameTemplate")} disabled={!state.presetId || currentPreset?.isDefault}
                      style={{ color: (!state.presetId || currentPreset?.isDefault) ? 'var(--text-mute)' : 'var(--text-mute)' }}>
                      <TextCursor />
                    </button>
                    <button className="btn icon small subtle" onClick={() => handleOverwrite(block)} title={t("tipDlgAiOverwrite")} disabled={!canOverwrite}
                      style={{ color: canOverwrite ? 'var(--text-mute)' : 'var(--text-mute)' }}>
                      <Save />
                    </button>
                    <button className="btn icon small subtle" onClick={() => handleBlockToggle(block)} title={t("tipDlgAiToggleBlock")}
                      style={{ color: state.enabled ? "var(--ok)" : 'var(--text-mute)' }}>
                      {state.enabled ? <Check /> : <X />}
                    </button>
                    <button className="btn icon small subtle" onClick={() => handleRemoveBlock(block)} title={t("tipDlgAiRemoveBlock")} disabled={currentTpl?.isDefault}
                      style={{ color: currentTpl?.isDefault ? 'var(--text-mute)' : 'var(--text-mute)' }}>
                      <Trash2 />
                    </button>
                  </div>
                  {expanded && (
                    <div style={{ padding: 10, borderTop: `1px solid var(--border)`, display: "flex", flexDirection: "column", gap: 6 }}>
                      {presetInputBlock === block && (
                        <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                          <input autoFocus value={presetInputValue} onChange={e => setPresetInputValue(e.target.value)}
                            onKeyDown={e => { if (e.key === "Enter") handlePresetInputConfirm(block); if (e.key === "Escape") { setPresetInputBlock(null); setPresetInputMode(null); setPresetInputValue(""); } }}
                            className="input" style={{ flex: 1, borderColor: 'var(--accent)' }} />
                          <button className="btn icon small subtle" onClick={() => handlePresetInputConfirm(block)} disabled={!presetInputValue.trim()}
                            style={{ color: presetInputValue.trim() ? "var(--ok)" : 'var(--text-mute)' }}>
                            <Check />
                          </button>
                          <button className="btn icon small subtle" onClick={() => { setPresetInputBlock(null); setPresetInputMode(null); setPresetInputValue(""); }}
                           >
                            <X />
                          </button>
                        </div>
                      )}
                      {presetPendingDelete === block && (
                        <div style={{ display: "flex", alignItems: "center", gap: 8, padding: "5px 8px", background: "color-mix(in srgb, var(--danger) 13%, transparent)", borderRadius: 6, border: "1px solid color-mix(in srgb, var(--danger) 40%, transparent)" }}>
                          <span style={{ fontSize: 12, color: 'var(--text)', flex: 1 }}>{t("cfmDlgAiDeletePresetQ")}</span>
                          <button className="btn primary" style={{ "--accent": "var(--danger)", "--accent-hov": "color-mix(in srgb, var(--danger) 82%, black)" }} onClick={() => handlePresetDeleteConfirm(block)}>{t("btnCardDelete")}</button>
                          <button className="btn subtle" onClick={() => setPresetPendingDelete(null)}>{t("btnGlbCancel")}</button>
                        </div>
                      )}
                      <textarea rows={4} value={state.content} onChange={e => setBlockStates(prev => ({ ...prev, [block]: { ...prev[block], content: e.target.value } }))}
                        className="textarea" />
                      {state.presetId && !currentPreset?.isDefault && (
                        <div style={{ display: "flex", justifyContent: "flex-end" }}>
                          <button className="btn small subtle" onClick={() => handleDeletePreset(block)} title={t("tipDlgAiDeletePreset")}
                           >
                            <Trash2 /><span style={{ color: 'var(--text-mute)' }}>{t("btnDlgAiDeletePreset")}</span>
                          </button>
                        </div>
                      )}
                    </div>
                  )}
                </div>
              );
            })}
            {!currentTpl?.isDefault && (() => {
              const available = Object.keys(presets).filter(b => !blockOrder.includes(b));
              return available.length > 0 && (
                <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                  <select id="addBlockSelect" className="select" style={{ flex: 1 }}>
                    {available.map(b => <option key={b} value={b}>{BLOCK_LABELS[b] ?? b}</option>)}
                  </select>
                  <button className="btn" onClick={() => { const sel = document.getElementById("addBlockSelect"); if (sel) handleAddBlock(sel.value); }}
                    title={t("tipDlgAiAddBlock")}>
                    <Plus className="icon-inline" />
                    <span>{t("btnDlgAiAddBlock")}</span>
                  </button>
                </div>
              );
            })()}
          </div>
        )}

        {allVars.length > 0 && (
          <div style={{ display: "flex", flexDirection: "column", gap: 8, background: "var(--dlg-bgd)", border: `1px solid var(--border-strong)`, borderRadius: 8, padding: "10px 12px" }}>
            <span className="dlg-field-label">{t("ttlDlgAiVariables")}</span>
            <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(180px, 1fr))", gap: 8 }}>
              {allVars.map(v => (
                <div key={v.name} style={{ display: "flex", flexDirection: "column", gap: 3 }}>
                  <label className="dlg-field-label">{v.name}</label>
                  {v.type === "ckb" ? (
                    <label className="check-label" style={{ padding: "5px 0" }}>
                      <input
                        type="checkbox" className="check-box"
                        checked={varValues[v.name] !== false}
                        onChange={e => setVarValues(prev => ({ ...prev, [v.name]: e.target.checked }))}
                      />
                      <span>{v.defaultVal}</span>
                    </label>
                  ) : (
                    <input
                      type={v.type === "num" ? "number" : "text"}
                      value={varValues[v.name] ?? v.defaultVal}
                      onChange={e => setVarValues(prev => ({ ...prev, [v.name]: e.target.value }))}
                      className="input"
                    />
                  )}
                </div>
              ))}
            </div>
          </div>
        )}

        <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
          <label className="dlg-field-label">{t("lblDlgAiYourRequest")}</label>
          <textarea rows={3} value={userRequest} onChange={e => setUserRequest(e.target.value)}
            placeholder={task === "vocal" ? t("plhDlgCardAiVocal") : task === "lyrics" ? t("plhDlgCardAiSong") : t("plhDlgCardAiMusic")}
            className="textarea"
            onKeyDown={e => { if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) handleGenerate(); }} />
        </div>

        <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
          {promptLengthMode && (() => {
            const step = promptLengthMode === "words" ? 10 : 50;
            return (
              <div style={{ display: "flex", alignItems: "center", gap: 6, flexShrink: 0 }}>
                <NumberField value={limit} min={step} step={step} onChange={setLimit} width={72} />
                <span className="hint">{promptLengthMode === "words" ? t("lblDlgCardWords") : t("lblDlgCardChars")}</span>
              </div>
            );
          })()}
          <button className="btn" onClick={handleCopy} disabled={!userRequest.trim()}
            title={t("tipDlgAiCopyRequest")} style={{ flexShrink: 0 }}>
            <Sparkles />
          </button>
          <button className="btn primary" onClick={handleGenerate} disabled={loading || !userRequest.trim() || !txtSettingsAiApiKey}
            style={{ flex: 1, justifyContent: "center" }}>
            {loading ? <RotateCcw className="spinner" /> : <Sparkles />}
            {loading ? "…" : t("tipDlgAiGenerate")}
          </button>
        </div>
        </div>
      </div>
    </div>
  );
}

function CardModal({ card, draft, allExistingTags, allCards, types, langKey, apiPort, txtSettingsAiApiKey, txtSettingsAiLimitsVocal, txtSettingsAiLimitsMusic, txtSettingsAiStyleThreshold, txtSettingsAiLyricsThreshold, txtSettingsAiStyleWordThreshold, activeProject, projects, txtSettingsCardsAiSites, txtSettingsCardsMusicSites, aiDefaultTemplates, tglSettingsAiTemplatesMode, onSetDefaultTemplate, globalEnvId, onMakeGlobal, onSave, onClose, showToast, onUpdateTranslationsIndex }) {
  const t = useT(langKey);
  // Fall back to static TYPES if not passed (shouldn't happen but safe)
  if (!types) types = buildTypes(DEFAULT_TYPE_COLORS);
  const rowsBonus = card ? UIDLG.textareaRowsEditBonus : 0;
  // ⚠ CLAUDE: `init` seeds the FIELDS only (an existing card, or the Suno draft the New button
  // built from the clipboard). Everything that asks "is this an existing card?" — the title,
  // the Create/Save label, the duplicate/global actions — must keep testing `card`, not `init`,
  // or a draft would present itself as an edit of a card that was never saved.
  const init = card || draft || null;
  const [type, setType]                   = useState(init?.type || "song");
  const [name, setName]                   = useState(init?.name || "");
  const [sortNumber, setSortNumber]       = useState(init?.sortNumber != null ? String(init.sortNumber) : "");
  const [desc, setDesc]                   = useState(init?.desc || "");
  const [note, setNote]                   = useState(init?.note || "");
  const [style, setStyle]                 = useState(init?.style || "");
  const [lyrics, setLyrics]               = useState(init?.lyrics || "");
  const [tagInput, setTagInput]           = useState(init?.tags?.join(", ") || "");
  const [linkedCards, setLinkedCards] = useState(init?.linkedCards || []);
  const [errors, setErrors]               = useState({});
  const [urls, setUrls]                   = useState(init?.urls || []);
  const [newUrlHref, setNewUrlHref]       = useState("");
  const [newUrlLabel, setNewUrlLabel]     = useState("");
  const [mediaPath, setMediaPath]         = useState(init?.mediaPath || "");
  const [sunoFetching, setSunoFetching]   = useState(false);
  const [nameSuggestions, setNameSuggestions] = useState([]);
  const [showAiModal, setShowAiModal]     = useState(false);
  const [aiModalTask, setAiModalTask]     = useState("music");
  const [aiModalInitLimit, setAiModalInitLimit] = useState(txtSettingsAiLimitsMusic || 700);
  const [aiTweaks, setAiTweaks]           = useState(init?.aiTweaks || {});
  const [dateVal, setDateVal]             = useState(() => init?.date ? new Date(init.date).toISOString().slice(0, 10) : new Date().toISOString().slice(0, 10));
  // Clear suggestions and reset limit when type changes
  const handleTypeChange = (newType) => {
    setType(newType); setNameSuggestions([]); setMvTab("desc");
    if (newType === "vocal") setAiModalInitLimit(txtSettingsAiLimitsVocal || 300);
    else if (newType === "music") setAiModalInitLimit(txtSettingsAiLimitsMusic || 700);
    else setAiModalInitLimit(txtSettingsAiStyleThreshold || 1000);
  };
  const [linkSearch, setLinkSearch] = useState("");
  const [songTab, setSongTab] = useState("desc");
  const [mvTab, setMvTab] = useState("desc");
  const [srcLang, setSrcLang] = useState(init?.lang || "");
  const [srcLangSearch, setSrcLangSearch] = useState("");
  // ── Wizard state ──
  const [wizardStep, setWizardStep] = useState(null);
  const [wizardArtist, setWizardArtist] = useState("");
  const [wizardArtistHistory, setWizardArtistHistory] = useState(() => { try { return JSON.parse(localStorage.getItem(`${STORAGE_PREFIX}-wizard-artists`) || "[]"); } catch { return []; } });
  const [wizardDna, setWizardDna] = useState(null);
  const [wizardSubject, setWizardSubject] = useState("");
  const [wizardLoading, setWizardLoading] = useState(false);
  const [wizardError, setWizardError] = useState("");
  const [wizardData, setWizardData] = useState(null);
  const translateDataRef = useRef(null);
  const [wizardList, setWizardList] = useState([]);
  const [wizardSelected, setWizardSelected] = useState(null);
  const [srcLangOpen, setSrcLangOpen] = useState(false);
  const [translations, setTranslations] = useState([]);
  const [expandedTranslations, setExpandedTranslations] = useState(new Set());
  const [addLangSearch, setAddLangSearch] = useState("");
  const [translatingId, setTranslatingId] = useState(null);
  const s = makeStyles();
  const cfg = type === "song" ? getSongState({ urls, linkedCards }, types, txtSettingsCardsAiSites, txtSettingsCardsMusicSites) : types[type];

  useEffect(() => {
    if (card?.id && type === "song") {
      fetch(`http://localhost:${apiPort || "3001"}/translations/${card.id}`)
        .then(r => r.json()).then(data => {
          if (Array.isArray(data)) {
            setTranslations(data);
            setExpandedTranslations(new Set(data.map(tr => tr.id)));
          }
        }).catch(() => {});
    }
  }, [card?.id]); // eslint-disable-line
  const { Icon: TypeIcon } = cfg;

  const currentWord   = tagInput.split(",").pop().trim().toLowerCase();
  const confirmedPart = tagInput.includes(",") ? tagInput.slice(0, tagInput.lastIndexOf(",") + 1) + " " : "";
  const alreadyAdded  = tagInput.split(",").map((tk) => tk.trim().toLowerCase()).filter(Boolean);
  const suggestions   = currentWord.length > 0 ? allExistingTags.filter((tg) => tg.toLowerCase().includes(currentWord) && !alreadyAdded.includes(tg.toLowerCase())) : [];

  const pickSuggestion = (tag) => setTagInput(confirmedPart + tag + ", ");

  const toggleLink = (id) => setLinkedCards((prev) => prev.includes(id) ? prev.filter((x) => x !== id) : [...prev, id]);

  // ── Wizard handlers ──────────────────────────────────────────────────────────
  const port = apiPort || "3001";

  const openWizard = async () => {
    setWizardArtist("");
    setWizardDna(null);
    setWizardSubject("");
    setWizardError("");
    setWizardSelected(null);
    if (!wizardData) {
      try {
        const [templates, presets, systemBlocks, wizards] = await Promise.all([
          fetch(`http://localhost:${port}/ai-templates`).then(r => r.json()),
          fetch(`http://localhost:${port}/ai-presets`).then(r => r.json()),
          fetch(`http://localhost:${port}/ai-system`).then(r => r.json()),
          fetch(`http://localhost:${port}/ai-wizards`).then(r => r.json()),
        ]);
        setWizardData({ templates, presets, systemBlocks });
        setWizardList(wizards);
        if (wizards.length === 1) { setWizardSelected(wizards[0]); setWizardStep(2); }
        else setWizardStep(1);
      } catch (e) { setWizardError(e.message); }
    } else if (wizardList.length === 1) {
      setWizardSelected(wizardList[0]);
      setWizardStep(2);
    } else {
      setWizardStep(1);
    }
  };

  const wizardTaskId = (category) => wizardSelected?.steps?.find(s => s.category === category)?.task;

  const handleWizardStep1 = async () => {
    if (!wizardArtist.trim() || !txtSettingsAiApiKey || !wizardData || !wizardSelected) return;
    const noteTaskId = wizardTaskId("wzd-note");
    if (!noteTaskId) return setWizardError("Missing wzd-note task in wizard");
    setWizardLoading(true);
    setWizardError("");
    try {
      const wizPrompt = buildTaskRequest(noteTaskId, { ARTIST: wizardArtist.trim() }, wizardData.templates, wizardData.presets, wizardData.systemBlocks);
      const bag = await callAi(wizPrompt, txtSettingsAiApiKey, port, noteTaskId);
      setWizardDna(bag);
      const dnaParts = [];
      if (bag.ARTIST) dnaParts.push(`ARTIST: ${bag.ARTIST}`);
      if (bag.LANGUAGE_PRIMARY) dnaParts.push(`LANGUAGE_PRIMARY: ${bag.LANGUAGE_PRIMARY}`);
      if (bag.LANGUAGE_SECONDARY) dnaParts.push(`LANGUAGE_SECONDARY: ${bag.LANGUAGE_SECONDARY}`);
      if (bag.STANCE) dnaParts.push(`STANCE: ${bag.STANCE}`);
      if (bag.STRUCTURE) dnaParts.push(`STRUCTURE: ${bag.STRUCTURE}`);
      if (bag.STYLE) dnaParts.push(`STYLE: ${bag.STYLE}`);
      if (bag.SUBJECTS) dnaParts.push(`SUBJECTS:\n${bag.SUBJECTS}`);
      setNote(dnaParts.join("\n\n"));
      setName(wizardArtist.trim());
      const artist = wizardArtist.trim();
      setWizardArtistHistory(prev => {
        const next = [artist, ...prev.filter(a => a.toLowerCase() !== artist.toLowerCase())].slice(0, 50);
        localStorage.setItem(`${STORAGE_PREFIX}-wizard-artists`, JSON.stringify(next));
        return next;
      });
      setWizardStep(3);
    } catch (e) { setWizardError(e.message); }
    setWizardLoading(false);
  };

  const handleWizardStep2 = async () => {
    if (!wizardSubject || !txtSettingsAiApiKey || !wizardData || !wizardDna || !wizardSelected) return;
    const styleTaskId = wizardTaskId("wzd-style");
    const lyricsTaskId = wizardTaskId("wzd-lyrics");
    if (!styleTaskId || !lyricsTaskId) return setWizardError("Missing wzd-style or wzd-lyrics task in wizard");
    setWizardLoading(true);
    setWizardError("");
    try {
      const lang = wizardDna.LANGUAGE_PRIMARY || "English";
      const wizPrompt2 = buildTaskRequest(styleTaskId, { ARTIST: wizardArtist.trim() }, wizardData.templates, wizardData.presets, wizardData.systemBlocks);
      const wizPrompt3 = buildTaskRequest(lyricsTaskId, { ARTIST: wizardArtist.trim(), SUBJECT: wizardSubject, LANGUAGE: lang }, wizardData.templates, wizardData.presets, wizardData.systemBlocks);
      const [styleBag, lyricsBag] = await Promise.all([
        callAi(wizPrompt2, txtSettingsAiApiKey, port, styleTaskId),
        callAi(wizPrompt3, txtSettingsAiApiKey, port, lyricsTaskId),
      ]);
      setStyle(styleBag.CONTENT || styleBag._raw || "");
      setLyrics(lyricsBag.LYRICS_1 || "");
      if (lyricsBag.TITLE_1) setName(lyricsBag.TITLE_1);
      if (lyricsBag.LANGUAGE_1) setSrcLang(lyricsBag.LANGUAGE_1);
      // Store translations if present
      if (lyricsBag.LYRICS_2) {
        const enTr = { id: "pending-en", lang: lyricsBag.LANGUAGE_2 || "eng", content: lyricsBag.LYRICS_2, name: lyricsBag.TITLE_2 || "", pending: true };
        const newTranslations = [...translations.filter(tr => tr.lang !== enTr.lang), enTr];
        if (lyricsBag.LYRICS_3) {
          const frTr = { id: "pending-fr", lang: lyricsBag.LANGUAGE_3 || "fra", content: lyricsBag.LYRICS_3, name: lyricsBag.TITLE_3 || "", pending: true };
          newTranslations.push(...newTranslations.filter(tr => tr.lang !== frTr.lang).length === newTranslations.length ? [frTr] : []);
          setTranslations([...newTranslations.filter(tr => tr.lang !== frTr.lang), frTr]);
        } else {
          setTranslations(newTranslations);
        }
        setExpandedTranslations(prev => new Set([...prev, enTr.id, "pending-fr"]));
      }
      if (lyricsBag.DESCRIPTION) setDesc(lyricsBag.DESCRIPTION);
      setType("song");
      setSongTab("style");
      setWizardStep(null);
      showToast(t("ttlDlgWizard") + " ✓");
    } catch (e) { setWizardError(e.message); }
    setWizardLoading(false);
  };

  // ── Translation handlers ─────────────────────────────────────────────────────

  const saveSrcLang = async (code) => {
    setSrcLang(code);
    setSrcLangSearch("");
    setSrcLangOpen(false);
    if (card?.id) {
      await fetch(`http://localhost:${port}/cards/${card.id}`, {
        method: "PUT", headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ lang: code })
      });
    }
  };

  const handleAddTranslation = (langCode) => {
    if (!langCode) return;
    if (translations.some(tr => tr.lang === langCode && !tr.deleted)) { showToast(t("tstDlgCardTranslationExists")); return; }
    const localId = "pending-" + langCode;
    const newList = [...translations, { id: localId, lang: langCode, content: "", pending: true }];
    setTranslations(newList);
    setExpandedTranslations(prev => new Set([...prev, localId]));
    setAddLangSearch("");
  };

  const updateTranslationLocal = (id, fields) => {
    setTranslations(prev => prev.map(tr => tr.id === id ? { ...tr, ...fields } : tr));
  };


  const handleDeleteTranslation = (tr) => {
    if (tr.pending) {
      const remaining = translations.filter(x => x.id !== tr.id);
      setTranslations(remaining);
      onUpdateTranslationsIndex?.(card.id, remaining);
    } else {
      setTranslations(prev => prev.map(x => x.id === tr.id ? { ...x, deleted: true } : x));
    }
    setExpandedTranslations(prev => { const s = new Set(prev); s.delete(tr.id); return s; });
    showToast(t("tstDlgCardTranslationDeleted"));
  };

  const handleCopyTranslation = (tr) => {
    const clean = cleanLyrics(tr.content);
    navigator.clipboard.writeText(clean);
    showToast("📋 " + t("tipDlgCardTranslationCopy"));
  };

  const getTranslateData = async () => {
    if (translateDataRef.current) return translateDataRef.current;
    const [templates, presets, systemBlocks] = await Promise.all([
      fetch(`http://localhost:${port}/ai-templates?task=translate`).then(r => r.json()),
      fetch(`http://localhost:${port}/ai-presets?task=translate`).then(r => r.json()),
      fetch(`http://localhost:${port}/ai-system`).then(r => r.json()),
    ]);
    translateDataRef.current = { templates, presets, systemBlocks };
    return translateDataRef.current;
  };

  const handleCopyAiTranslationRequest = async (tr) => {
    try {
      const targetLangName = getLangName(tr.lang, 'en');
      const { templates, presets, systemBlocks } = await getTranslateData();
      const srcLangName = srcLang ? getLangName(srcLang, 'en') : "";
      const request = buildTaskRequest("translate", { SOURCE_LANGUAGE: srcLangName, TARGET_LANGUAGE: targetLangName, SONG_NAME: name.trim(), LYRICS: lyrics }, templates, presets, systemBlocks);
      navigator.clipboard.writeText(request);
      showToast("📋 " + t("tipDlgAiCopyRequest"));
    } catch (e) { showToast("Error: " + e.message); }
  };

  const handleTranslateAi = async (tr) => {
    if (!txtSettingsAiApiKey) { showToast("Gemini API key required"); return; }
    if (!lyrics.trim()) { showToast("No lyrics to translate"); return; }
    setTranslatingId(tr.id);
    try {
      const targetLangName = getLangName(tr.lang, 'en');
      const { templates, presets, systemBlocks } = await getTranslateData();
      const srcLangName = srcLang ? getLangName(srcLang, 'en') : "";
      const vars = { SOURCE_LANGUAGE: srcLangName, TARGET_LANGUAGE: targetLangName, SONG_NAME: name.trim(), LYRICS: lyrics };
      const request = buildTaskRequest("translate", vars, templates, presets, systemBlocks);
      const bag = await callAi(request, txtSettingsAiApiKey, port, "translate");
      const translatedContent = bag.CONTENT?.trim() || (bag._raw || "").trim();
      const translatedName = bag.NAME?.trim() || "";
      if (bag.SOURCE && !srcLang) {
        const detectedCode = bag.SOURCE.trim();
        if (SONG_LANGUAGES.some(l => l.code === detectedCode)) {
          await saveSrcLang(detectedCode);
        }
      }
      const updates = { content: translatedContent };
      if (translatedName) updates.name = translatedName;
      updateTranslationLocal(tr.id, updates);
    } catch (e) { showToast("Translation error: " + e.message); }
    setTranslatingId(null);
  };

  const submit = () => {
    const e = {};
    if (!name.trim()) e.name = t("msgDlgCardRequired");
    if (Object.keys(e).length) { setErrors(e); return; }
    const tags = tagInput.split(",").map((tk) => tk.trim().toLowerCase()).filter(Boolean);
    const date = dateVal ? new Date(dateVal).getTime() : (card?.date || Date.now());
    onSave({ type, name: name.trim(), sortNumber: sortNumber.trim(), desc: desc.trim(), note: note.trim(), style: style.trim(), lyrics: lyrics.trim(), tags, linkedCards, urls, mediaPath, favorite: card?.favorite || false, aiTweaks, date, translations });
  };

  const projectCardIds = activeProject && activeProject !== "__unlinked__"
    ? (projects.find(p => p.id === activeProject)?.cardIds || [])
    : null;
  const linkableCards = allCards.filter((p) =>
    (p.type === "vocal" || p.type === "music") &&
    p.id !== card?.id &&
    (projectCardIds === null || projectCardIds.includes(p.id) || linkedCards.includes(p.id))
  );

  const fetchLyrics = async (targetUrl) => {
    setSunoFetching(true);
    try {
      const endpoint = targetUrl.includes("producer.ai/song/") ? "/fetch-producer-lyrics"
                     : targetUrl.includes("tunee.ai/music/") ? "/fetch-tunee-lyrics"
                     : "/fetch-suno-lyrics";
      const res = await fetch(`http://localhost:${port}${endpoint}`, {
        method: "POST", headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ url: targetUrl })
      });
      const data = await res.json();
      if (data.error) { showToast("Lyrics fetch failed: " + data.error); }
      else {
        let fetched = false;
        if (data.lyrics) { setLyrics(data.lyrics); fetched = true; }
        if (type === "song" && linkedCards.length === 0 && data.style) { setStyle(data.style); fetched = true; }
        showToast(fetched ? "Lyrics fetched!" : "No lyrics found on page");
      }
    } catch (e) { showToast("Lyrics fetch error: " + e.message); }
    setSunoFetching(false);
  };

  const handleAddUrl = async () => {
    if (!newUrlHref.trim()) return;
    let href = newUrlHref.trim().startsWith("http") ? newUrlHref.trim() : "https://" + newUrlHref.trim();
    // A Suno share link (suno.com/s/<code>) is only a redirect stub — expand it to the canonical
    // suno.com/song/<uuid> so the song state, the lyrics fetch and the media-provenance link all
    // see the same URL. Suno unreachable → keep the share link, which still opens fine.
    href = await resolveSunoLink(href);
    // Auto-detect label from URL
    let autoLabel = newUrlLabel.trim();
    if (!autoLabel) {
      if (href.includes("mozartai.com")) autoLabel = "mozartai";
      else if (href.includes("producer.ai")) autoLabel = "producer";
      else if (href.includes("suno.com")) autoLabel = "suno";
      else if (href.includes("tunee.ai")) autoLabel = "tunee";
      else if (href.includes("musicfy.club")) autoLabel = "musicfy";
      else if (href.includes("wikipedia")) autoLabel = "wikipedia";
      else { try { autoLabel = new URL(href).hostname.replace("www.", "").replace(/\.(com|ai)$/, ""); } catch { autoLabel = "Link"; } }
    }
    if (card?.id) {
      const res = await fetch(`http://localhost:${port}/cards/${card.id}/urls`, {
        method: "POST", headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ href, label: autoLabel })
      });
      const data = await res.json();
      setUrls(data.urls || []);
    } else {
      setUrls(prev => [...prev, { id: Date.now().toString(), label: autoLabel, href }]);
    }
    setNewUrlHref(""); setNewUrlLabel("");
    // Auto-fetch lyrics if content is empty and URL is a supported platform
    const isSupportedUrl = href.includes("suno.com/song/") || href.includes("producer.ai/song/") || href.includes("tunee.ai/music/");
    const shouldFetch = isSupportedUrl && (
      !lyrics.trim() || (type === "song" && linkedCards.length === 0 && !style.trim())
    );
    if (shouldFetch) fetchLyrics(href);
  };

  // Media browse — open the OS file dialog at the card's project folder (falling back to the
  // active project), so a song's media file is found where the project actually lives.
  const browseMedia = async () => {
    const dir = projects?.find(p => p.id === (card?.project || activeProject))?.path || "";
    const data = await apiFetch(`/open-media-dialog${dir ? `?defaultPath=${encodeURIComponent(dir)}` : ""}`);
    if (data.path) setMediaPath(data.path);
  };

  const handleRemoveUrl = async (urlId) => {
    if (card?.id) {
      const res = await fetch(`http://localhost:${port}/cards/${card.id}/urls/${urlId}`, { method: "DELETE" });
      const data = await res.json();
      setUrls(data.urls || []);
    } else {
      setUrls(prev => prev.filter(u => u.id !== urlId));
    }
  };

  const handleSaveTweaks = (newTweaks) => {
    setAiTweaks(newTweaks);
    if (card?.id) {
      fetch(`http://localhost:${port}/cards/${card.id}`, {
        method: "PUT", headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ aiTweaks: newTweaks }),
      }).catch(console.error);
    }
  };

  const _aiList    = (txtSettingsCardsAiSites    || '').split(',').map(s => s.trim().toLowerCase()).filter(Boolean);
  const _musicList = (txtSettingsCardsMusicSites || '').split(',').map(s => s.trim().toLowerCase()).filter(Boolean);
  const sortedUrls = [...urls].sort((a, b) => {
    const lbl = u => (u.label || '').toLowerCase().trim();
    const rank = u => _aiList.includes(lbl(u)) ? 0 : _musicList.includes(lbl(u)) ? 1 : 2;
    const ra = rank(a), rb = rank(b);
    if (ra !== rb) return ra - rb;
    return (a.label || '').localeCompare(b.label || '');
  });

  return (
    <div className="dl-backdrop" onMouseDown={(e) => { if (e.target === e.currentTarget) onClose(); }}>
      <div className="dlg" style={{ maxWidth: 900 }} onKeyDown={(e) => { if (e.key !== 'Tab') return; const els = [...e.currentTarget.querySelectorAll('button, input, select, textarea, [tabindex]:not([tabindex="-1"])')].filter(el => !el.disabled && el.offsetParent !== null); const first = els[0]; const last = els[els.length - 1]; if (e.shiftKey) { if (document.activeElement === first) { e.preventDefault(); last.focus(); } } else { if (document.activeElement === last) { e.preventDefault(); first.focus(); } } }}>
        <div style={{ padding: "28px 28px 0", flexShrink: 0 }}>
          <div style={s.modalHead}>
            <h2 className="dlg-title" style={{ margin: 0 }}><TypeIcon color={cfg.accent} />{card ? t("ttlDlgCardEdit") : t("ttlDlgCardNew")}<span style={{ fontSize: 12, fontWeight: 700, color: cfg.accent, border: `1px solid ${cfg.accent}55`, borderRadius: 6, padding: "2px 8px" }}>v{card?.version || 1}</span></h2>
            <button className="dl-close" ref={el => { if (el && !el.dataset.didFocus) { el.dataset.didFocus = "1"; el.focus(); } }} onClick={onClose}><X /></button>
          </div>
        </div>
        <div style={{ flex: 1, minHeight: 0, overflowY: "auto", padding: "20px 28px", display: "flex", flexDirection: "column", gap: 20 }}>
        {!card && (
          <div className="dlg-field">
            <label className="dlg-field-label" style={{ color: cfg.accent }}>{t("lblDlgCardType")}</label>
            <div className="opt-btns">
              {[["note", types.note], ["music", types.music], ["vocal", types.vocal], ["song", types.SongStateUndefined]].map(([key, typ]) => {
                const active = type === key;
                const { Icon: TIcon } = typ;
                return <button key={key} onClick={() => handleTypeChange(key)} className={`opt-btn${active ? " active" : ""}`} style={{ "--accent": typ.accent }}><TIcon /><span>{typ.label}</span></button>;
              })}
            </div>
          </div>
        )}
        <div className="dlg-field">
          <label className="dlg-field-label" style={{ color: cfg.accent }}>{t("lblDlgCardName")}</label>
          {nameSuggestions.length > 0 && (
            <div style={{ display: "flex", flexWrap: "wrap", gap: 6, marginBottom: 8 }}>
              {nameSuggestions.map((sug, i) => (
                <button className="btn primary" key={i} onClick={() => { setName(sug); setNameSuggestions([]); }}
                 >
                  <Sparkles />{sug}
                </button>
              ))}
              <button className="btn icon small subtle" onClick={() => setNameSuggestions([])}><X /></button>
            </div>
          )}
          <div style={{ display: "flex", gap: 8, alignItems: "flex-start" }}>
            <NumberField
              value={sortNumber === "" ? "" : Number(sortNumber)}
              onChange={(v) => setSortNumber(v === "" ? "" : String(v))}
              min={1}
              max={999}
              allowEmpty
              width={64}
              placeholder="#"
              style={{ textAlign: "center", fontWeight: 700 }}
            />
            <input className="input" style={{ ...(errors.name ? s.errBorder : {}), flex: 1, margin: 0 }}
              placeholder={type === "song" ? t("plhDlgCardNameSong") : type === "vocal" ? t("plhDlgCardNameVocal") : t("plhDlgCardNameMusic")}
              value={name} onChange={(e) => { setName(e.target.value); setErrors((v) => ({ ...v, name: null })); }} />
          </div>
          {errors.name && <span style={s.errMsg}>{errors.name}</span>}
        </div>
        {type === "note" ? (<>
          <div className="tabs" style={{ margin: "0 -28px", padding: "0 28px" }}>
            {[["desc", AlignLeft, t("tabDlgCardDesc")], ["note", FileText, t("tabDlgCardNote")]].map(([key, TabIcon, label]) => {
              const active = songTab === key;
              return (
                <button key={key} onClick={() => setSongTab(key)} className={`tab${active ? " active" : ""}`} style={active ? { "--accent": cfg.accent } : undefined}>
                  <TabIcon />{label}
                </button>
              );
            })}
          </div>
          <div style={{ minHeight: card ? UIDLG.tabMinHeightEdit : UIDLG.tabMinHeightNew, display: "flex", flexDirection: "column", gap: 20 }}>
            {songTab === "desc" && (
              <div className="dlg-field">
                <label className="dlg-field-label" style={{ color: cfg.accent }}>{t("tabDlgCardDesc")}</label>
                <textarea className="textarea" style={{ minHeight: 200 }}
                  placeholder={t("tabDlgCardDesc") + "…"} rows={UIDLG.textareaRowsDescNote + rowsBonus}
                  value={desc} onChange={(e) => setDesc(e.target.value)} />
              </div>
            )}
            {songTab === "note" && (
              <div className="dlg-field">
                <label className="dlg-field-label" style={{ color: cfg.accent }}>{t("tabDlgCardNote")}</label>
                <textarea className="textarea" style={{ minHeight: 200 }}
                  placeholder={t("tabDlgCardNote") + "…"} rows={UIDLG.textareaRowsDescNote + rowsBonus}
                  value={note} onChange={(e) => setNote(e.target.value)} />
              </div>
            )}
          </div>
        </>) : type === "song" ? (<>
          <div className="tabs" style={{ margin: "0 -28px", padding: "0 28px" }}>
            {[["desc", AlignLeft, t("tabDlgCardDesc")], ["style", Music2, t("lblDlgCardStyle")], ["linked", Link2, t("lblDlgCardLinkedStyles")], ["lyrics", ScrollText, t("tabDlgCardLyrics")], ["translation", Languages, t("tabDlgCardTranslation")], ["links", Globe, t("tabDlgCardLinks")], ["note", FileText, t("tabDlgCardNote")]].map(([key, TabIcon, label]) => {
              const active = songTab === key;
              return (
                <button key={key} onClick={() => setSongTab(key)} className={`tab${active ? " active" : ""}`} style={active ? { "--accent": cfg.accent } : undefined}>
                  <TabIcon />{label}
                </button>
              );
            })}
          </div>
          <div style={{ minHeight: card ? UIDLG.tabMinHeightEdit : UIDLG.tabMinHeightNew, display: "flex", flexDirection: "column", gap: 20 }}>
          {songTab === "desc" && (
            <div className="dlg-field">
              <label className="dlg-field-label" style={{ color: cfg.accent }}>{t("tabDlgCardDesc")}</label>
              <textarea className="textarea"
                placeholder={t("tabDlgCardDesc") + "…"} rows={UIDLG.textareaRowsDescNote + rowsBonus}
                value={desc} onChange={(e) => setDesc(e.target.value)} />
            </div>
          )}
          {songTab === "note" && (
            <div className="dlg-field">
              <label className="dlg-field-label" style={{ color: cfg.accent }}>{t("tabDlgCardNote")}</label>
              <textarea className="textarea"
                placeholder={t("tabDlgCardNote") + "…"} rows={UIDLG.textareaRowsDescNote + rowsBonus}
                value={note} onChange={(e) => setNote(e.target.value)} />
            </div>
          )}
          {songTab === "style" && <>
            <div className="dlg-field">
              <label className="dlg-field-label" style={{ color: cfg.accent }}>{t("lblDlgCardStyle")}</label>
              <textarea className="textarea"
                placeholder={t("plhDlgCardStyle")} rows={UIDLG.textareaRowsStyle + rowsBonus}
                value={style} onChange={(e) => { setStyle(e.target.value); setErrors((v) => ({ ...v, style: null })); }} />
              <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginTop: 4 }}>
                <button className="btn" onClick={() => { setAiModalTask("music"); setAiModalInitLimit(txtSettingsAiStyleThreshold || 1000); setShowAiModal(true); }}>
                  <Sparkles />{t("tipDlgAiGenerate")}
                </button>
                <span className="hint" style={{ fontVariantNumeric: "tabular-nums" }}>{style.length} {t("lblDlgCardChars")}</span>
              </div>
            </div>
          </>}
          {songTab === "linked" && <div className="dlg-field">
              <div style={{ display: "flex", alignItems: "center", marginBottom: 0 }}>
                <label className="dlg-field-label" style={{ color: cfg.accent, marginBottom: 0, display: "flex", alignItems: "center", gap: 5 }}><Link2 className="icon-inline" />{t("lblDlgCardLinkedStyles")}<span style={{ color: 'var(--text-mute)', textTransform: "none", letterSpacing: 0, fontWeight: 400 }}>({t("msgDlgCardOptional")})</span></label>
              </div>
              {linkableCards.length === 0 ? <p className="hint">{t("empDlgCardNoMusicVocal")}</p> : (
                <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                  <input value={linkSearch} onChange={(e) => setLinkSearch(e.target.value)}
                    placeholder={linkedCards.length > 0 ? t("plhHdrSearchSeeAll") : t("plhHdrSearchCards")} className="input" />
                  <div style={{ display: "flex", flexDirection: "column", gap: 8, maxHeight: 260, overflowY: "auto" }}>
                    {["music", "vocal"].map((typeKey) => {
                      const q = linkSearch.trim().toLowerCase();
                      const localGroup = linkableCards
                        .filter((p) => p.type === typeKey && (linkedCards.includes(p.id) || (q && p.name.toLowerCase().includes(q))));
                      const globalGroup = globalEnvId && q
                        ? allCards.filter(p =>
                            p.type === typeKey &&
                            p.env === globalEnvId &&
                            !linkableCards.some(lp => lp.id === p.id) &&
                            p.name.toLowerCase().includes(q)
                          )
                        : [];
                      const group = [...localGroup, ...globalGroup]
                        .sort((a, b) => {
                          if (a.favorite !== b.favorite) return a.favorite ? -1 : 1;
                          const nc = a.name.localeCompare(b.name);
                          return nc !== 0 ? nc : (a.version || 1) - (b.version || 1);
                        });
                      if (!group.length) return null;
                      const tc = types[typeKey]; const { Icon: GIcon } = tc;
                      return <div key={typeKey}><div style={{ fontSize: 10, color: tc.accent, letterSpacing: 1, textTransform: "uppercase", marginBottom: 4, display: "flex", alignItems: "center", gap: 4 }}><GIcon className="icon-inline" />{tc.label}</div><div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>{group.map((p) => { const linked = linkedCards.includes(p.id); const isGlobal = globalEnvId && p.env === globalEnvId && !linkableCards.some(lp => lp.id === p.id); return <button key={p.id} onClick={() => toggleLink(p.id)} title={isGlobal ? "🌍 Global" : undefined} className={"btn" + (linked ? " active" : "")} style={{ "--accent": tc.accent }}>{linked && <Check className="icon-inline" />}{p.favorite && !linked && <Star className="icon-inline" style={{ opacity: 0.5 }} />}{isGlobal && <span style={{ fontSize: 9, opacity: 0.6 }}>🌍</span>}{p.name}<span style={{ fontSize: 10, opacity: 0.7, fontWeight: 700 }}>v{p.version || 1}</span></button>; })}</div></div>;
                    })}
                  </div>
                </div>
              )}
            </div>}
          {songTab === "lyrics" && <div className="dlg-field">
            <label className="dlg-field-label" style={{ color: cfg.accent }}>Lyrics</label>
            <textarea className="textarea"
              placeholder={t("plhDlgCardLyrics")} rows={UIDLG.textareaRowsLyrics + rowsBonus}
              value={lyrics} onChange={(e) => setLyrics(e.target.value)} />
            <div style={{ display: "flex", alignItems: "center", marginTop: 4, gap: 8 }}>
              <button className="btn" onClick={() => { setAiModalTask("lyrics"); setAiModalInitLimit(txtSettingsAiLyricsThreshold || 5000); setShowAiModal(true); }}>
                <Sparkles />{t("tipDlgAiGenerate")}
              </button>
              {urls.some(u => u.href.includes("suno.com/song/") || u.href.includes("producer.ai/song/") || u.href.includes("tunee.ai/music/")) && (
                <button title={t("tipDlgCardFetchLyrics")} disabled={sunoFetching} className="btn"
                  onClick={() => { const u = urls.find(x => x.href.includes("suno.com/song/") || x.href.includes("producer.ai/song/") || x.href.includes("tunee.ai/music/")); if (u) fetchLyrics(u.href); }}>
                  <Globe />{sunoFetching ? t("lblDlgCardFetching") : t("lblDlgCardFetchLyrics")}
                </button>
              )}
              {lyrics.trim() && (
                <button className="btn" title={t("tipDlgCardCopyLyricsClean")}
                  onClick={() => { navigator.clipboard.writeText(cleanLyrics(lyrics)); showToast("📋 " + t("btnDlgCardCopyLyricsClean")); }}>
                  <Music />{t("btnDlgCardCopyLyricsClean")}
                </button>
              )}
              <div style={{ flex: 1 }} />
              {/* Source language picker */}
              <span className="hint" style={{ flexShrink: 0 }}>{t("lblDlgCardTranslationSourceLanguage")}</span>
              <div style={{ flexShrink: 0 }}>
                <Combobox
                  inputStyle={{ width: srcLang ? "auto" : 110, maxWidth: 150 }}
                  placeholder="…"
                  value={srcLangOpen ? srcLangSearch : (srcLang ? `${getLangName(srcLang, langKey)} (${srcLang})` : "")}
                  onChange={setSrcLangSearch}
                  onFocus={() => setSrcLangSearch(srcLang ? getLangName(srcLang, langKey) : "")}
                  onOpenChange={setSrcLangOpen}
                  items={SONG_LANGUAGES.filter(l => {
                    const q = srcLangSearch.toLowerCase();
                    return !q || l.name.toLowerCase().includes(q) || getLangName(l.code, langKey).toLowerCase().includes(q) || l.code.toLowerCase().includes(q);
                  })}
                  itemKey={l => l.code}
                  itemActive={l => l.code === srcLang}
                  renderItem={l => <>{getLangName(l.code, langKey)} <span className="pop-dim">({l.code})</span></>}
                  onPick={l => saveSrcLang(l.code)}
                  renderHeader={srcLang ? ({ close }) => (
                    <button type="button" className="pop-item pop-dim" onMouseDown={e => { e.preventDefault(); saveSrcLang(""); close(); }}>(clear)</button>
                  ) : undefined}
                />
              </div>
              <span className="hint" style={{ fontVariantNumeric: "tabular-nums", flexShrink: 0 }}>{lyrics.length} {t("lblDlgCardChars")}</span>
            </div>
          </div>}
          {songTab === "translation" && <div className="dlg-field">
            {/* Translation blocks */}
            {translations.filter(x => !x.deleted).length === 0 && <p className="hint" style={{ textAlign: "center", padding: "16px 0" }}>{t("empDlgCardTranslationNone")}</p>}
            {translations.filter(x => !x.deleted).map(tr => {
              const expanded = expandedTranslations.has(tr.id);
              return (
                <div key={tr.id} style={{ border: `1px solid var(--border-strong)`, borderRadius: 8, marginBottom: 8, overflow: "hidden" }}>
                  <div style={{ display: "flex", alignItems: "center", gap: 6, padding: "7px 10px", background: 'var(--bg-input)', cursor: "pointer" }}
                    onClick={() => setExpandedTranslations(prev => { const s = new Set(prev); expanded ? s.delete(tr.id) : s.add(tr.id); return s; })}>
                    <ChevronDown className="icon-inline" style={{ transform: expanded ? "rotate(180deg)" : "none", transition: "transform 0.15s", color: expanded ? cfg.accent : 'var(--text-mute)', flexShrink: 0 }} />
                    <span style={{ fontSize: 13, fontWeight: 600, color: 'var(--text)', flexShrink: 0 }}>{getLangName(tr.lang, langKey)} <span style={{ color: 'var(--text-mute)', fontWeight: 400 }}>({tr.lang})</span></span>
                    <input className="input"
                      style={{ flex: 1 }}

                      value={tr.name || ""}
                      onClick={e => e.stopPropagation()}
                      onChange={e => updateTranslationLocal(tr.id, { name: e.target.value })} />
                    <button title={t("btnDlgCardTranslationAi")} disabled={translatingId === tr.id} className="btn small"
                      onClick={e => { e.stopPropagation(); handleTranslateAi(tr); }}>
                      <Sparkles />{translatingId === tr.id ? "…" : t("btnDlgCardTranslationAi")}
                    </button>
                    <button title={t("tipDlgAiCopyRequest")} onClick={e => { e.stopPropagation(); handleCopyAiTranslationRequest(tr); }}
                      className="btn icon small subtle">
                      <Sparkles />
                    </button>
                    <button title={t("tipDlgCardTranslationCopy")} onClick={e => { e.stopPropagation(); handleCopyTranslation(tr); }}
                      className="btn icon small subtle">
                      <Copy />
                    </button>
                    <button title={t("tipDlgCardTranslationDelete")} onClick={e => { e.stopPropagation(); handleDeleteTranslation(tr); }}
                      className="btn icon small subtle">
                      <Trash2 />
                    </button>
                  </div>
                  {expanded && (
                    <textarea className="textarea" style={{ width: "100%" }} rows={UIDLG.textareaRowsTranslation + rowsBonus}
                      value={tr.content}
                      onChange={e => updateTranslationLocal(tr.id, { content: e.target.value })} />
                  )}
                </div>
              );
            })}

            {/* Add translation */}
            <div style={{ marginTop: 4 }}>
              <Combobox
                placeholder={t("btnDlgCardTranslationAdd") + "…"}
                value={addLangSearch}
                onChange={setAddLangSearch}
                items={SONG_LANGUAGES.filter(l => {
                  if (translations.some(tr => tr.lang === l.code && !tr.deleted)) return false;
                  const q = addLangSearch.toLowerCase();
                  return !q || l.name.toLowerCase().includes(q) || getLangName(l.code, langKey).toLowerCase().includes(q) || l.code.toLowerCase().includes(q);
                })}
                itemKey={l => l.code}
                renderItem={l => <>{getLangName(l.code, langKey)} <span className="pop-dim">({l.code})</span></>}
                onPick={l => handleAddTranslation(l.code)}
              />
            </div>
          </div>}
          {songTab === "links" && <>
            <div className="dlg-field">
              <label className="dlg-field-label" style={{ color: cfg.accent, display: "flex", alignItems: "center", gap: 5 }}><Globe className="icon-inline" />{t("lblDlgCardExternalLinks")} <span style={{ color: 'var(--text-mute)', textTransform: "none", letterSpacing: 0, fontWeight: 400 }}>({t("msgDlgCardOptional")})</span></label>
              <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                {sortedUrls.map((u) => (
                  <UrlRow key={u.id} url={u} accent={cfg.accent} onRemove={handleRemoveUrl} />
                ))}
                <div style={{ display: "flex", gap: 6 }}>
                  <input className="input" style={{ width: 80, flexShrink: 0 }} placeholder={t("plhDlgCardUrlLabel")} value={newUrlLabel} onChange={(e) => setNewUrlLabel(e.target.value)} />
                  <input className="input" style={{ flex: 1 }} placeholder="https://suno.com/song/…" value={newUrlHref} onChange={(e) => setNewUrlHref(e.target.value)} onKeyDown={(e) => { if (e.key === "Enter") { e.preventDefault(); handleAddUrl(); } }} />
                  <button className="btn primary" onClick={handleAddUrl} disabled={!newUrlHref.trim()} style={{ "--accent": cfg.accent, "--accent-hov": cfg.accent }}>{t("btnDlgCardAdd")}</button>
                </div>
              </div>
            </div>
            <div className="dlg-field">
              <label className="dlg-field-label" style={{ color: cfg.accent, display: "flex", alignItems: "center", gap: 5 }}><Play className="icon-inline" />{t("lblDlgCardMediaFile")} <span style={{ color: 'var(--text-mute)', textTransform: "none", letterSpacing: 0, fontWeight: 400 }}>({t("msgDlgCardOptional")})</span></label>
              <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <button className="btn" onClick={browseMedia}>
                  <FolderOpen />{t("btnDlgCardBrowse")}
                </button>
                {mediaPath
                  ? <><span style={{ fontSize: 12, color: cfg.accent, flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }} title={mediaPath}>{mediaPath.split(/[\\/]/).pop()}</span>
                      <button className="btn icon small subtle" onClick={() => setMediaPath("")}><X /></button></>
                  : <span className="hint">{t("msgDlgCardNoFileSelected")}</span>}
              </div>
            </div>
          </>}
          </div>
        </>) : (<>
          <div className="tabs" style={{ margin: "0 -28px", padding: "0 28px" }}>
            {[["desc", AlignLeft, t("tabDlgCardDesc")], ["style", Music2, t("lblDlgCardStyle")], ["links", Globe, t("lblDlgCardExternalLinks")], ["note", FileText, t("tabDlgCardNote")]].map(([key, TabIcon, label]) => {
              const active = mvTab === key;
              return (
                <button key={key} onClick={() => setMvTab(key)} className={`tab${active ? " active" : ""}`} style={active ? { "--accent": cfg.accent } : undefined}>
                  <TabIcon />{label}
                </button>
              );
            })}
          </div>
          <div style={{ minHeight: card ? UIDLG.tabMinHeightEdit : UIDLG.tabMinHeightNew, display: "flex", flexDirection: "column", gap: 20 }}>
          {mvTab === "desc" && (
            <div className="dlg-field">
              <label className="dlg-field-label" style={{ color: cfg.accent }}>{t("tabDlgCardDesc")}</label>
              <textarea className="textarea"
                placeholder={t("tabDlgCardDesc") + "…"} rows={UIDLG.textareaRowsDescNote + rowsBonus}
                value={desc} onChange={(e) => setDesc(e.target.value)} />
            </div>
          )}
          {mvTab === "note" && (
            <div className="dlg-field">
              <label className="dlg-field-label" style={{ color: cfg.accent }}>{t("tabDlgCardNote")}</label>
              <textarea className="textarea"
                placeholder={t("tabDlgCardNote") + "…"} rows={UIDLG.textareaRowsDescNote + rowsBonus}
                value={note} onChange={(e) => setNote(e.target.value)} />
            </div>
          )}
          {mvTab === "style" && (
            <div className="dlg-field">
              <label className="dlg-field-label" style={{ color: cfg.accent }}>{t("lblDlgCardStyle")}</label>
              <textarea className="textarea"
                placeholder={t("plhDlgCardStyle")} rows={UIDLG.textareaRowsStyle + rowsBonus}
                value={style} onChange={(e) => { setStyle(e.target.value); setErrors((v) => ({ ...v, style: null })); }} />
              <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginTop: 4 }}>
                <button className="btn" onClick={() => { const task = type === "vocal" ? "vocal" : "music"; const lim = type === "vocal" ? (txtSettingsAiLimitsVocal || 300) : (txtSettingsAiLimitsMusic || 700); setAiModalTask(task); setAiModalInitLimit(lim); setShowAiModal(true); }}>
                  <Sparkles />{t("tipDlgAiGenerate")}
                </button>
                <span className="hint" style={{ fontVariantNumeric: "tabular-nums", color: style.length > (txtSettingsAiStyleThreshold || 1000) ? "var(--danger)" : undefined, fontWeight: style.length > (txtSettingsAiStyleThreshold || 1000) ? 700 : undefined }}>{style.length} {t("lblDlgCardChars")}</span>
              </div>
              {globalEnvId && card && (
                <div style={{ marginTop: 12, display: "flex", alignItems: "center", gap: 8 }}>
                  {card.env === globalEnvId ? (
                    <span style={{ fontSize: 12, color: 'var(--accent)', display: "flex", alignItems: "center", gap: 5 }}>
                      <Globe className="icon-inline" color={'var(--accent)'} />{t("tipPnlProjectGlobal")}
                    </span>
                  ) : (
                    <button className="btn" onClick={() => onMakeGlobal(card.id)}>
                      <Globe className="icon-inline" />{t("btnPnlProjectMakeGlobal")}
                    </button>
                  )}
                </div>
              )}
            </div>
          )}
          {mvTab === "links" && <>
            <div className="dlg-field">
              <label className="dlg-field-label" style={{ color: cfg.accent, display: "flex", alignItems: "center", gap: 5 }}><Globe className="icon-inline" />{t("lblDlgCardExternalLinks")} <span style={{ color: 'var(--text-mute)', textTransform: "none", letterSpacing: 0, fontWeight: 400 }}>({t("msgDlgCardOptional")})</span></label>
              <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                {sortedUrls.map((u) => (
                  <UrlRow key={u.id} url={u} accent={cfg.accent} onRemove={handleRemoveUrl} />
                ))}
                <div style={{ display: "flex", gap: 6 }}>
                  <input className="input" style={{ width: 80, flexShrink: 0 }} placeholder={t("plhDlgCardUrlLabel")} value={newUrlLabel} onChange={(e) => setNewUrlLabel(e.target.value)} />
                  <input className="input" style={{ flex: 1 }} placeholder="https://…" value={newUrlHref} onChange={(e) => setNewUrlHref(e.target.value)} onKeyDown={(e) => { if (e.key === "Enter") { e.preventDefault(); handleAddUrl(); } }} />
                  <button className="btn primary" onClick={handleAddUrl} disabled={!newUrlHref.trim()} style={{ "--accent": cfg.accent, "--accent-hov": cfg.accent }}>{t("btnDlgCardAdd")}</button>
                </div>
              </div>
            </div>
            <div className="dlg-field">
              <label className="dlg-field-label" style={{ color: cfg.accent, display: "flex", alignItems: "center", gap: 5 }}><Play className="icon-inline" />{t("lblDlgCardMediaFile")} <span style={{ color: 'var(--text-mute)', textTransform: "none", letterSpacing: 0, fontWeight: 400 }}>({t("msgDlgCardOptional")})</span></label>
              <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <button className="btn" onClick={browseMedia}>
                  <FolderOpen />{t("btnDlgCardBrowse")}
                </button>
                {mediaPath
                  ? <><span style={{ fontSize: 12, color: cfg.accent, flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }} title={mediaPath}>{mediaPath.split(/[\\/]/).pop()}</span>
                      <button className="btn icon small subtle" onClick={() => setMediaPath("")}><X /></button></>
                  : <span className="hint">{t("msgDlgCardNoFileSelected")}</span>}
              </div>
            </div>
          </>}
          </div>
        </>)}
        </div>
        <div className="dlg-field" style={{ padding: "12px 28px 12px", flexShrink: 0, position: "relative", zIndex: 10 }}>
          <label className="dlg-field-label" style={{ color: cfg.accent }}>{t("lblDlgCardTags")} <span style={{ color: 'var(--text-mute)', textTransform: "none", letterSpacing: 0, fontWeight: 400 }}>({t("msgDlgCardTagsHint")})</span></label>
          <Combobox
            placeholder={type === "song" ? t("plhDlgCardTagsSong") : type === "vocal" ? t("plhDlgCardTagsVocal") : t("plhDlgCardTagsMusic")}
            value={tagInput}
            onChange={setTagInput}
            items={suggestions}
            itemKey={tag => tag}
            renderItem={tag => {
              const color = getTagColor(tag) || "#888";
              const idx = tag.toLowerCase().indexOf(currentWord);
              const before = tag.slice(0, idx), match = tag.slice(idx, idx + currentWord.length), after = tag.slice(idx + currentWord.length);
              return <span className="chip" style={{ "--chip": color }}>{before}<strong>{match}</strong>{after}</span>;
            }}
            onPick={pickSuggestion}
          />
        </div>
        <div style={{ display: "flex", justifyContent: "flex-end", flexDirection: "column", gap: 10, padding: "0 28px 28px", flexShrink: 0 }}>
          {(card || type === "song") && (
            <div className="hint" style={{ alignSelf: "flex-start", display: "flex", gap: 12, alignItems: "center" }}>
              {card && <>
                <DatePicker value={dateVal} onChange={setDateVal} locale={langKey} todayLabel={t('btnGlbToday')} clearLabel={t('btnGlbClear')} />
                <span style={{ fontWeight: 700, color: cfg.accent }}>v{card.version || 1}</span>
              </>}
              {type === "song" && (() => {
                const totalStyleChars = style.length + linkedCards.reduce((sum, id) => { const p = allCards.find(x => x.id === id); return sum + (p ? p.style.length : 0); }, 0);
                return <>
                  <span className="hint" style={{ fontVariantNumeric: "tabular-nums", color: totalStyleChars > (txtSettingsAiStyleThreshold || 1000) ? "var(--danger)" : undefined, fontWeight: totalStyleChars > (txtSettingsAiStyleThreshold || 1000) ? 700 : undefined }}>{t("lblDlgCardStyleChars")} {totalStyleChars} {t("lblDlgCardChars")}</span>
                  <span className="hint" style={{ fontVariantNumeric: "tabular-nums", color: lyrics.length > (txtSettingsAiLyricsThreshold || 5000) ? "var(--danger)" : undefined, fontWeight: lyrics.length > (txtSettingsAiLyricsThreshold || 5000) ? 700 : undefined }}>{t("lblDlgCardLyricsChars")} {lyrics.length} {t("lblDlgCardChars")}</span>
                </>;
              })()}
            </div>
          )}
          <div style={{ display: "flex", gap: 10, width: "100%", alignItems: "center" }}>
            {txtSettingsAiApiKey && type === "song" && (
              <button className="btn" onClick={openWizard}>
                <Wand2 />{t("ttlDlgWizard")}
              </button>
            )}
            <div style={{ flex: 1 }} />
            <button className="btn subtle" onClick={onClose}>{t("btnGlbCancel")}</button>
            <button className="btn primary" style={{ "--accent": cfg.accent, "--accent-hov": cfg.accent }} onClick={submit}>{card ? t("btnDlgCardSaveChanges") : t("btnDlgCardCreate")}</button>
          </div>
        </div>
      </div>
      {showAiModal && (
        <AiGenerateModal
          task={aiModalTask}
          txtSettingsAiApiKey={txtSettingsAiApiKey}
          apiPort={port}
          cardId={card?.id}
          aiTweaks={aiTweaks}
          defaultTemplateId={aiDefaultTemplates?.[aiModalTask] || null}
          defaultExpertMode={tglSettingsAiTemplatesMode}
          langKey={langKey}
          initLimit={aiModalInitLimit}
          initLimitWords={aiModalTask !== "lyrics" ? txtSettingsAiStyleWordThreshold : null}
          onGenerate={(text, suggestions) => {
            if (aiModalTask === "lyrics") setLyrics(text);
            else setStyle(text);
            if (suggestions?.length) setNameSuggestions(suggestions);
          }}
          onSaveTweaks={handleSaveTweaks}
          onSetDefault={onSetDefaultTemplate}
          onClose={() => setShowAiModal(false)}
          showToast={showToast}
        />
      )}
      {/* ── Wizard overlay ── */}
      {wizardStep && (
        <div style={{ position: "fixed", inset: 0, background: 'var(--bg)', display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", zIndex: 200, padding: 40 }}>
          <div style={{ width: "100%", maxWidth: 420, display: "flex", flexDirection: "column", gap: 20 }}>
            <h3 style={{ margin: 0, fontSize: 17, fontWeight: 700, color: 'var(--text)', display: "flex", alignItems: "center", gap: 8 }}>
              <Wand2 className="icon-inline" color={'var(--accent)'} />
              {t("ttlDlgWizard")}
              <span style={{ fontSize: 12, fontWeight: 500, color: 'var(--text-mute)', marginLeft: "auto" }}>{wizardList.length > 1 ? `${wizardStep}/3` : `${wizardStep - 1}/2`}</span>
            </h3>

            {wizardStep === 1 && (
              <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                {wizardList.map(w => (
                  <button key={w.id} className="lv-item" onClick={() => { setWizardSelected(w); setWizardStep(2); }}>
                    <span className="lv-item-name">{w.name}</span>
                  </button>
                ))}
              </div>
            )}

            {wizardStep === 2 && wizardSelected && (
              <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
                {wizardList.length > 1 && (
                  <div style={{ fontSize: 12, fontWeight: 600, color: 'var(--accent)' }}>{wizardSelected.name}</div>
                )}
                <label style={{ fontSize: 13, fontWeight: 600, color: 'var(--text)' }}>{t("lblDlgWizardArtist")}</label>
                <div>
                  <Combobox
                    autoFocus
                    placeholder="Tiken Jah Fakoly, Idir, Cesária Évora…"
                    inputStyle={{ width: "100%" }}
                    value={wizardArtist}
                    onChange={setWizardArtist}
                    onKeyDown={e => { if (e.key === "Enter" && wizardArtist.trim()) handleWizardStep1(); }}
                    items={wizardArtist.trim().length > 0 ? wizardArtistHistory.filter(a => a.toLowerCase().startsWith(wizardArtist.trim().toLowerCase()) && a.toLowerCase() !== wizardArtist.trim().toLowerCase()).slice(0, 8) : []}
                    itemKey={a => a}
                    renderItem={a => a}
                    onPick={a => setWizardArtist(a)}
                  />
                </div>
              </div>
            )}

            {wizardStep === 3 && wizardDna && (
              <div style={{ display: "flex", flexDirection: "column", gap: 14 }}>
                <div style={{ display: "flex", gap: 12 }}>
                  <div style={{ flex: 1 }}>
                    <label style={{ fontSize: 11, fontWeight: 600, color: 'var(--text-mute)', textTransform: "uppercase", letterSpacing: "0.06em" }}>{t("lblDlgWizardArtist")}</label>
                    <div style={{ fontSize: 14, color: 'var(--text)', marginTop: 4 }}>{wizardArtist}</div>
                  </div>
                  <div style={{ flex: 1 }}>
                    <label style={{ fontSize: 11, fontWeight: 600, color: 'var(--text-mute)', textTransform: "uppercase", letterSpacing: "0.06em" }}>{t("lblDlgWizardLanguage")}</label>
                    <div style={{ fontSize: 14, color: 'var(--text)', marginTop: 4 }}>{wizardDna.LANGUAGE_PRIMARY || "-"}</div>
                  </div>
                </div>
                <div>
                  <label style={{ fontSize: 11, fontWeight: 600, color: 'var(--text-mute)', textTransform: "uppercase", letterSpacing: "0.06em", marginBottom: 6, display: "block" }}>{t("lblDlgWizardSubjects")}</label>
                  <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                    {(wizardDna.SUBJECTS || "").split("\n").map(l => l.replace(/^\d+\.\s*/, "").trim()).filter(Boolean).map((sub, i) => (
                      <button key={i} className={"lv-item" + (wizardSubject === sub ? " active" : "")} onClick={() => setWizardSubject(sub)}>
                        <span className="lv-item-name">{sub}</span>
                      </button>
                    ))}
                    <input
                      type="text"
                      value={wizardSubject}
                      onChange={e => setWizardSubject(e.target.value)}
                      placeholder={t("plhDlgWizardCustomSubject")}
                      style={{ marginTop: 4 }}
                      className="input"
                    />
                  </div>
                </div>
              </div>
            )}

            {wizardError && <div style={{ fontSize: 12, color: "var(--danger)", padding: "8px 12px", background: "#e74c3c18", borderRadius: 8 }}>{wizardError}</div>}

            <div style={{ display: "flex", gap: 10, justifyContent: "flex-end" }}>
              <button className="btn subtle" onClick={() => setWizardStep(null)}>
                {t("btnGlbCancel")}
              </button>
              {wizardStep > 1 && (
                <button className="btn subtle" onClick={() => setWizardStep(wizardStep === 2 && wizardList.length <= 1 ? null : wizardStep - 1)}>
                  {t("btnDlgWizardBack")}
                </button>
              )}
              {wizardStep >= 2 && (
                <button className="btn primary"
                  disabled={wizardLoading || (wizardStep === 2 && !wizardArtist.trim()) || (wizardStep === 3 && !wizardSubject)}
                  onClick={wizardStep === 2 ? handleWizardStep1 : handleWizardStep2}
                 >
                  {wizardLoading && <RotateCcw className="spinner" />}
                  {wizardLoading ? t("lblDlgWizardGenerating") : t("btnDlgWizardNext")}
                </button>
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

// ── Styles ─────────────────────────────────────────────────────────────────────
function makeStyles() {
  return {
    root:             { background: 'var(--bg)', color: 'var(--text)', fontFamily: "'Inter', system-ui, sans-serif", paddingBottom: 0, transition: "background 0.3s, color 0.3s" }, /* frame (flex column, 100vh, overflow hidden) comes from .app-root */
    header:           { display: "flex", alignItems: "center", justifyContent: "space-between", gap: 4, padding: "6px 10px", borderBottom: `1px solid var(--border)`, background: "var(--bar-bgd)", position: "sticky", top: 0, zIndex: 20, backdropFilter: "blur(12px)", flexWrap: "wrap" },
    logoWrap:         { display: "flex", alignItems: "center", gap: 10 },
    sidebar:          { width: 220, minWidth: 220, background: 'var(--pnl-bgd)', borderRight: `1px solid var(--border)`, display: "flex", flexDirection: "column", overflow: "hidden" },
    grid:             { display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(340px, 1fr))", gap: 18, padding: "20px 28px", paddingBottom: 60, overflowY: "auto", flex: 1, minHeight: 0, alignContent: "start" },
    list:             { display: "flex", flexDirection: "column", gap: 10, padding: "20px 28px", paddingBottom: 60, overflowY: "auto", flex: 1, minHeight: 0 },
    card:             { background: 'var(--bg-elev)', borderWidth: "1px", borderStyle: "solid", borderColor: 'var(--border)', borderRadius: 12, padding: 20, display: "flex", flexDirection: "column", gap: 12, transition: "all 0.2s" },
    cardTop:          { display: "flex", justifyContent: "space-between", alignItems: "center" },
    cardTitle:        { margin: 0, fontSize: 16, fontWeight: 700, color: 'var(--text)', lineHeight: 1.3 },
    cardBody:         { margin: 0, fontSize: 13, color: 'var(--text-dim)', lineHeight: 1.65, whiteSpace: "pre-wrap", wordBreak: "break-word" },
    cardTags:         { display: "flex", flexWrap: "wrap", gap: 6 },
    inlineTag:        { fontSize: 11, border: "1px solid", borderRadius: 4, padding: "2px 8px", opacity: 0.85 },
    cardActions:      { display: "flex", gap: 8, borderTop: `1px solid var(--border)`, paddingTop: 12, justifyContent: "flex-end" },
    modalHead:        { display: "flex", justifyContent: "space-between", alignItems: "center" },
    errBorder:        { borderColor: "var(--danger)" },
    errMsg:           { fontSize: 12, color: "var(--danger)" },
  };
}

function buildCss() {
  return `
    @import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&family=JetBrains+Mono:wght@400;500;600&display=swap');
    /* Palette tokens (--bg, --accent, --border, --text, --shadow, --gradient, …) come from
       lib/ui-colors.css (imported first in index.jsx) — the canonical :root for both themes.
       Shared classes (tokens/chrome/dialog/list-view) resolve against those; no token bridge
       is emitted here. ⚠ CLAUDE: never re-add a :root palette bridge — ui-colors.css owns it. */
    * { box-sizing: border-box; }
    button:focus-visible { outline: 2px solid var(--accent) !important; outline-offset: 2px; border-radius: 4px; }
    body { margin: 0; background: var(--bg); font-family: 'Inter', system-ui, sans-serif; }
    /* ── Shared header (GUI Standard Rules 9 & 10): .barh-app-name + .barh-app-version are
       unscoped in ui-app.css (bg = --bar-bgd). Not redefined here. */
    .act-btn:hover { border-color: #8888ff88 !important; color: var(--accent) !important; background: var(--bg-hov) !important; }
    .act-btn-danger:hover { border-color: var(--danger) !important; }
    @keyframes spin { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }
    /* Splitters now use the shared <Splitter> (lib/ui-ctl-splitter.jsx) - its .app-splitter look is
       self-injected; the old .sidebar-resize rules were removed (this app is the reference). */
    .card-item:hover { border-color: var(--card-accent) !important; box-shadow: 0 4px 24px rgba(0,0,0,0.15); transform: translateY(-2px); }
  `;
}
