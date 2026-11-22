//! The cards of the main view, in the two layouts of the Electron app: the grid tile and the one-line list row. Both are built from
//! the same facts (`Facts`) and the same buttons, so what a card offers cannot differ between the layouts.

use crate::{
    dnd::{self, Dragged, DropAt},
    ui::{title_of, Act, Dialog, Nav, Ui},
};
use dioxus::prelude::*;
use dioxus_free_icons::{
    icons::ld_icons::{LdCheck, LdChevronDown, LdChevronRight, LdCopy, LdFile, LdFileMusic, LdFolderOpen, LdGlobe, LdLink2, LdMicVocal, LdMusic, LdMusic2, LdPause, LdPencil, LdPlay, LdSquare, LdSquareCheck, LdStar, LdTag, LdTrash2, LdTriangleAlert, LdX},
    Icon,
};
use engine::store::{Card, Url};
use std::path::Path;

/// What a card shows that has to be worked out: its linked cards, whether its files are there, which of its links are the three that
/// are drawn.
struct Facts {
    linked: Vec<Card>,
    /// How many songs a music / vocal card is linked from.
    linked_by: usize,
    media_ok: bool,
    lrc_ok: bool,
    lrc_installed: bool,
    lrc_shown: bool,
    has_style: bool,
    ai_url: Option<Url>,
    music_url: Option<Url>,
    other_url: Option<Url>,
}

/// Whether LRC Editor is installed, looked up on disk at most once every few seconds: every card asks, and a list holds hundreds.
fn lrc_editor_installed() -> bool {
    thread_local! {
        static SEEN: std::cell::Cell<Option<(std::time::Instant, bool)>> = const { std::cell::Cell::new(None) };
    }
    SEEN.with(|s| match s.get() {
        Some((at, found)) if at.elapsed().as_secs() < 5 => found,
        _ => {
            let found = engine::media::find_lrc_editor().is_some();
            s.set(Some((std::time::Instant::now(), found)));
            found
        }
    })
}

impl Facts {
    fn of(ui: Ui, card: &Card) -> Self {
        let mut linked: Vec<Card> = card.linked_cards.iter().filter_map(|id| ui.card(id)).collect();
        // the linked cards are listed notes first, then music, vocal, songs, each by name
        let rank = |k: &str| match k {
            "note" => 0,
            "music" => 1,
            "vocal" => 2,
            _ => 3,
        };
        linked.sort_by(|a, b| rank(&a.kind).cmp(&rank(&b.kind)).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
        let linked_by = if card.kind == "music" || card.kind == "vocal" { ui.cards.read().iter().filter(|p| p.linked_cards.contains(&card.id)).count() } else { 0 };
        let has_media = !card.media_path.is_empty();
        let (ai_list, music_list) = (ui.ai_sites(), ui.music_sites());
        let label = |u: &Url| u.label.trim().to_lowercase();
        let has_style = if card.kind == "song" { !card.style.trim().is_empty() || linked.iter().any(|l| !l.style.trim().is_empty()) } else { !card.style.trim().is_empty() };
        Facts {
            linked_by,
            media_ok: has_media && Path::new(&card.media_path).exists(),
            lrc_ok: has_media && engine::media::lrc_of(Path::new(&card.media_path)).exists(),
            lrc_installed: lrc_editor_installed(),
            lrc_shown: card.kind == "song" && has_media && !engine::exports::clean_lyrics(&card.lyrics).trim().is_empty(),
            has_style,
            ai_url: card.urls.iter().find(|u| ai_list.contains(&label(u))).cloned(),
            music_url: card.urls.iter().find(|u| music_list.contains(&label(u))).cloned(),
            other_url: card.urls.iter().find(|u| !ai_list.contains(&label(u)) && !music_list.contains(&label(u))).cloned(),
            linked,
        }
    }
}

pub(crate) fn kind_icon(kind: &str, size: u32) -> Element {
    match kind {
        "note" => rsx! { Icon { width: size, height: size, icon: LdFile } },
        "music" => rsx! { Icon { width: size, height: size, icon: LdMusic } },
        "vocal" => rsx! { Icon { width: size, height: size, icon: LdMicVocal } },
        _ => rsx! { Icon { width: size, height: size, icon: LdMusic2 } },
    }
}

// ─── The buttons both layouts share ─────────────────────────────────────────────────────────────

#[component]
fn PlayBtn(card: Card, media_ok: bool) -> Element {
    let ui = use_context::<Ui>();
    let playing = *ui.playing.read() == card.id && *ui.sounding.read();
    let title = if !media_ok { ui.t("tipCardMediaMissing") } else if playing { ui.t("tipCardPause") } else { ui.t("tipCardPlay") };
    let has_media = !card.media_path.is_empty();
    rsx! {
        button {
            class: if playing { "uis-btn ico small active" } else { "uis-btn ico small" },
            // pointer-events: a finalized song card is inert, but listening is not editing (Electron keeps the play button clickable)
            // ⚠ what comes and goes is its own attribute: dropped from a style string, it would stay on the button
            style: "pointer-events:auto",
            visibility: if !has_media { "hidden" },
            color: if has_media && !media_ok { "var(--uis-danger)" },
            border_color: if has_media && !media_ok { "var(--uis-danger)" },
            title: "{title}",
            disabled: !media_ok,
            onclick: move |_| ui.play_card(&card),
            if !media_ok {
                Icon { width: 14, height: 14, icon: LdTriangleAlert }
            } else if playing {
                Icon { width: 14, height: 14, icon: LdPause }
            } else {
                Icon { width: 14, height: 14, icon: LdPlay }
            }
        }
    }
}

#[component]
fn LrcBtn(card: Card, shown: bool, lrc_ok: bool, installed: bool) -> Element {
    let ui = use_context::<Ui>();
    let title = ui.t("tipCardOpenLrcEditor");
    rsx! {
        button {
            class: "uis-btn ico small",
            // ⚠ two attributes, each always or never there: a style string that swaps one for the other keeps both
            color: if lrc_ok { "var(--ap-accent)" } else { "var(--uis-text-mute)" },
            visibility: if !shown { "hidden" },
            title: "{title}",
            disabled: !installed,
            onclick: move |_| {
                let _ = engine::media::launch_lrc_editor(&card.media_path, &engine::exports::clean_lyrics(&card.lyrics));
            },
            Icon { width: 14, height: 14, icon: LdFileMusic }
        }
    }
}

#[component]
fn FavBtn(card: Card) -> Element {
    let ui = use_context::<Ui>();
    rsx! {
        button {
            class: "uis-btn ico small subtle",
            style: if card.favorite { "color:var(--uis-star)" } else { "color:var(--uis-text-mute)" },
            onclick: move |_| ui.toggle_fav(&card.id),
            // ⚠ the icon crate's `fill` prop is the STROKE colour (never the fill), so "none" there drew no star at all; the
            // filled star of the Electron app (`fill="var(--uis-star)"`) is a style, which wins over the svg's `fill="none"`.
            Icon { width: 14, height: 14, icon: LdStar, style: if card.favorite { "fill:var(--uis-star)" } else { "fill:none" } }
        }
    }
}

#[component]
fn SelectBtn(id: String) -> Element {
    let ui = use_context::<Ui>();
    let on = ui.selected.read().contains(&id);
    rsx! {
        button { class: "uis-btn ico small subtle", style: if on { "color:var(--uis-accent)" } else { "color:var(--uis-text-mute)" }, onclick: move |_| ui.toggle_select(&id),
            if on {
                Icon { width: 14, height: 14, icon: LdSquareCheck }
            } else {
                Icon { width: 14, height: 14, icon: LdSquare }
            }
        }
    }
}

/// One of the card's three link buttons: its colour says what kind of site it is.
#[component]
fn UrlBtn(url: Url, colour: String) -> Element {
    rsx! {
        button {
            class: "uis-btn ico small",
            style: "color:{colour};border-color:color-mix(in srgb, {colour} 40%, transparent)",
            title: "{url.label}: {url.href}",
            onclick: move |_| {
                let _ = open::that(&url.href);
            },
            span { class: "ap-url-l", "{url.label.chars().next().unwrap_or('?').to_uppercase()}" }
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Copy {
    Title,
    Style,
    Lyrics,
}

/// A copy button: the check mark and the green stay for a moment after the copy. `shown` false keeps the place and hides it.
#[component]
fn CopyBtn(card: Card, what: Copy, shown: bool, linked_style: bool) -> Element {
    let ui = use_context::<Ui>();
    let (mode, key, tip) = match what {
        Copy::Title => ("title", format!("{}_title", card.id), ui.t("tipCardCopyTitle")),
        Copy::Style if linked_style => ("linked", format!("{}_linked", card.id), ui.t("tipCardCopyStyle")),
        Copy::Style => ("", card.id.clone(), ui.t("tipCardCopyStyle")),
        Copy::Lyrics => ("lyrics", format!("{}_lyrics", card.id), ui.t("tipCardCopyLyrics")),
    };
    let done = (ui.copied)() == key;
    rsx! {
        button {
            class: if done { "uis-btn ico small ap-ok" } else { "uis-btn ico small" },
            style: if !shown { "visibility:hidden" },
            title: "{tip}",
            onclick: move |_| ui.copy(&card, mode),
            if done {
                Icon { width: 14, height: 14, icon: LdCheck }
            } else {
                Icon { width: 14, height: 14, icon: LdCopy }
            }
        }
    }
}

#[component]
fn UnlinkBtn(id: String) -> Element {
    let ui = use_context::<Ui>();
    let tip = ui.t("tipCardUnlink");
    rsx! {
        button {
            class: "uis-btn ico small ap-warnbtn",
            title: "{tip}",
            onclick: move |_| {
                if let Nav::Project(p) = ui.nav.peek().clone() {
                    let _ = ui.st().remove_card_from_project(&p, &id);
                    ui.reload();
                    ui.ok(ui.t("tstAppRemovedFromProject"));
                }
            },
            Icon { width: 14, height: 14, icon: LdX }
        }
    }
}

#[component]
fn DeleteBtn(id: String) -> Element {
    let ui = use_context::<Ui>();
    let mut dialog = ui.dialog;
    let tip = ui.t("tipCardDelete");
    rsx! {
        button {
            class: "uis-btn ico small ap-danger",
            title: "{tip}",
            onclick: move |_| dialog.set(Dialog::Confirm { title: ui.t("cfmDlgDeleteCardQ"), sub: ui.t("cfmDlgDeleteCardSub"), act: Act::DeleteCard(id.clone()) }),
            Icon { width: 14, height: 14, icon: LdTrash2 }
        }
    }
}

#[component]
fn EditBtn(id: String) -> Element {
    let ui = use_context::<Ui>();
    let tip = ui.t("tipCardEdit");
    rsx! {
        button { class: "uis-btn ico small", title: "{tip}", onclick: move |_| ui.open_card(&id), Icon { width: 14, height: 14, icon: LdPencil } }
    }
}

fn url_buttons(ui: Ui, f: &Facts) -> [Option<(Url, String)>; 3] {
    [
        f.ai_url.clone().map(|u| (u, ui.colour("SongStateCreated"))),
        f.music_url.clone().map(|u| (u, ui.colour("SongStatePublished"))),
        f.other_url.clone().map(|u| (u, "var(--uis-text-mute)".to_owned())),
    ]
}

/// The chips of the cards a song is linked to (or, on a music / vocal card, how many songs link to it).
fn linked_chips(ui: Ui, card: &Card, f: &Facts, compact: bool) -> Element {
    if card.kind == "song" && !f.linked.is_empty() {
        let size = if compact { 10 } else { 11 };
        return rsx! {
            div { class: if compact { "ap-linked-row" } else { "ap-linked" },
                if !compact {
                    Icon { width: 12, height: 12, icon: LdLink2 }
                }
                for l in f.linked.iter() {
                    span {
                        key: "{l.id}",
                        class: "ap-chip",
                        title: "{l.name} v{l.version}",
                        style: "color:{ui.colour_of(l)};border-color:{ui.colour_of(l)}55;background:{ui.colour_of(l)}18",
                        {kind_icon(&l.kind, size)}
                        span { class: "ap-chip-name", "{l.name}" }
                        b { class: "ap-chip-v", "v{l.version}" }
                    }
                }
            }
        };
    }
    if card.kind != "song" && card.kind != "note" && f.linked_by > 0 {
        let c = ui.colour("SongStateUndefined");
        return rsx! {
            div { class: if compact { "ap-linked-row" } else { "ap-linked" },
                Icon { width: 12, height: 12, icon: LdLink2 }
                span { class: "ap-chip", style: "color:{c};border-color:{c}55;background:{c}18", "{f.linked_by}" }
            }
        };
    }
    rsx! {}
}

/// The one button a song card keeps in the Sort view (the Electron `SongListCard` in its sort mode): the play button, hidden when the
/// song has no media.
#[component]
pub fn SongPlay(card: Card) -> Element {
    let ui = use_context::<Ui>();
    let f = Facts::of(ui, &card);
    rsx! {
        PlayBtn { card: card.clone(), media_ok: f.media_ok }
    }
}

// ─── The area ───────────────────────────────────────────────────────────────────────────────────

#[component]
pub fn CardArea() -> Element {
    let ui = use_context::<Ui>();
    let cards = ui.shown_cards();
    let list = (ui.list_view)();
    if cards.is_empty() {
        // Electron's empty state: an icon and a line that say WHY nothing is listed (every card has tags / is in a project / has its
        // LRC, a project with no cards, or filters that match nothing), and under it what to do about it
        let nav = ui.nav.read().clone();
        let warn = ui.warn.read().clone();
        let kind = ui.kind.read().clone();
        let in_project = matches!(nav, Nav::Project(_));
        let (untagged, unlinked, no_lrc) = (nav == Nav::Untagged || warn == "tagged", nav == Nav::Unsorted, warn == "lrc");
        let special = untagged || unlinked || no_lrc;
        let title = if untagged {
            ui.t("empGLViewAllHaveTags")
        } else if unlinked {
            ui.t("empGLViewAllInProject")
        } else if no_lrc {
            ui.t("empGLViewAllHaveLrc")
        } else if in_project {
            ui.t("empGLViewNoCardsProject")
        } else {
            ui.t("empGLViewNoCardsFound")
        };
        let sub = if special {
            String::new()
        } else if in_project {
            ui.t("empGLViewDragToAdd")
        } else {
            ui.t("empGLViewAdjustFilters")
        };
        return rsx! {
            div { class: "uis-emp",
                span { style: "display: inline-flex; color: var(--uis-text-mute)",
                    if untagged {
                        Icon { icon: LdTag }
                    } else if unlinked {
                        Icon { icon: LdStar }
                    } else if no_lrc {
                        Icon { icon: LdFileMusic }
                    } else if in_project {
                        Icon { icon: LdFolderOpen }
                    } else if !kind.is_empty() {
                        {kind_icon(&kind, 24)}
                    } else {
                        Icon { icon: LdMusic2 }
                    }
                }
                p { class: "uis-emp-title", "{title}" }
                p { class: "uis-emp-sub", "{sub}" }
            }
        };
    }
    rsx! {
        div { class: if list { "ap-grid ap-list" } else { "ap-grid" },
            for c in cards {
                if list {
                    CardRow { key: "{c.id}", card: c.clone() }
                } else {
                    CardTile { key: "{c.id}", card: c.clone() }
                }
            }
        }
    }
}

/// The card being carried is half transparent, as in the Electron app (`opacity: isDragging ? 0.5 : 1`).
fn dragged_class(ui: Ui, id: &str) -> &'static str {
    match &ui.drag.read().live {
        Some((Dragged::Cards(ids), _)) if ids.iter().any(|x| x == id) => " ap-card-dragged",
        _ => "",
    }
}

// ─── The grid tile ──────────────────────────────────────────────────────────────────────────────

#[component]
fn CardTile(card: Card) -> Element {
    let ui = use_context::<Ui>();
    let accent = ui.colour_of(&card);
    let selected = ui.selected.read().contains(&card.id);
    let is_note = card.kind == "note";
    let is_song = card.kind == "song";
    let in_project = matches!(*ui.nav.read(), Nav::Project(_));
    let f = Facts::of(ui, &card);
    let text = if is_note { card.desc.clone() } else if !card.style.is_empty() { card.style.clone() } else { card.lyrics.clone() };
    let preview: String = if text.chars().count() > 160 { text.chars().take(160).collect::<String>() + "…" } else { text.clone() };
    let global = !is_song && card.env == ui.global_env();
    let mut expanded = use_signal(|| false);
    let title = title_of(&card);
    let linked_style = is_song && !f.linked.is_empty();
    let chips = linked_chips(ui, &card, &f, false);
    let urls = url_buttons(ui, &f);
    let at = DropAt::Song(card.id.clone());
    let (at_a, at_b) = (at.clone(), at.clone());
    let (drag_id, drag_label) = (card.id.clone(), title.clone());
    let over = ui.is_over(&at);
    let dragged_class = dragged_class(ui, &card.id);
    let base_class = if over { "ap-card ap-drop" } else if selected { "ap-card ap-card-on" } else { "ap-card" };
    rsx! {
        div {
            class: "{base_class}{dragged_class}",
            style: "--ap-accent:{accent}",
            onmousedown: move |e| ui.arm_drag(Dragged::Cards(dnd::cards_of(ui, &drag_id)), drag_label.clone(), &e),
            onmouseenter: move |_| ui.drag_over(at_a.clone()),
            onmouseleave: move |_| ui.drag_leave(&at_b),
            div { class: "ap-card-top",
                SelectBtn { id: card.id.clone() }
                div { class: "ap-card-main",
                    h3 { class: "ap-card-title",
                        "{title}"
                        span { class: "ap-badge", "v{card.version}" }
                        if !card.lang.is_empty() {
                            span { class: "ap-badge ap-badge-mute", "{card.lang}" }
                        }
                        if global {
                            span { class: "ap-badge ap-badge-mute", title: "{ui.t(\"lblCardGlobalResource\")}", Icon { width: 10, height: 10, icon: LdGlobe } }
                        }
                    }
                    p { class: if expanded() { "ap-card-text" } else { "ap-card-text ap-clamp" },
                        if expanded() { "{text}" } else { "{preview}" }
                    }
                    if text.chars().count() > 160 {
                        button { class: "uis-btn ico small subtle", onclick: move |_| expanded.toggle(),
                            if expanded() {
                                Icon { width: 12, height: 12, icon: LdChevronDown }
                            } else {
                                Icon { width: 12, height: 12, icon: LdChevronRight }
                            }
                        }
                    }
                    if !card.tags.is_empty() {
                        div { class: "ap-tags",
                            for tag in card.tags.iter() {
                                span { key: "{tag}", class: "ap-tag", style: "color:{ui.tag_colour(tag)};border-color:{ui.tag_colour(tag)}55", "{tag}" }
                            }
                        }
                    }
                    {chips}
                }
                div { class: "ap-card-side",
                    FavBtn { card: card.clone() }
                    for (u , c) in urls.into_iter().flatten() {
                        UrlBtn { key: "{u.id}", url: u, colour: c }
                    }
                }
            }
            div { class: "ap-card-foot",
                div { class: "ap-card-acts",
                    EditBtn { id: card.id.clone() }
                    PlayBtn { card: card.clone(), media_ok: f.media_ok }
                    LrcBtn { card: card.clone(), shown: f.lrc_shown, lrc_ok: f.lrc_ok, installed: f.lrc_installed }
                }
                div { class: "ap-card-acts",
                    CopyBtn { card: card.clone(), what: Copy::Title, shown: true, linked_style }
                    if !is_note {
                        CopyBtn { card: card.clone(), what: Copy::Style, shown: f.has_style, linked_style }
                        CopyBtn { card: card.clone(), what: Copy::Lyrics, shown: is_song && !card.lyrics.is_empty(), linked_style }
                    }
                    if in_project {
                        UnlinkBtn { id: card.id.clone() }
                    }
                    DeleteBtn { id: card.id.clone() }
                }
            }
        }
    }
}

// ─── The list row ───────────────────────────────────────────────────────────────────────────────

/// One line: select, edit, play, LRC editor, favourite, the title and its version and language; under it the text, up to three tags and
/// the linked cards. On the right two rows of five slots, so the buttons line up from one row to the next.
#[component]
fn CardRow(card: Card) -> Element {
    let ui = use_context::<Ui>();
    let accent = ui.colour_of(&card);
    let selected = ui.selected.read().contains(&card.id);
    let is_note = card.kind == "note";
    let is_song = card.kind == "song";
    let in_project = matches!(*ui.nav.read(), Nav::Project(_));
    let f = Facts::of(ui, &card);
    let text = if is_note { card.desc.clone() } else if !card.style.is_empty() { card.style.clone() } else { card.lyrics.clone() };
    let global = !is_song && card.env == ui.global_env();
    let title = title_of(&card);
    let linked_style = is_song && !f.linked.is_empty();
    let chips = linked_chips(ui, &card, &f, true);
    let [ai, music, other] = url_buttons(ui, &f);
    let more = card.tags.len().saturating_sub(3);
    let global_tip = ui.t("lblCardGlobalResource");
    let at = DropAt::Song(card.id.clone());
    let (at_a, at_b) = (at.clone(), at.clone());
    let (drag_id, drag_label) = (card.id.clone(), title.clone());
    let over = ui.is_over(&at);
    let dragged_class = dragged_class(ui, &card.id);
    let base_class = if over { "ap-card ap-rowcard ap-drop" } else if selected { "ap-card ap-rowcard ap-card-on" } else { "ap-card ap-rowcard" };
    rsx! {
        div {
            class: "{base_class}{dragged_class}",
            style: "--ap-accent:{accent}",
            onmousedown: move |e| ui.arm_drag(Dragged::Cards(dnd::cards_of(ui, &drag_id)), drag_label.clone(), &e),
            onmouseenter: move |_| ui.drag_over(at_a.clone()),
            onmouseleave: move |_| ui.drag_leave(&at_b),
            SelectBtn { id: card.id.clone() }
            div { class: "ap-rowcard-main",
                div { class: "ap-rowcard-top",
                    EditBtn { id: card.id.clone() }
                    PlayBtn { card: card.clone(), media_ok: f.media_ok }
                    LrcBtn { card: card.clone(), shown: f.lrc_shown, lrc_ok: f.lrc_ok, installed: f.lrc_installed }
                    FavBtn { card: card.clone() }
                    h3 { class: "ap-rowcard-title",
                        span { class: "ap-rowcard-name", "{title}" }
                        span { class: "ap-badge", "v{card.version}" }
                    }
                    if !card.lang.is_empty() {
                        span { class: "ap-badge ap-badge-mute", "{card.lang}" }
                    }
                }
                div { class: "ap-rowcard-second",
                    if !text.is_empty() {
                        p { class: "ap-rowcard-text", "{text}" }
                    } else {
                        div { class: "ap-grow" }
                    }
                    if !card.tags.is_empty() {
                        div { class: "ap-tags ap-tags-row",
                            for tag in card.tags.iter().take(3) {
                                span { key: "{tag}", class: "ap-tag", style: "color:{ui.tag_colour(tag)};border-color:{ui.tag_colour(tag)}55", "{tag}" }
                            }
                            if more > 0 {
                                span { class: "uis-hnt", "+{more}" }
                            }
                        }
                    }
                    {chips}
                }
            }
            div { class: "ap-rowcard-slots",
                CopyBtn { card: card.clone(), what: Copy::Title, shown: true, linked_style }
                if !is_note {
                    CopyBtn { card: card.clone(), what: Copy::Style, shown: f.has_style, linked_style }
                    CopyBtn { card: card.clone(), what: Copy::Lyrics, shown: is_song && !card.lyrics.is_empty(), linked_style }
                } else {
                    span { class: "ap-slot-empty" }
                    span { class: "ap-slot-empty" }
                }
                if in_project {
                    UnlinkBtn { id: card.id.clone() }
                } else {
                    span { class: "ap-slot-empty" }
                }
                DeleteBtn { id: card.id.clone() }
                if global {
                    button { class: "uis-btn ico small", style: "cursor:default;color:var(--uis-accent)", title: "{global_tip}", Icon { width: 14, height: 14, icon: LdGlobe } }
                } else {
                    span { class: "ap-slot-empty" }
                }
                for slot in [ai, music, other] {
                    if let Some((u , c)) = slot {
                        UrlBtn { url: u, colour: c }
                    } else {
                        span { class: "ap-slot-empty" }
                    }
                }
                span { class: "ap-slot-empty" }
            }
        }
    }
}
