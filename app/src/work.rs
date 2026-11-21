// ⚠ NO HARDCODED UI STRINGS IN THIS FILE.
//   Every string a user can read MUST go through the t() lookup, and the key MUST be a string literal at the call site.

//! The three working views of the songs: put them in order, link the music and vocal cards to them, link the media files to them.
//! Done with drags (`dnd.rs`): a song is dragged to its place, music and vocal cards to a song, a media file to a song.
//!
//! The cards are the Electron ones (`SongListCard` in its sort / link / media modes, `UilLinkViewMVCard`, `MediaLinkFileCard`), with
//! their inline values copied: nothing here is an app class that stands in for them.

use crate::{
    card_view::{SongPlay, kind_icon},
    catalog::UisPnlSplitter,
    dnd::{Dragged, DropAt},
    ui::{Nav, Ui, title_of},
    views::drag_width,
};
use dioxus::prelude::*;
use dioxus_free_icons::{
    Icon,
    icons::ld_icons::{LdArrowUpDown, LdCircleDashed, LdDisc, LdFolder, LdFolderOpen, LdGlobe, LdLink2, LdMusic2, LdPencil, LdSquare, LdSquareCheck, LdWandSparkles, LdX},
};
use engine::{cards::CardPatch, media, projects::ProjectPatch, store::Card};
use std::path::Path;

pub fn songs_of(ui: Ui) -> Vec<Card> {
    let mut songs: Vec<Card> = ui.shown_cards().into_iter().filter(|c| c.kind == "song").collect();
    songs.sort_by(|a, b| a.sort_number.unwrap_or(i64::MAX).cmp(&b.sort_number.unwrap_or(i64::MAX)).then(a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    songs
}

/// A project that is finalized keeps its order, its links and its media.
pub fn finalized(ui: Ui) -> bool {
    match ui.nav.read().clone() {
        Nav::Project(id) => ui.projects.read().iter().any(|p| p.id == id && p.finalized),
        _ => false,
    }
}

/// The strip at the top of the content area in the three working views (the catalog's `.uis-barh-view`, as in the Electron app): the
/// toggle that finalizes the project (checked = its order, links and media are locked, unchecked = the views are live again), and,
/// in the Sort view, the toggle that renames each linked media file's number prefix on a reorder.
#[component]
pub fn WorkStrip(sort: bool) -> Element {
    let ui = use_context::<Ui>();
    let Nav::Project(id) = ui.nav.read().clone() else {
        return rsx! {};
    };
    let locked = finalized(ui);
    let mut rename = ui.sort_rename;
    let (lbl_fin, tip_fin, lbl_rename, tip_rename) = (ui.t("lblViewFinalized"), ui.t("tipDlgProjectFinalized"), ui.t("lblViewRenameMedia"), ui.t("tipViewRenameMedia"));
    rsx! {
        div { class: "uis-barh-view",
            button {
                class: "uis-btn small subtle",
                r#type: "button",
                title: "{tip_fin}",
                style: "color: var(--uis-text)",
                onclick: move |_| {
                    if let Err(e) = ui.st().update_project(&id, &ProjectPatch { finalized: Some(!locked), ..Default::default() }) {
                        ui.fail(e);
                    }
                    ui.reload();
                },
                if locked {
                    Icon { width: 14, height: 14, icon: LdSquareCheck, style: "color: var(--uis-accent)" }
                } else {
                    Icon { width: 14, height: 14, icon: LdSquare, style: "color: var(--uis-text-mute)" }
                }
                span { "{lbl_fin}" }
            }
            if sort {
                button {
                    class: "uis-btn small subtle",
                    r#type: "button",
                    title: "{tip_rename}",
                    style: "color: var(--uis-text)",
                    onclick: move |_| rename.toggle(),
                    if rename() {
                        Icon { width: 14, height: 14, icon: LdSquareCheck, style: "color: var(--uis-accent)" }
                    } else {
                        Icon { width: 14, height: 14, icon: LdSquare, style: "color: var(--uis-text-mute)" }
                    }
                    span { "{lbl_rename}" }
                }
            }
        }
    }
}

/// The width of the songs panel in the Link and Media views: remembered, 320 the first time (as in the Electron app).
fn use_link_width(ui: Ui) -> Signal<f64> {
    let width = use_signal(|| ui.st().setting("link-panel-width").parse::<f64>().unwrap_or(320.0).clamp(200.0, 700.0));
    use_effect(move || ui.st().set_setting("link-panel-width", &(width() as i64).to_string()));
    width
}

// The values below are Electron's inline ones (App.jsx, the three views).
const HEAD: &str = "padding: 10px 16px; border-bottom: 1px solid var(--uis-border); display: flex; align-items: center; gap: 8px; flex-shrink: 0;";
const HEAD_TXT: &str = "font-size: 12px; font-weight: 600; color: var(--uis-text-mute)";
const LEFT_LIST: &str = "flex: 1; overflow-y: auto; padding: 12px 14px; display: flex; flex-direction: column; gap: 8px;";

/// The room the list keeps under its last card: more while the footer player is up.
fn list_bottom(ui: Ui) -> u32 {
    if ui.playing.read().is_empty() { 60 } else { 90 }
}

/// A song's chip colour: the colour at 13 % (Electron's `accentDim`).
fn dim(c: &str) -> String {
    format!("color-mix(in srgb, {c} 13%, transparent)")
}

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Sort,
    Link,
    Media,
}

/// The song card of the three views (Electron `SongListCard`, `mode` sort / link / media): the title with its version, and under it what
/// the view is about: the cards linked to the song (Link), its media file (Media); the Sort view keeps the play button only.
#[component]
fn SongCard(card: Card, mode: Mode, locked: bool, last: bool) -> Element {
    let ui = use_context::<Ui>();
    let accent = ui.colour_of(&card);
    let title = title_of(&card);
    let live = ui.drag.read().live.clone();
    let any_dragging = match (&live, mode) {
        (Some((Dragged::Cards(_), _)), Mode::Link) | (Some((Dragged::Media(_), _)), Mode::Media) | (Some(_), Mode::Sort) => true,
        _ => false,
    };
    let carried = matches!(&live, Some((Dragged::Cards(ids), _)) if ids.iter().any(|x| *x == card.id));
    let over = mode != Mode::Sort && any_dragging && ui.is_over(&DropAt::Song(card.id.clone()));
    let mark = match (mode, ui.drag.read().over.clone()) {
        (Mode::Sort, Some(DropAt::Sort { id, after })) if id == card.id => Some(after),
        _ => None,
    };
    let mark_side = mark.map(|after| if after { "bottom: -5px;" } else { "top: -5px;" });
    let mut height = use_signal(|| 56.0f64);
    let (id_down, id_move, id_leave, id_in, id_out) = (card.id.clone(), card.id.clone(), card.id.clone(), card.id.clone(), card.id.clone());
    let label = title.clone();
    let mut linked: Vec<Card> = card.linked_cards.iter().filter_map(|l| ui.card(l)).collect();
    let rank = |k: &str| match k {
        "note" => 0,
        "music" => 1,
        "vocal" => 2,
        _ => 3,
    };
    linked.sort_by(|a, b| rank(&a.kind).cmp(&rank(&b.kind)).then(a.name.cmp(&b.name)));
    let file = if card.media_path.is_empty() { None } else { Path::new(&card.media_path).file_name().map(|n| n.to_string_lossy().into_owned()) };
    let (rel, over_style, lock_style) = (
        if mode == Mode::Sort { "position: relative;" } else { "" },
        if over { format!("border-color: {accent}; box-shadow: 0 0 0 2px {accent}33;") } else { String::new() },
        if locked { "pointer-events: none; opacity: 0.45;" } else { "" },
    );
    let dashed_border = if any_dragging { format!("{accent}88") } else { "var(--uis-border)".to_owned() };
    let dashed_colour = if any_dragging { accent.clone() } else { "var(--uis-text-mute)".to_owned() };
    let (tip_unlink, tip_remove) = (ui.t("tipPnlLinkUnlink"), ui.t("tipPnlLinkRemoveMedia"));
    let dashed_text = match (mode, any_dragging, linked.is_empty(), file.is_some()) {
        (Mode::Link, true, _, _) => ui.t("lblPnlLinkDrop"),
        (Mode::Link, false, _, _) => ui.t("lblPnlLinkNoCards"),
        (_, true, _, _) => ui.t("lblPnlLinkDropMedia"),
        _ => ui.t("lblPnlLinkNoMedia"),
    };
    let dragged_class = if carried { "ap-card ap-rowcard ap-card-dragged" } else { "ap-card ap-rowcard" };
    let song_id = card.id.clone();
    let media_path = card.media_path.clone();
    rsx! {
        div {
            key: "{over}-{locked}",
            class: "{dragged_class}",
            style: "--ap-accent:{accent};{rel}{over_style}{lock_style}",
            onmounted: move |e| {
                spawn(async move {
                    if let Ok(r) = e.get_client_rect().await {
                        height.set(r.size.height);
                    }
                });
            },
            onmousedown: move |e| {
                if mode == Mode::Sort && !locked {
                    ui.arm_drag(Dragged::Cards(vec![id_down.clone()]), label.clone(), &e);
                }
            },
            // Sort: one indicator per gap, always "before" this card, except the last card's lower half, which is "after"
            onmousemove: move |e| {
                if mode == Mode::Sort {
                    ui.drag_over(DropAt::Sort { id: id_move.clone(), after: last && e.element_coordinates().y >= height() / 2.0 });
                }
            },
            onmouseenter: move |_| {
                if mode != Mode::Sort {
                    ui.drag_over(DropAt::Song(id_in.clone()));
                }
            },
            onmouseleave: move |_| {
                if mode == Mode::Sort {
                    ui.drag_leave(&DropAt::Sort { id: id_leave.clone(), after: false });
                } else {
                    ui.drag_leave(&DropAt::Song(id_out.clone()));
                }
            },
            if let Some(side) = mark_side {
                div { style: "position: absolute; left: 0; right: 0; height: 3px; border-radius: 2px; background: var(--uis-accent); pointer-events: none; z-index: 2; {side}" }
            }
            div { class: "ap-rowcard-main",
                div { class: "ap-rowcard-top",
                    if mode == Mode::Sort {
                        SongPlay { card: card.clone() }
                    }
                    h3 { class: "ap-rowcard-title",
                        span { class: "ap-rowcard-name", "{title}" }
                        span { style: "font-size: 9px; font-weight: 700; color: {accent}; border: 1px solid {accent}55; border-radius: 4px; padding: 1px 5px; opacity: 0.85; flex-shrink: 0;", "v{card.version}" }
                    }
                }
                if mode == Mode::Link {
                    div { class: "ap-rowcard-second",
                        if linked.is_empty() {
                            div {
                                key: "dash-{any_dragging}",
                                style: "border: 1px dashed {dashed_border}; border-radius: 6px; padding: 6px 12px; text-align: center; font-size: 11px; color: {dashed_colour}; transition: all 0.2s; flex: 1; min-width: 0;",
                                "{dashed_text}"
                            }
                        } else {
                            div { style: "display: flex; flex-wrap: wrap; gap: 5px; flex: 1; min-width: 0;",
                                for l in linked.iter() {
                                    {
                                        let c = ui.kind_colour(&l.kind);
                                        let (sid, lid) = (song_id.clone(), l.id.clone());
                                        rsx! {
                                            span {
                                                key: "{l.id}",
                                                style: "font-size: 11px; border: 1px solid {c}44; border-radius: 4px; padding: 3px 6px 3px 8px; color: {c}; background: {dim(&c)}; display: inline-flex; align-items: center; gap: 4px;",
                                                span { style: "display: inline-flex", {kind_icon(&l.kind, 11)} }
                                                "{l.name}"
                                                span { style: "opacity: 0.65; font-weight: 700;", "v{l.version}" }
                                                button {
                                                    class: "uis-btn ico small subtle",
                                                    title: "{tip_unlink}",
                                                    style: "color: {c}99",
                                                    onclick: move |_| {
                                                        let Some(song) = ui.card(&sid) else { return };
                                                        let links: Vec<String> = song.linked_cards.iter().filter(|x| **x != lid).cloned().collect();
                                                        if let Err(e) = ui.st().update_card(&sid, &CardPatch { linked_cards: Some(links), ..Default::default() }) {
                                                            ui.fail(e);
                                                        }
                                                        ui.reload();
                                                    },
                                                    Icon { width: 14, height: 14, icon: LdX }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                if mode == Mode::Media {
                    div { class: "ap-rowcard-second",
                        if let Some(f) = file {
                            div { style: "display: flex; align-items: center; gap: 6px; border: 1px solid {accent}44; border-radius: 6px; padding: 4px 10px; background: {dim(&accent)}; flex: 1; min-width: 0;",
                                span { style: "display: inline-flex; flex-shrink: 0; color: {accent}", Icon { width: 14, height: 14, icon: LdDisc } }
                                span { style: "font-size: 11px; color: {accent}; flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;", title: "{media_path}", "{f}" }
                                button {
                                    class: "uis-btn ico small subtle",
                                    title: "{tip_remove}",
                                    style: "color: {accent}99",
                                    onclick: move |_| {
                                        let _ = ui.st().update_card(&song_id, &CardPatch { media_path: Some(String::new()), ..Default::default() });
                                        ui.reload();
                                    },
                                    Icon { width: 14, height: 14, icon: LdX }
                                }
                            }
                        } else {
                            div {
                                key: "dash-{any_dragging}",
                                style: "border: 1px dashed {dashed_border}; border-radius: 6px; padding: 6px 12px; text-align: center; font-size: 11px; color: {dashed_colour}; transition: all 0.2s; flex: 1; min-width: 0;",
                                "{dashed_text}"
                            }
                        }
                    }
                }
            }
        }
    }
}

/// A music or vocal card of the Link view's right panel (Electron `UilLinkViewMVCard`): dragged onto a song to link it.
#[component]
fn MvCard(card: Card, locked: bool) -> Element {
    let ui = use_context::<Ui>();
    let accent = ui.kind_colour(&card.kind);
    let carried = matches!(&ui.drag.read().live, Some((Dragged::Cards(ids), _)) if ids.iter().any(|x| *x == card.id));
    let by_count = ui.cards.read().iter().filter(|s| s.linked_cards.contains(&card.id)).count();
    let preview: String = if card.style.chars().count() > 100 { card.style.chars().take(100).collect::<String>() + "…" } else { card.style.clone() };
    let (drag_id, drag_label, edit_id) = (card.id.clone(), card.name.clone(), card.id.clone());
    let opacity = if locked || carried { "0.45" } else { "1" };
    let events = if locked { "none" } else { "auto" };
    let song_c = ui.colour("SongStateUndefined");
    let tip_edit = ui.t("tipCardEdit");
    rsx! {
        div {
            style: "background: var(--uis-bg-elev); border: 1px solid var(--uis-border); border-radius: 10px; padding: 12px 14px; display: flex; flex-direction: column; gap: 6px; cursor: grab; opacity: {opacity}; transition: opacity 0.15s; user-select: none; pointer-events: {events};",
            onmousedown: move |e| {
                if !locked {
                    ui.arm_drag(Dragged::Cards(vec![drag_id.clone()]), drag_label.clone(), &e);
                }
            },
            div { style: "display: flex; align-items: center; gap: 6px;",
                span { style: "display: inline-flex; color: {accent}", {kind_icon(&card.kind, 14)} }
                span { style: "font-size: 13px; font-weight: 700; color: {accent}; flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;", "{card.name}" }
                span { style: "font-size: 10px; color: {accent}; border: 1px solid {accent}44; border-radius: 4px; padding: 1px 5px; opacity: 0.8; flex-shrink: 0;", "v{card.version}" }
                button { class: "uis-btn ico small subtle", title: "{tip_edit}", onclick: move |_| ui.open_card(&edit_id), Icon { width: 14, height: 14, icon: LdPencil } }
            }
            if !preview.is_empty() {
                p { style: "margin: 0; font-size: 11px; color: var(--uis-text-dim); line-height: 1.5; overflow: hidden; display: -webkit-box; -webkit-line-clamp: 3; -webkit-box-orient: vertical;", "{preview}" }
            }
            if by_count > 0 {
                div { style: "display: flex; gap: 6px; border-top: 1px solid var(--uis-border); padding-top: 6px; align-items: center;",
                    span { style: "display: inline-flex; color: var(--uis-text-mute)", Icon { width: 14, height: 14, icon: LdLink2 } }
                    span { style: "font-size: 10px; border: 1px solid {song_c}44; border-radius: 4px; padding: 2px 6px; color: {song_c}; background: {dim(&song_c)}; display: inline-flex; align-items: center; gap: 3px;", "{by_count}" }
                }
            }
        }
    }
}

/// A media file of the Media view's right panel (Electron `MediaLinkFileCard`): dragged onto a song to link it.
#[component]
fn MediaFileCard(path: String, owner: Option<Card>, locked: bool) -> Element {
    let ui = use_context::<Ui>();
    let name = Path::new(&path).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let carried = matches!(&ui.drag.read().live, Some((Dragged::Media(p), _)) if *p == path);
    let opacity = if locked || carried { "0.45" } else { "1" };
    let events = if locked { "none" } else { "auto" };
    let (drag_path, drag_name) = (path.clone(), name.clone());
    rsx! {
        div {
            style: "background: var(--uis-bg-elev); border: 1px solid var(--uis-border); border-radius: 8px; padding: 8px 12px; display: flex; align-items: center; gap: 8px; cursor: grab; opacity: {opacity}; transition: opacity 0.15s; user-select: none; pointer-events: {events};",
            onmousedown: move |e| {
                if !locked {
                    ui.arm_drag(Dragged::Media(drag_path.clone()), drag_name.clone(), &e);
                }
            },
            span { style: "display: inline-flex; flex-shrink: 0; color: var(--uis-text-mute)", Icon { width: 14, height: 14, icon: LdDisc } }
            span { style: "font-size: 12px; color: var(--uis-text); flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;", title: "{path}", "{name}" }
            if let Some(o) = owner {
                {
                    let c = ui.colour_of(&o);
                    let text = match o.sort_number {
                        Some(n) => format!("{n:02}-{}", o.name),
                        None => o.name.clone(),
                    };
                    rsx! {
                        span {
                            style: "font-size: 10px; border: 1px solid {c}44; border-radius: 4px; padding: 2px 6px; color: {c}; background: {dim(&c)}; flex-shrink: 0; max-width: 140px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;",
                            title: "{o.name}",
                            "{text}"
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn SortView() -> Element {
    let ui = use_context::<Ui>();
    let songs = songs_of(ui);
    let locked = finalized(ui);
    // a song carried past what is on screen: the list scrolls while the pointer is near its top or bottom edge
    use_effect(|| {
        document::eval(
            "const el = document.getElementById('ap-sort-list'); if (el && !el.dataset.scroll) { el.dataset.scroll = '1'; el.addEventListener('mousemove', e => { if (!document.querySelector('.ap-dragging')) return; const r = el.getBoundingClientRect(), z = 80; if (e.clientY < r.top + z) el.scrollTop -= Math.ceil((r.top + z - e.clientY) / z * 20); else if (e.clientY > r.bottom - z) el.scrollTop += Math.ceil((e.clientY - (r.bottom - z)) / z * 20); }); }",
        );
    });
    let (lbl, empty) = (ui.t("lblPnlSortSongs"), ui.t("empPnlSortNoSongs"));
    let n = songs.len();
    let bottom = list_bottom(ui);
    rsx! {
        div { class: "ap-work",
            div { style: "{HEAD}",
                span { style: "display: inline-flex; color: var(--uis-text-mute)", Icon { width: 14, height: 14, icon: LdArrowUpDown } }
                span { style: "{HEAD_TXT}", "{lbl} ({n})" }
            }
            div { id: "ap-sort-list", style: "flex: 1; overflow-y: auto; padding: 12px 14px; padding-bottom: {bottom}px; display: flex; flex-direction: column; gap: 8px;",
                if songs.is_empty() {
                    div { class: "uis-emp",
                        span { style: "display: inline-flex; color: var(--uis-text-mute)", Icon { icon: LdMusic2 } }
                        p { class: "uis-emp-title", "{empty}" }
                    }
                }
                for (i , s) in songs.iter().enumerate() {
                    SongCard { key: "{s.id}", card: s.clone(), mode: Mode::Sort, locked, last: i + 1 == n }
                }
            }
        }
    }
}

/// Put `dragged` before (or after) `target` in the order of the songs; the whole sequence is renumbered.
pub fn reorder_song(ui: Ui, dragged: &str, target: &str, after: bool) {
    let mut ids: Vec<String> = songs_of(ui).into_iter().map(|s| s.id).collect();
    ids.retain(|id| id != dragged);
    let Some(mut at) = ids.iter().position(|id| id == target) else { return };
    if after {
        at += 1;
    }
    ids.insert(at, dragged.to_owned());
    match ui.st().reorder_cards(&ids, *ui.sort_rename.peek()) {
        Ok(_) => ui.reload(),
        Err(e) => ui.fail(e),
    }
}

/// The left panel of the Link and Media views: the songs, as drop targets.
#[component]
fn SongsPanel(mode: Mode, width: f64) -> Element {
    let ui = use_context::<Ui>();
    let songs = songs_of(ui);
    let locked = finalized(ui);
    let (lbl, empty) = (ui.t("lblPnlLinkSongs"), ui.t("empPnlLinkNoSongs"));
    let n = songs.len();
    let c = ui.colour("SongStateCreated");
    let bottom = list_bottom(ui);
    rsx! {
        div { style: "width: {width}px; min-width: {width}px; display: flex; flex-direction: column; overflow: hidden;",
            div { style: "{HEAD}",
                span { style: "display: inline-flex; color: {c}", Icon { width: 14, height: 14, icon: LdMusic2 } }
                span { style: "{HEAD_TXT}", "{lbl} ({n})" }
            }
            div { style: "{LEFT_LIST} padding-bottom: {bottom}px;",
                if songs.is_empty() {
                    div { class: "uis-emp",
                        span { style: "display: inline-flex; color: var(--uis-text-mute)", Icon { icon: LdMusic2 } }
                        p { class: "uis-emp-title", "{empty}" }
                    }
                }
                for s in songs {
                    SongCard { key: "{s.id}", card: s.clone(), mode, locked, last: false }
                }
            }
        }
    }
}

#[component]
pub fn LinkView() -> Element {
    let ui = use_context::<Ui>();
    let locked = finalized(ui);
    let mut scope_global = use_signal(|| false);
    let mut query = use_signal(String::new);
    let width = use_link_width(ui);
    let (lbl_mv, empty_mv, lbl_global, lbl_current, plh_search) =
        (ui.t("lblPnlLinkMusicVocal"), ui.t("empPnlLinkNoMusicVocal"), ui.t("lblPnlLinkGlobal"), ui.t("lblPnlLinkCurrentProject"), ui.t("plhHdrSearchCards"));
    let global = ui.global_env();
    let has_global = !global.is_empty();
    // the cards to link: those of this environment, or (with the switch) those of the global one; the search narrows by name
    let needle = query().trim().to_lowercase();
    let mut mv: Vec<Card> = if scope_global() && has_global {
        ui.cards.read().iter().filter(|c| c.env == global && (c.kind == "music" || c.kind == "vocal")).cloned().collect()
    } else {
        ui.env_cards().into_iter().filter(|c| c.kind == "music" || c.kind == "vocal").collect()
    };
    mv.retain(|c| needle.is_empty() || c.name.to_lowercase().contains(&needle));
    mv.sort_by(|a, b| a.kind.cmp(&b.kind).then(a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    let mv_total = mv.len();
    let bottom = list_bottom(ui);
    rsx! {
        div { class: "ap-work ap-split",
            SongsPanel { mode: Mode::Link, width: width() }
            UisPnlSplitter { on_resize_start: move |e: MouseEvent| drag_width(width, e.client_coordinates().x, 200.0, 700.0) }
            div { style: "flex: 1; min-width: 0; display: flex; flex-direction: column; overflow: hidden;",
                div { style: "padding: 10px 16px 8px; border-bottom: 1px solid var(--uis-border); display: flex; flex-direction: column; gap: 6px; flex-shrink: 0;",
                    div { style: "display: flex; align-items: center; gap: 8px;",
                        span { style: "display: inline-flex; color: var(--uis-text-mute)", Icon { width: 14, height: 14, icon: LdLink2 } }
                        span { style: "{HEAD_TXT}; flex: 1;", "{lbl_mv} ({mv_total})" }
                        if has_global {
                            div { class: "uis-opt-set",
                                button {
                                    class: if !scope_global() { "uis-opt active" } else { "uis-opt" },
                                    title: "{lbl_current}",
                                    onclick: move |_| scope_global.set(false),
                                    Icon { width: 14, height: 14, icon: LdFolder }
                                    "{lbl_current}"
                                }
                                button {
                                    class: if scope_global() { "uis-opt active" } else { "uis-opt" },
                                    title: "{lbl_global}",
                                    onclick: move |_| scope_global.set(true),
                                    Icon { width: 14, height: 14, icon: LdGlobe }
                                    "{lbl_global}"
                                }
                            }
                        }
                    }
                    input { class: "uis-txt", style: "width: 100%;", placeholder: "{plh_search}", value: "{query}", oninput: move |e| query.set(e.value()) }
                }
                div { style: "flex: 1; overflow-y: auto; padding: 12px; padding-bottom: {bottom}px; display: grid; grid-template-columns: repeat(auto-fill, minmax(220px, 1fr)); gap: 10px; align-content: start;",
                    if mv.is_empty() {
                        div { class: "uis-emp",
                            span { style: "display: inline-flex; color: var(--uis-text-mute)", Icon { icon: LdMusic2 } }
                            p { class: "uis-emp-title", "{empty_mv}" }
                        }
                    }
                    for c in mv {
                        MvCard { key: "{c.id}", card: c.clone(), locked }
                    }
                }
            }
        }
    }
}

#[component]
pub fn MediaView() -> Element {
    let ui = use_context::<Ui>();
    let songs = songs_of(ui);
    let locked = finalized(ui);
    let mut folder = use_signal(|| match ui.nav.peek().clone() {
        Nav::Project(id) => ui.projects.peek().iter().find(|p| p.id == id).map(|p| p.path.clone()).unwrap_or_default(),
        _ => String::new(),
    });
    let width = use_link_width(ui);
    // hidden or shown, the choice is remembered (shown the first time, as in the Electron app)
    let mut hide_linked = use_signal(|| ui.st().setting("hide-linked-media") == "1");
    let files = if folder().is_empty() { Vec::new() } else { media::scan_folder(Path::new(&folder())).unwrap_or_default() };
    let linked_paths: Vec<String> = ui.cards.read().iter().filter(|c| !c.media_path.is_empty()).map(|c| c.media_path.to_lowercase()).collect();
    let (lbl_none, tip_choose, empty_browse, empty_none, all_linked, tip_auto) =
        (ui.t("lblPnlLinkNoFolder"), ui.t("tipPnlLinkChooseFolder"), ui.t("empPnlLinkBrowseFolder"), ui.t("empPnlLinkNoAudio"), ui.t("empPnlMediaAllLinked"), ui.t("tipPnlMediaAutoLink"));
    let (tip_hide, tip_show) = (ui.t("tipPnlMediaHideLinked"), ui.t("tipPnlMediaShowLinked"));
    let linked_count = files.iter().filter(|f| linked_paths.contains(&f.to_string_lossy().to_lowercase())).count();
    let shown: Vec<String> = files.iter().map(|f| f.to_string_lossy().into_owned()).filter(|f| !hide_linked() || !linked_paths.contains(&f.to_lowercase())).collect();
    let songs_auto = songs.clone();
    let files_auto = files.clone();
    let total = files.len();
    let folder_name = Path::new(&folder()).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| folder());
    let bottom = list_bottom(ui);
    rsx! {
        div { class: "ap-work ap-split",
            SongsPanel { mode: Mode::Media, width: width() }
            UisPnlSplitter { on_resize_start: move |e: MouseEvent| drag_width(width, e.client_coordinates().x, 200.0, 700.0) }
            div { style: "flex: 1; min-width: 0; display: flex; flex-direction: column; overflow: hidden;",
                div { style: "{HEAD}",
                    span { style: "display: inline-flex; color: var(--uis-text-mute)", Icon { width: 14, height: 14, icon: LdDisc } }
                    span { style: "{HEAD_TXT}; flex: 1;",
                        if folder().is_empty() {
                            "{lbl_none}"
                        } else {
                            "{folder_name}"
                        }
                        if total > 0 {
                            " ({total})"
                        }
                    }
                    button {
                        class: "uis-btn ico small",
                        title: "{tip_choose}",
                        onclick: move |_| {
                            spawn(async move {
                                if let Some(dir) = rfd::AsyncFileDialog::new().set_parent(&dioxus::desktop::window().window).pick_folder().await {
                                    folder.set(dir.path().to_string_lossy().into_owned());
                                }
                            });
                        },
                        Icon { width: 14, height: 14, icon: LdFolderOpen }
                    }
                    if total > 0 {
                        button {
                            class: "uis-btn ico small",
                            title: "{tip_auto}",
                            disabled: locked,
                            onclick: move |_| {
                                // a file whose number and title match a song that has no media yet is linked to it
                                let mut n = 0;
                                for f in &files_auto {
                                    let stem = f.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                                    let number = stem.split(|c: char| !c.is_ascii_digit()).next().and_then(|d| d.parse::<i64>().ok());
                                    let name = engine::cards::strip_track_prefix(&stem).to_lowercase();
                                    if let Some(song) = songs_auto.iter().find(|s| s.media_path.is_empty() && s.name.to_lowercase() == name && (number.is_none() || s.sort_number == number)) {
                                        if ui.st().update_card(&song.id, &CardPatch { media_path: Some(f.to_string_lossy().into_owned()), ..Default::default() }).is_ok() {
                                            n += 1;
                                        }
                                    }
                                }
                                ui.reload();
                                ui.ok(format!("{n} {}", ui.t("tstAppMediaAutoLinked")));
                            },
                            Icon { width: 14, height: 14, icon: LdWandSparkles }
                        }
                    }
                    if linked_count > 0 {
                        button {
                            class: if hide_linked() { "uis-btn ico small active" } else { "uis-btn ico small" },
                            title: if hide_linked() { "{tip_show}" } else { "{tip_hide}" },
                            onclick: move |_| {
                                let now = !hide_linked();
                                hide_linked.set(now);
                                ui.st().set_setting("hide-linked-media", if now { "1" } else { "0" });
                            },
                            if hide_linked() {
                                Icon { width: 14, height: 14, icon: LdCircleDashed }
                            } else {
                                Icon { width: 14, height: 14, icon: LdDisc }
                            }
                        }
                    }
                }
                div { style: "flex: 1; overflow-y: auto; padding: 12px; padding-bottom: {bottom}px; display: flex; flex-direction: column; gap: 6px;",
                    if folder().is_empty() || files.is_empty() || shown.is_empty() {
                        div { class: "uis-emp",
                            span { style: "display: inline-flex; color: var(--uis-text-mute)", Icon { icon: LdDisc } }
                            p { class: "uis-emp-title",
                                if folder().is_empty() {
                                    "{empty_browse}"
                                } else if files.is_empty() {
                                    "{empty_none}"
                                } else {
                                    "{all_linked}"
                                }
                            }
                        }
                    }
                    for f in shown {
                        {
                            let owner = songs.iter().find(|s| s.media_path.eq_ignore_ascii_case(&f)).cloned();
                            rsx! {
                                MediaFileCard { key: "{f}", path: f.clone(), owner, locked }
                            }
                        }
                    }
                }
            }
        }
    }
}
