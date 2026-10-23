import { app } from "electron";
import path from "path";
import fs from "fs";

const FEED_URL = "https://apps.yaiol.com/news.json";
const FETCH_TIMEOUT_MS = 8000;

const stateFile = () => path.join(app.getPath("appData"), "yaiol", "news-state.json");

export function readNewsState() {
  try {
    const raw = JSON.parse(fs.readFileSync(stateFile(), "utf8"));
    return {
      enabled: raw.enabled !== false,
      dismissed: Array.isArray(raw.dismissed) ? raw.dismissed.filter(id => typeof id === "string") : [],
    };
  } catch {
    return { enabled: true, dismissed: [] };
  }
}

function writeNewsState(state) {
  const file = stateFile();
  const tmp = `${file}.${process.pid}.tmp`;
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(tmp, JSON.stringify(state, null, 2), "utf8");
  fs.renameSync(tmp, file);
}

function updateNewsState(change) {
  const next = { ...readNewsState(), ...change };
  writeNewsState(next);
  return next;
}

async function loadFeed() {
  if (process.env.YAIOL_DEV) return null;
  const res = await fetch(FEED_URL, { cache: "no-cache", signal: AbortSignal.timeout(FETCH_TIMEOUT_MS) });
  return res.ok ? await res.json() : null;
}

// The edition of the announcement format this reader understands. An announcement carries its own `version`
// (the cockpit writes the lowest edition that can express it); one written in a LATER edition is skipped, because
// this reader cannot know what restriction or feature it adds. A missing number reads as 1.
const READER_VERSION = 1;

const isReadable = (version) => !(typeof version === "number" && version > READER_VERSION);

// "Who sees it": `only` these apps, or everyone `all-but` these. Anything else is seen by nobody.
function isTargeted(target, appId) {
  const apps = Array.isArray(target?.apps) ? target.apps : [];
  if (target?.mode === "only") return apps.includes(appId);
  if (target?.mode === "all-but") return !apps.includes(appId);
  return false;
}

function isNotStarted(start) {
  return typeof start === "string" && new Date(`${start}T00:00:00`).getTime() > Date.now();
}

function isExpired(end) {
  return typeof end === "string" && new Date(`${end}T23:59:59`).getTime() < Date.now();
}

// `lang` is { <language>: { title, subtitle?, text?, image? } }: an announcement is shown only when some language has a title.
const hasTitle = (langs) => !!langs && typeof langs === "object" && Object.values(langs).some(b => typeof b?.title === "string" && b.title);

async function currentAnnouncements({ appId }) {
  const state = readNewsState();
  if (!state.enabled) return [];
  const feed = await loadFeed();
  const list = Array.isArray(feed?.announcements) ? feed.announcements : [];
  // A draft (`draft: true`) is never shown to a user, even if one slips into the published feed.
  return list
    .filter(a => a && typeof a.id === "string" && a.draft !== true && isReadable(a.version) && !state.dismissed.includes(a.id) && !isNotStarted(a.start) && !isExpired(a.end) && isTargeted(a.target, appId) && hasTitle(a.lang))
    .map(a => ({ id: a.id, lang: a.lang, link: typeof a.link === "string" ? a.link : null }));
}

const SITE_APPS = "https://apps.yaiol.com";

// Compare release versions ("1.2.3" vs "1.2.10"). Returns > 0 when a is newer than b.
function cmpVersion(a, b) {
  const pa = String(a).split(".").map(n => parseInt(n, 10) || 0);
  const pb = String(b).split(".").map(n => parseInt(n, 10) || 0);
  for (let i = 0; i < Math.max(pa.length, pb.length); i++) {
    const d = (pa[i] || 0) - (pb[i] || 0);
    if (d !== 0) return d;
  }
  return 0;
}

async function fetchJson(url) {
  const res = await fetch(url, { cache: "no-cache", signal: AbortSignal.timeout(FETCH_TIMEOUT_MS) });
  return res.ok ? await res.json() : null;
}

// "A newer version exists": ONE card, in the dialog's item shape. The beacon (p/<id>/latest.json) says which
// version is out; only then is the notes file of the reader's language fetched (own language, then its base
// language, then English), and the release blocks newer than the version running make up the card's text.
// Its id names the latest version, so "don't show again" silences the update until a newer one comes out.
// The news switch does NOT apply: it is about announcements only.
async function currentUpdate({ appId, appVersion, lang }) {
  if (process.env.YAIOL_DEV || !appVersion) return [];
  const base = `${SITE_APPS}/p/${appId}`;
  const latest = (await fetchJson(`${base}/latest.json`))?.version;
  if (typeof latest !== "string" || cmpVersion(latest, appVersion) <= 0) return [];
  const id = `update:${appId}:${latest}`;
  if (readNewsState().dismissed.includes(id)) return [];
  const code = String(lang || "en").replace(/_/g, "-");
  let releases = [];
  for (const k of [...new Set([code, code.split("-")[0], "en"])]) {
    try {
      const list = (await fetchJson(`${base}/${k}.json`))?.releases;
      if (Array.isArray(list)) { releases = list; break; }
    } catch { /* try the next language */ }
  }
  const link = `${SITE_APPS}/${code}/p/${appId}/download/`;
  const newer = releases.filter(r => r && typeof r.version === "string" && typeof r.text === "string" && cmpVersion(r.version, appVersion) > 0);
  // ONE card for the whole update: a single new version is just its notes; several are listed newest first, each under its
  // own bold line. No notes at all (file missing, or older than the beacon) still tells the reader a version is out.
  const text = newer.length > 1
    ? newer.map(r => `**v${r.version}${r.date ? ` — ${r.date}` : ""}**\n\n${r.text}`).join("\n\n")
    : (newer[0]?.text || "");
  return [{ id, kind: "update", version: latest, text, link }];
}

export function registerNewsRoutes(api, { appId, appVersion }) {
  // Update cards first, then the announcements; each half fails on its own.
  api.get("/news", async (req, res) => {
    const lang = typeof req.query?.lang === "string" ? req.query.lang : "en";
    const [update, news] = await Promise.all([
      currentUpdate({ appId, appVersion, lang }).catch(() => []),
      currentAnnouncements({ appId }).catch(() => []),
    ]);
    res.json([...update, ...news]);
  });

  api.get("/news-settings", (req, res) => {
    res.json({ enabled: readNewsState().enabled });
  });

  api.post("/news-settings", (req, res) => {
    res.json({ enabled: updateNewsState({ enabled: req.body?.enabled !== false }).enabled });
  });

  api.post("/news-dismiss", (req, res) => {
    const ids = (Array.isArray(req.body?.ids) ? req.body.ids : []).filter(id => typeof id === "string" && id);
    if (!ids.length) { res.json({ ok: false }); return; }
    const state = readNewsState();
    const fresh = ids.filter(id => !state.dismissed.includes(id));
    if (fresh.length) updateNewsState({ dismissed: [...state.dismissed, ...fresh] });
    res.json({ ok: true });
  });
}
