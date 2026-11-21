// ⚠ NO HARDCODED UI STRINGS IN THIS FILE.
//   Every string a user can read MUST go through the t() lookup, and the key MUST be a string literal at the call site.

//! The workspace on screen: what is loaded (`Ui` - signals, so it is `Copy`) and every operation the window runs.
//!
//! The database is read on the UI thread (it is a local file and the calls are short); what waits on the
//! network (Gemini, a Suno page) runs in `spawn_blocking` and never touches the store.

use crate::{catalog::toast::LbsToast, i18n};
use dioxus::prelude::*;
use engine::{
    cards::{CardPatch, NewCard},
    exports,
    media,
    projects::ProjectPatch,
    store::{Card, Environment, Project, Store, new_id},
};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    path::{Path, PathBuf},
    rc::Rc,
};

/// What the sidebar has open.
#[derive(Clone, PartialEq)]
pub enum Nav {
    All,
    Unsorted,
    Untagged,
    Project(String),
}

/// The big views of the work area.
#[derive(Clone, Copy, PartialEq)]
pub enum View {
    Cards,
    Sort,
    Link,
    Media,
}

/// What a confirm question will do when answered yes.
#[derive(Clone, PartialEq)]
pub enum Act {
    DeleteCard(String),
    DeleteSelected,
    UnlinkSelected,
    /// Make the songs of a folder, in this project.
    ImportFolder { project: String, folder: String },
}

/// The dialog on screen, if any. Dialogs that edit a card or a project open from the id and keep their own draft.
#[derive(Clone, PartialEq)]
pub enum Dialog {
    None,
    /// A card to edit (`id`), or a new one: of this `kind`, filled from a `draft` when there is one.
    Card { id: Option<String>, kind: String, draft: Option<Card> },
    Project { id: Option<String>, parent: Option<String> },
    DeleteProject(String),
    Confirm { title: String, sub: String, act: Act },
    /// The cards a tag is added to or taken from, and the words that say whose they are (fixed when the dialog opens).
    Tag { ids: Vec<String>, scope: String },
    /// Translate the songs of these cards.
    Translate(Vec<String>),
    MoveEnv(Vec<String>),
    /// Move a project, its sub-projects and their cards to another environment.
    MoveFolder(String),
    AddToProject(Vec<String>),
    Doc,
    Info { title: String, lines: Vec<String> },
    /// What a switch of the stored paths would do, shown before anything is written (Electron's `mediaAudit`).
    MediaPaths(engine::extras::MediaPlan),
}

#[derive(Clone, Copy, PartialEq)]
pub enum Tone {
    Plain,
    Ok,
    Err,
}

#[derive(Clone, Copy)]
pub struct Ui {
    pub store: Signal<Rc<Store>>,
    pub lang: Signal<String>,
    pub toast: LbsToast,
    pub cards: Signal<Vec<Card>>,
    pub projects: Signal<Vec<Project>>,
    pub envs: Signal<Vec<Environment>>,
    pub trans: Signal<HashMap<String, String>>,
    pub env: Signal<String>,
    pub nav: Signal<Nav>,
    pub view: Signal<View>,
    pub search: Signal<String>,
    pub kind: Signal<String>,
    pub tags: Signal<Vec<String>>,
    pub fav: Signal<bool>,
    /// `""`, `tagged`, `media`, `lrc`, `ai`, `published`: the warning filter in force.
    pub warn: Signal<String>,
    pub sort_name: Signal<bool>,
    pub sort_asc: Signal<bool>,
    /// The projects' own order in the sidebar: by name (else by date), ascending (else descending).
    pub proj_sort_name: Signal<bool>,
    pub proj_sort_asc: Signal<bool>,
    pub list_view: Signal<bool>,
    pub selected: Signal<HashSet<String>>,
    pub expanded: Signal<HashSet<String>>,
    pub dialog: Signal<Dialog>,
    /// A draft is being made (the clipboard's Suno link, a dropped file): nothing else starts meanwhile.
    pub draft_busy: Signal<bool>,
    /// What is being dragged inside the window, and where the pointer is (only the ghost reads the position).
    pub drag: Signal<crate::dnd::DragState>,
    pub drag_at: Signal<(f64, f64)>,
    /// The Sort view's "rename media on sort" box: session state, starting from the setting.
    pub sort_rename: Signal<bool>,
    pub copied: Signal<String>,
    pub status: Signal<(Tone, String)>,
    /// The card on the player bar (empty = no bar), the cards the bar walks through, and whether it is sounding now.
    pub playing: Signal<String>,
    pub playlist: Signal<Vec<String>>,
    pub sounding: Signal<bool>,
    /// The card colours the user chose (`type-colors`); a key that is absent has its default (`COLOUR_KEYS`).
    pub colours: Signal<BTreeMap<String, String>>,
    /// The colours the user gave to tags (`tag-colors`); every other tag has its automatic colour.
    pub tag_colours: Signal<BTreeMap<String, String>>,
}

/// The six card colours and their defaults (the Cards settings of the Electron app; the keys are its own).
pub const COLOUR_KEYS: [(&str, &str); 6] = [
    ("note", "#22c55e"),
    ("music", "#ff6eb4"),
    ("vocal", "#f0c040"),
    ("SongStateCreated", "#3f61d2"),
    ("SongStateUndefined", "#60a5fa"),
    ("SongStatePublished", "#645492"),
];

const TAG_PALETTE: [&str; 16] = [
    "#4dc8c8", "#ff6eb4", "#a78bfa", "#f0c040", "#5dba6f", "#e06060", "#60a5fa", "#fb923c", "#34d399", "#f87171", "#38bdf8", "#e879f9", "#fbbf24", "#a3e635", "#7de8e8", "#ff9ecf",
];

/// A tag's automatic colour: a hash of its name, so the same tag is the same colour everywhere.
pub fn tag_palette_colour(tag: &str) -> &'static str {
    let mut h: i32 = 0;
    for u in tag.encode_utf16() {
        h = h.wrapping_shl(5).wrapping_sub(h).wrapping_add(u as i32);
    }
    TAG_PALETTE[(h as i64).unsigned_abs() as usize % TAG_PALETTE.len()]
}

/// A JSON object of colours as the Electron app saves it (`type-colors`, `tag-colors`).
fn colours_from(text: &str) -> BTreeMap<String, String> {
    serde_json::from_str::<BTreeMap<String, String>>(text).unwrap_or_default()
}

pub fn load_colours(st: &engine::store::Store) -> (BTreeMap<String, String>, BTreeMap<String, String>) {
    let mut types = colours_from(&st.setting("type-colors"));
    // an old version of the Electron app called vocal "vocalist"
    if let Some(v) = types.remove("vocalist") {
        types.entry("vocal".to_owned()).or_insert(v);
    }
    (types, colours_from(&st.setting("tag-colors")))
}

pub fn title_of(c: &Card) -> String {
    match c.sort_number {
        Some(n) => format!("{n:02}-{}", c.name),
        None => c.name.clone(),
    }
}

fn list_of(setting: String, default: &str) -> Vec<String> {
    let text = if setting.trim().is_empty() { default.to_owned() } else { setting };
    text.split(',').map(|s| s.trim().to_lowercase()).filter(|s| !s.is_empty()).collect()
}

fn type_rank(kind: &str) -> u8 {
    match kind {
        "note" => 0,
        "music" => 1,
        "vocal" => 2,
        _ => 3,
    }
}

impl Ui {
    pub fn t(self, key: &str) -> String {
        i18n::t(&self.lang.peek(), key)
    }

    pub fn st(self) -> Rc<Store> {
        self.store.peek().clone()
    }

    pub fn ok(self, text: impl Into<String>) {
        let mut status = self.status;
        status.set((Tone::Ok, String::new()));
        self.toast.show(text);
    }

    /// An error stays on screen, in the status line: it can be read, and copied.
    pub fn fail(self, text: impl Into<String>) {
        let mut status = self.status;
        status.set((Tone::Err, text.into()));
    }

    pub fn ai_sites(self) -> Vec<String> {
        list_of(self.st().setting("ai-sites"), "udio, suno, producer, tunee")
    }

    pub fn music_sites(self) -> Vec<String> {
        list_of(self.st().setting("music-sites"), "soundcloud")
    }

    /// One of the six card colours, by its key (`COLOUR_KEYS`).
    pub fn colour(self, key: &str) -> String {
        if let Some(c) = self.colours.read().get(key) {
            return c.clone();
        }
        COLOUR_KEYS.iter().find(|(k, _)| *k == key).map_or("#60a5fa", |(_, d)| d).to_owned()
    }

    /// A tag's colour: the user's, else the automatic one.
    pub fn tag_colour(self, tag: &str) -> String {
        self.tag_colours.read().get(tag).cloned().unwrap_or_else(|| tag_palette_colour(tag).to_owned())
    }

    /// The song's state: published when it has a link to a music site, created when it has one to an AI site.
    pub fn song_state(self, c: &Card) -> &'static str {
        let labels: Vec<String> = c.urls.iter().map(|u| u.label.trim().to_lowercase()).collect();
        if labels.iter().any(|l| self.music_sites().contains(l)) {
            "SongStatePublished"
        } else if labels.iter().any(|l| self.ai_sites().contains(l)) {
            "SongStateCreated"
        } else {
            "SongStateUndefined"
        }
    }

    pub fn is_published(self, c: &Card) -> bool {
        c.kind == "song" && self.song_state(c) == "SongStatePublished"
    }

    /// The colour a card is drawn in: its kind's, and for a song the colour of its state.
    pub fn colour_of(self, c: &Card) -> String {
        if c.kind == "song" { self.colour(self.song_state(c)) } else { self.colour(&c.kind) }
    }

    /// A kind's own colour; "song" has none of its own and takes the one of a song with no link.
    pub fn kind_colour(self, kind: &str) -> String {
        self.colour(if kind == "song" { "SongStateUndefined" } else { kind })
    }

    pub fn set_colours(self, map: BTreeMap<String, String>) {
        self.st().set_setting("type-colors", &serde_json::to_string(&map).unwrap_or_default());
        let mut s = self.colours;
        s.set(map);
    }

    pub fn set_tag_colours(self, map: BTreeMap<String, String>) {
        self.st().set_setting("tag-colors", &serde_json::to_string(&map).unwrap_or_default());
        let mut s = self.tag_colours;
        s.set(map);
    }

    // ─── Loading ─────────────────────────────────────────────────────────────────────────────────

    pub fn reload(self) {
        let st = self.st();
        let (mut cards, mut projects, mut envs, mut trans, mut env) = (self.cards, self.projects, self.envs, self.trans, self.env);
        match (st.cards(), st.projects(), st.environments()) {
            (Ok(c), Ok(p), Ok(e)) => {
                if !e.iter().any(|x| x.id == *env.peek()) {
                    env.set(st.default_env.clone());
                }
                cards.set(c);
                projects.set(p);
                envs.set(e);
            }
            (c, p, e) => self.fail(c.err().or(p.err()).or(e.err()).unwrap_or_default()),
        }
        if let Ok(index) = st.translation_index() {
            trans.set(index);
        }
    }

    pub fn card(self, id: &str) -> Option<Card> {
        self.cards.peek().iter().find(|c| c.id == id).cloned()
    }

    pub fn global_env(self) -> String {
        self.st().global_env.clone()
    }

    // ─── What is shown ───────────────────────────────────────────────────────────────────────────

    /// The cards of this environment, plus the Global music and vocal cards a song of it links to.
    pub fn env_cards(self) -> Vec<Card> {
        let (cards, env, global) = (self.cards.read(), self.env.read().clone(), self.global_env());
        let songs: HashSet<&String> = cards.iter().filter(|c| c.env == env && c.kind == "song").map(|c| &c.id).collect();
        let used: HashSet<String> = cards
            .iter()
            .filter(|c| songs.contains(&c.id))
            .flat_map(|s| s.linked_cards.iter().cloned())
            .collect();
        cards.iter().filter(|c| c.env == env || (c.env == global && (c.kind == "music" || c.kind == "vocal") && used.contains(&c.id))).cloned().collect()
    }

    pub fn env_projects(self) -> Vec<Project> {
        let env = self.env.read().clone();
        self.projects.read().iter().filter(|p| p.env == env).cloned().collect()
    }

    /// The ids of a project and everything below it.
    pub fn project_tree(self, id: &str) -> Vec<String> {
        let projects = self.env_projects();
        let mut out = vec![id.to_owned()];
        let mut i = 0;
        while i < out.len() {
            let parent = out[i].clone();
            for p in projects.iter().filter(|p| p.parent_id.as_deref() == Some(parent.as_str())) {
                out.push(p.id.clone());
            }
            i += 1;
        }
        out
    }

    fn matches_search(self, c: &Card, q: &str) -> bool {
        let q = q.trim().to_lowercase();
        if q.is_empty() {
            return true;
        }
        let (mode, needle) = match q.get(..2) {
            Some(p @ ("n:" | "s:" | "l:")) => (p.chars().next().unwrap_or(' '), q[2..].trim().to_owned()),
            _ => (' ', q),
        };
        if needle.is_empty() {
            return true;
        }
        let has = |s: &str| s.to_lowercase().contains(&needle);
        match mode {
            'n' => has(&c.name),
            's' => has(&c.style) || has(&c.reference_artist) || has(&c.reference_complement),
            'l' => has(&c.lyrics) || self.trans.read().get(&c.id).is_some_and(|t| has(t)),
            _ => has(&c.name) || has(&c.style) || has(&c.desc) || has(&c.note) || has(&c.lyrics) || c.tags.iter().any(|t| has(t)) || self.trans.read().get(&c.id).is_some_and(|t| has(t)),
        }
    }

    /// The cards of the open project (or of the whole environment), before the kind and the warning narrow them.
    fn nav_cards(self) -> Vec<Card> {
        let env_cards = self.env_cards();
        let nav = self.nav.read().clone();
        match &nav {
            Nav::Unsorted => {
                let linked: HashSet<String> = self.env_projects().iter().flat_map(|p| p.card_ids.iter().cloned()).collect();
                env_cards.into_iter().filter(|c| !linked.contains(&c.id)).collect()
            }
            Nav::Untagged => env_cards.into_iter().filter(|c| c.tags.is_empty()).collect(),
            Nav::Project(id) => {
                let projects = self.env_projects();
                let ids: HashSet<String> = self.project_tree(id).iter().filter_map(|p| projects.iter().find(|x| &x.id == p)).flat_map(|p| p.card_ids.iter().cloned()).collect();
                self.cards.read().iter().filter(|c| ids.contains(&c.id)).cloned().collect()
            }
            Nav::All => env_cards,
        }
    }

    /// The cards of the open project / kind / warning, before the search, tags and favourites narrow them.
    pub fn source_cards(self) -> Vec<Card> {
        if matches!(*self.nav.read(), Nav::Unsorted | Nav::Untagged) {
            return self.nav_cards();
        }
        let mut base = self.nav_cards();
        let kind = self.kind.read().clone();
        if !kind.is_empty() {
            base.retain(|c| c.kind == kind);
        }
        let w = self.warn.read().clone();
        if !w.is_empty() {
            let ai = self.ai_sites();
            base.retain(|c| self.warn_matches(&w, c, &ai));
        }
        base
    }

    /// Does the card trip this warning (`tagged`, `ai`, `media`, `lrc`, `published`)?
    fn warn_matches(self, warn: &str, c: &Card, ai: &[String]) -> bool {
        match warn {
            "tagged" => c.tags.is_empty(),
            "ai" => c.kind == "song" && !c.urls.iter().any(|u| ai.contains(&u.label.trim().to_lowercase())),
            "media" => c.kind == "song" && c.media_path.is_empty(),
            "lrc" => c.kind == "song" && !exports::clean_lyrics(&c.lyrics).trim().is_empty() && !c.media_path.is_empty() && !media::lrc_of(Path::new(&c.media_path)).exists(),
            "published" => c.kind == "song" && !self.is_published(c),
            _ => false,
        }
    }

    pub fn unsorted_count(self) -> usize {
        let linked: HashSet<String> = self.env_projects().iter().flat_map(|p| p.card_ids.iter().cloned()).collect();
        self.env_cards().iter().filter(|c| !linked.contains(&c.id)).count()
    }

    /// How many cards of the open project (or of everything) trip each warning, in the order of the header's buttons.
    pub fn warn_counts(self) -> [usize; 5] {
        let nav = self.nav.read().clone();
        let scope = if matches!(nav, Nav::Project(_)) { self.nav_cards() } else { self.env_cards() };
        let ai = self.ai_sites();
        ["tagged", "media", "lrc", "ai", "published"].map(|w| scope.iter().filter(|c| self.warn_matches(w, c, &ai)).count())
    }

    /// The projects in the order the sidebar shows them.
    pub fn sorted_projects(self, mut list: Vec<Project>) -> Vec<Project> {
        let (by_name, asc) = (*self.proj_sort_name.read(), *self.proj_sort_asc.read());
        list.sort_by(|a, b| {
            let o = if by_name { a.name.to_lowercase().cmp(&b.name.to_lowercase()) } else { a.date.cmp(&b.date) };
            if asc { o } else { o.reverse() }
        });
        list
    }

    pub fn shown_cards(self) -> Vec<Card> {
        let q = self.search.read().clone();
        let tags = self.tags.read().clone();
        let fav = *self.fav.read();
        let (by_name, asc) = (*self.sort_name.read(), *self.sort_asc.read());
        let mut out: Vec<Card> = self.source_cards().into_iter().filter(|c| (tags.is_empty() || tags.iter().all(|t| c.tags.contains(t))) && (!fav || c.favorite) && self.matches_search(c, &q)).collect();
        out.sort_by(|a, b| {
            let ranked = type_rank(&a.kind).cmp(&type_rank(&b.kind));
            if ranked != std::cmp::Ordering::Equal {
                return ranked;
            }
            let o = if by_name {
                let key = |c: &Card| title_of(c).to_lowercase();
                key(a).cmp(&key(b)).then(a.version.cmp(&b.version))
            } else {
                a.date.cmp(&b.date)
            };
            if asc { o } else { o.reverse() }
        });
        out
    }

    pub fn all_tags(self) -> Vec<String> {
        let mut tags: Vec<String> = self.source_cards().iter().flat_map(|c| c.tags.iter().cloned()).collect::<HashSet<_>>().into_iter().collect();
        tags.sort();
        tags
    }

    // ─── Cards ───────────────────────────────────────────────────────────────────────────────────

    pub fn copy(self, c: &Card, mode: &str) {
        let (text, key, msg) = match mode {
            "title" => (title_of(c), format!("{}_title", c.id), self.t("tstCardTitleCopied")),
            "lyrics" => (c.lyrics.clone(), format!("{}_lyrics", c.id), self.t("tstCardLyricsCopied")),
            "linked" => {
                let linked: Vec<Card> = c.linked_cards.iter().filter_map(|id| self.card(id)).collect();
                let mut parts: Vec<String> = Vec::new();
                if !c.style.trim().is_empty() {
                    parts.push(c.style.trim().to_owned());
                }
                parts.extend(linked.iter().map(|l| l.style.clone()).filter(|s| !s.is_empty()));
                (parts.join("\n"), format!("{}_linked", c.id), self.t("tstCardLinkedCopied").replace("{n}", &linked.len().to_string()))
            }
            _ => (c.style.clone(), c.id.clone(), self.t("tstCardCopied")),
        };
        if arboard::Clipboard::new().and_then(|mut b| b.set_text(text)).is_err() {
            return;
        }
        let mut copied = self.copied;
        copied.set(key.clone());
        self.ok(msg);
        spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(1800)).await;
            if *copied.peek() == key {
                copied.set(String::new());
            }
        });
    }

    pub fn toggle_fav(self, id: &str) {
        let Some(c) = self.card(id) else { return };
        if let Err(e) = self.st().update_card(id, &CardPatch { favorite: Some(!c.favorite), ..Default::default() }) {
            return self.fail(e);
        }
        self.reload();
    }

    pub fn toggle_select(self, id: &str) {
        let mut selected = self.selected;
        let mut set = selected.write();
        if !set.remove(id) {
            set.insert(id.to_owned());
        }
    }

    /// What the selection holds, as the cards still on screen: a selection left behind a filter is dropped.
    pub fn selected_ids(self) -> Vec<String> {
        let shown: HashSet<String> = self.shown_cards().into_iter().map(|c| c.id).collect();
        self.selected.read().iter().filter(|id| shown.contains(*id)).cloned().collect()
    }

    pub fn delete_card(self, id: &str) {
        match self.st().delete_card(id) {
            Ok(_) => {
                let mut selected = self.selected;
                selected.write().remove(id);
                self.reload();
                self.ok(self.t("tstAppCardDeleted"));
            }
            Err(e) => self.fail(e),
        }
    }

    pub fn delete_selected(self) {
        let ids = self.selected_ids();
        let mut failed = 0;
        for id in &ids {
            if self.st().delete_card(id).is_err() {
                failed += 1;
            }
        }
        let mut selected = self.selected;
        selected.set(HashSet::new());
        self.reload();
        if failed > 0 {
            self.fail(format!("{failed} {}", self.t("tstAppBulkDeleteFailed")));
        } else {
            self.ok(self.t("tstAppCardDeleted"));
        }
    }

    pub fn unlink_selected(self) {
        let Nav::Project(project) = self.nav.peek().clone() else { return };
        for id in self.selected_ids() {
            let _ = self.st().remove_card_from_project(&project, &id);
        }
        let mut selected = self.selected;
        selected.set(HashSet::new());
        self.reload();
        self.ok(self.t("tstAppRemovedFromProject"));
    }

    /// Add and take tags on these cards; the number of cards that changed.
    pub fn tag_cards(self, ids: &[String], add: &[String], remove: &[String]) -> usize {
        let mut changed = 0;
        for id in ids {
            let Some(c) = self.card(id) else { continue };
            let mut tags: Vec<String> = c.tags.iter().filter(|t| !remove.contains(t)).cloned().collect();
            for t in add {
                if !tags.contains(t) {
                    tags.push(t.clone());
                }
            }
            if tags != c.tags && self.st().update_card(id, &CardPatch { tags: Some(tags), ..Default::default() }).is_ok() {
                changed += 1;
            }
        }
        self.reload();
        changed
    }

    /// The cards of a project and of every project below it.
    pub fn tree_card_ids(self, project: &str) -> Vec<String> {
        let tree = self.project_tree(project);
        let mut out: Vec<String> = Vec::new();
        for p in self.env_projects().iter().filter(|p| tree.contains(&p.id)) {
            for id in &p.card_ids {
                if !out.contains(id) {
                    out.push(id.clone());
                }
            }
        }
        out
    }

    /// A new card, in the open project when there is one.
    pub fn create_card(self, new: NewCard) -> Option<Card> {
        let st = self.st();
        match st.create_card(&NewCard { env: Some(self.env.peek().clone()), ..new }) {
            Ok(card) => {
                if let Nav::Project(p) = self.nav.peek().clone() {
                    let _ = st.add_card_to_project(&p, &card.id);
                }
                self.reload();
                self.ok(self.t("tstAppCardCreated"));
                Some(card)
            }
            Err(e) => {
                self.fail(e);
                None
            }
        }
    }

    pub fn save_card(self, id: &str, patch: &CardPatch) -> bool {
        match self.st().update_card(id, patch) {
            Ok(_) => {
                self.reload();
                self.ok(self.t("tstAppCardUpdated"));
                true
            }
            Err(e) => {
                self.fail(e);
                false
            }
        }
    }

    // ─── Projects and environments ───────────────────────────────────────────────────────────────

    pub fn delete_project(self, id: &str, with_cards: bool) {
        match self.st().delete_project(id, with_cards) {
            Ok(_) => {
                if *self.nav.peek() == Nav::Project(id.to_owned()) {
                    let mut nav = self.nav;
                    nav.set(Nav::All);
                }
                self.reload();
                self.ok(self.t("tstAppProjectDeleted"));
            }
            Err(e) => self.fail(e),
        }
    }

    pub fn open_card(self, id: &str) {
        let mut dialog = self.dialog;
        dialog.set(Dialog::Card { id: Some(id.to_owned()), kind: String::new(), draft: None });
    }

    pub fn close(self) {
        let mut dialog = self.dialog;
        dialog.set(Dialog::None);
    }

    pub fn answer(self, yes: bool) {
        let asked = self.dialog.peek().clone();
        self.close();
        if let (true, Dialog::Confirm { act, .. }) = (yes, asked) {
            match act {
                Act::DeleteCard(id) => self.delete_card(&id),
                Act::DeleteSelected => self.delete_selected(),
                Act::UnlinkSelected => self.unlink_selected(),
                Act::ImportFolder { project, folder } => self.import_folder(&project, &folder),
            }
        }
    }

    /// Every media file of a folder becomes a song of this project, as a drop of those files would make them
    /// (filled from Suno when a file says which song it is); the project then points at the folder.
    pub fn import_folder(self, project: &str, folder: &str) {
        let files = match media::folder_tracks(Path::new(folder)) {
            Ok(f) => f,
            Err(e) => return self.fail(e.to_string()),
        };
        if files.is_empty() {
            return self.fail(self.t("tstAppImportJSONNothing"));
        }
        let (project, folder) = (project.to_owned(), folder.to_owned());
        spawn(async move {
            self.create_cards_from_media(files, Some(project.clone())).await;
            let _ = self.st().update_project(&project, &ProjectPatch { path: Some(folder), ..Default::default() });
            self.reload();
            let mut nav = self.nav;
            nav.set(Nav::Project(project));
        });
    }

    /// New card: a Suno link in the clipboard pre-fills it (name, number, style, lyrics, date, link); anything
    /// else opens it empty, a song as always.
    pub fn open_new_card(self) {
        if *self.draft_busy.peek() {
            return;
        }
        let text = arboard::Clipboard::new().and_then(|mut c| c.get_text()).unwrap_or_default();
        let (mut busy, mut dialog) = (self.draft_busy, self.dialog);
        busy.set(true);
        spawn(async move {
            let draft = tokio::task::spawn_blocking(move || engine::drafts::from_clipboard(&text)).await.ok().flatten();
            busy.set(false);
            dialog.set(Dialog::Card { id: None, kind: "song".to_owned(), draft });
        });
    }

    /// One dropped track: the editor opens on a song linked to that file, filled from Suno when the file's tags
    /// say which song it is. A file that names a song Suno no longer has opens nothing.
    pub fn open_card_from_media(self, path: PathBuf) {
        if *self.draft_busy.peek() {
            return;
        }
        let (mut busy, mut dialog) = (self.draft_busy, self.dialog);
        busy.set(true);
        spawn(async move {
            let got = tokio::task::spawn_blocking(move || engine::drafts::from_media(&path)).await;
            busy.set(false);
            match got {
                Ok(Ok(draft)) => dialog.set(Dialog::Card { id: None, kind: "song".to_owned(), draft: Some(draft) }),
                Ok(Err(_)) => self.fail(self.t("tstAppDropMediaSongGone").replace("{n}", "1")),
                Err(e) => self.fail(e.to_string()),
            }
        });
    }

    /// Several tracks are a BULK IMPORT, not an edit: each becomes a card straight away, in the order given,
    /// with no dialog in the way. One at a time on purpose: it keeps the order and asks Suno for one song at a
    /// time. `project` is where the cards go.
    pub async fn create_cards_from_media(self, paths: Vec<PathBuf>, project: Option<String>) {
        if *self.draft_busy.peek() {
            return;
        }
        let mut busy = self.draft_busy;
        busy.set(true);
        let (mut made, mut gone) = (0, 0);
        let mut failure = None;
        for path in paths {
            let draft = match tokio::task::spawn_blocking(move || engine::drafts::from_media(&path)).await {
                Ok(Ok(d)) => d,
                Ok(Err(_)) => {
                    gone += 1;
                    continue;
                }
                Err(e) => {
                    failure = Some(e.to_string());
                    break;
                }
            };
            let st = self.st();
            let new = NewCard {
                id: new_id(),
                kind: "song".to_owned(),
                name: draft.name.clone(),
                style: draft.style.clone(),
                lyrics: draft.lyrics.clone(),
                sort_number: draft.sort_number,
                media_path: draft.media_path.clone(),
                date: Some(draft.date).filter(|d| *d != 0),
                env: Some(self.env.peek().clone()),
                ..Default::default()
            };
            match st.create_card(&new) {
                Ok(card) => {
                    for u in &draft.urls {
                        let _ = st.add_card_url(&card.id, &u.label, &u.href);
                    }
                    if let Some(p) = &project {
                        let _ = st.add_card_to_project(p, &card.id);
                    }
                    made += 1;
                }
                // whatever was created before the failure stays: the shortfall is reported, never hidden
                Err(e) => {
                    failure = Some(e);
                    break;
                }
            }
        }
        busy.set(false);
        self.reload();
        if let Some(e) = failure {
            self.fail(e);
        } else if gone > 0 {
            self.fail(self.t("tstAppDropMediaSongGone").replace("{n}", &gone.to_string()));
        } else if made > 0 {
            self.ok(format!("{made} {}", self.t("msgAppImportCardsCreated")));
        }
    }
}
