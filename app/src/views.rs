// ⚠ NO HARDCODED UI STRINGS IN THIS FILE.
//   Every string a user can read MUST go through the t() lookup, and the key MUST be a string literal at the call site.

//! The workspace window: the sidebar (environments and projects), the toolbar, the card grid or list and the status line.
//! The dialogs are in `dialogs.rs`, the three working views (sort, link, media) in `work.rs`.

use crate::{
    catalog::{UisAppHeader, UisMenu, UisMenuItem, UisPnlSplitter, UisPnlToggle, UisPopover},
    dialogs,
    dnd::{Dragged, DropAt},
    exchange,
    lucide_extra::{LdCalendarArrowDown, LdCalendarArrowUp},
    ui::{Act, Dialog, Nav, Tone, Ui, View},
    work,
};
use dioxus::prelude::*;
use dioxus_free_icons::{
    Icon,
    icons::ld_icons::{
        LdArrowDownAZ, LdArrowDownZA, LdArrowUpDown, LdCheck, LdChevronDown, LdChevronRight, LdDisc3, LdDownload, LdFile, LdFileMusic, LdFilePlus, LdFileSpreadsheet, LdFolder,
        LdFolderOpen, LdFolderPlus, LdFolderUp, LdGlobe, LdLanguages, LdLayoutGrid, LdLink2, LdList, LdMicVocal, LdMusic, LdMusic2, LdPencil, LdPlus, LdRefreshCw, LdRotateCcw, LdSearch, LdSquareCheckBig, LdStar, LdTag,
        LdTextCursor, LdTrash2, LdUnlink, LdUpload, LdX,
    },
};
use engine::store::Project;

/// Drag a splitter: the width follows the pointer until the button is released (the width is a signal the caller keeps).
pub fn drag_width(mut width: Signal<f64>, start_x: f64, min: f64, max: f64) {
    let start_w = *width.peek();
    spawn(async move {
        let js = format!(
            "const x0={start_x},w0={start_w};const mv=e=>dioxus.send(w0+e.clientX-x0);const up=()=>{{window.removeEventListener('mousemove',mv);window.removeEventListener('mouseup',up);dioxus.send(-1)}};window.addEventListener('mousemove',mv);window.addEventListener('mouseup',up);"
        );
        let mut eval = document::eval(&js);
        while let Ok(w) = eval.recv::<f64>().await {
            if w < 0.0 {
                break;
            }
            width.set(w.clamp(min, max));
        }
    });
}

#[component]
pub fn Workspace() -> Element {
    let ui = use_context::<Ui>();
    let view = (ui.view)();
    let mut open = use_signal(|| true);
    // the width of the project panel is remembered, as in the Electron app (220 the first time)
    let width = use_signal(|| ui.st().setting("sidebar-width").parse::<f64>().unwrap_or(220.0).clamp(200.0, 520.0));
    use_effect(move || ui.st().set_setting("sidebar-width", &(width() as i64).to_string()));
    let tip_toggle = if open() { ui.t("tipPnlProjectHide") } else { ui.t("tipPnlProjectShow") };
    rsx! {
        div { class: "ap-body",
            if open() {
                Sidebar { width: width() }
                UisPnlSplitter {
                    on_resize_start: move |e: MouseEvent| drag_width(width, e.client_coordinates().x, 200.0, 520.0),
                }
            }
            UisPnlToggle { open: open(), on_toggle: move |_| open.toggle(), title: tip_toggle }
            div { class: "ap-main",
                match view {
                    View::Cards => rsx! {
                        TagStrip {}
                        crate::card_view::CardArea {}
                    },
                    View::Sort => rsx! {
                        work::WorkStrip { sort: true }
                        work::SortView {}
                    },
                    View::Link => rsx! {
                        work::WorkStrip { sort: false }
                        work::LinkView {}
                    },
                    View::Media => rsx! {
                        work::WorkStrip { sort: false }
                        work::MediaView {}
                    },
                }
            }
        }
        dialogs::Dialogs {}
        crate::player::PlayerBar {}
        StatusBar {}
    }
}

// ─── Sidebar ────────────────────────────────────────────────────────────────────────────────────

/// The side panel: the vertical tool bar (the catalog's `.uis-barv-tools`), then the environment and the projects.
#[component]
fn Sidebar(width: f64) -> Element {
    let ui = use_context::<Ui>();
    let envs = (ui.envs)();
    let env = (ui.env)();
    let nav = (ui.nav)();
    let (lbl_all, lbl_unsorted, lbl_untagged) = (ui.t("lblPnlProjectAll"), ui.t("lblPnlProjectUnsorted"), ui.t("lblPnlProjectUntagged"));
    let tip_new = ui.t("tipSbrNewProject");
    let (lbl_no_cards, lbl_drop_hint) = (ui.t("empGLViewNoCardsFound"), ui.t("empGLViewDropOnProject"));
    let tip_excel = match &nav {
        Nav::Project(id) => ui.projects.read().iter().find(|p| &p.id == id).map(|p| format!("{} \"{}\"", ui.t("tipSbrExportExcel"), p.name)).unwrap_or_else(|| ui.t("tipSbrExportExcel")),
        _ => ui.t("tipSbrExportExcel"),
    };
    let tip_sync = ui.t("tipSbrSyncUrls");
    // the sync button works on the open project, and only when it holds a playlist address on a music site
    let sync_project = match &nav {
        Nav::Project(id) if ui.st().has_playlist(id) => Some(id.clone()),
        _ => None,
    };
    let (tip_doc, tip_sort_date, tip_sort_name) = (ui.t("tipSbrCreateDoc"), ui.t("tipHdrSortByDate"), ui.t("tipHdrSortByName"));
    let projects = ui.sorted_projects(ui.env_projects());
    let roots: Vec<Project> = projects.iter().filter(|p| p.parent_id.as_ref().is_none_or(|parent| !projects.iter().any(|x| &x.id == parent))).cloned().collect();
    let lbl_root = ui.t("btnPnlProjectMoveToRoot");
    let current_global = envs.iter().find(|e| e.id == env).is_some_and(|e| e.is_global);
    let unsorted_count = ui.unsorted_count();
    let root_over = ui.is_over(&DropAt::Root);
    let (mut nav_s, mut dialog) = (ui.nav, ui.dialog);
    let (mut by_name, mut asc) = (ui.proj_sort_name, ui.proj_sort_asc);
    let (name_now, asc_now) = (by_name(), asc());
    let nav_now = nav.clone();
    let all_count = ui.env_cards().len();
    let untagged_count = ui.env_cards().iter().filter(|c| c.tags.is_empty()).count();
    rsx! {
        // ⚠ the width is its own attributes so that the style string never changes: a rewritten string loses a shorthand holding a var()
        aside { style: "background: var(--uis-pnl-bgd); border-right: 1px solid var(--uis-border); display: flex; flex-direction: row; overflow: hidden; padding: 0;", width: "{width}px", min_width: "{width}px",
            div { class: "uis-barv-tools",
                div { class: "uis-barv-grp",
                    button {
                        class: if !name_now { "uis-btn ico active" } else { "uis-btn ico" },
                        title: "{tip_sort_date}",
                        onclick: move |_| {
                            if by_name() {
                                by_name.set(false);
                                asc.set(false);
                            } else {
                                asc.toggle();
                            }
                        },
                        if !name_now && asc_now {
                            Icon { width: 14, height: 14, icon: LdCalendarArrowUp }
                        } else {
                            Icon { width: 14, height: 14, icon: LdCalendarArrowDown }
                        }
                    }
                    button {
                        class: if name_now { "uis-btn ico active" } else { "uis-btn ico" },
                        title: "{tip_sort_name}",
                        onclick: move |_| {
                            if by_name() {
                                asc.toggle();
                            } else {
                                by_name.set(true);
                                asc.set(true);
                            }
                        },
                        if name_now && !asc_now {
                            Icon { width: 14, height: 14, icon: LdArrowDownZA }
                        } else {
                            Icon { width: 14, height: 14, icon: LdArrowDownAZ }
                        }
                    }
                }
                div { class: "uis-barv-grp",
                    button {
                        class: "uis-btn ico",
                        title: "{tip_new}",
                        disabled: current_global,
                        onclick: move |_| {
                            let parent = match nav_now.clone() {
                                Nav::Project(p) => Some(p),
                                _ => None,
                            };
                            dialog.set(Dialog::Project { id: None, parent });
                        },
                        Icon { width: 14, height: 14, icon: LdFolderPlus }
                    }
                    button {
                        class: "uis-btn ico",
                        title: "{tip_sync}",
                        disabled: sync_project.is_none(),
                        onclick: move |_| {
                            if let Some(id) = sync_project.clone() {
                                exchange::sync_music_urls(ui, id);
                            }
                        },
                        Icon { width: 14, height: 14, icon: LdRefreshCw }
                    }
                }
                div { class: "uis-barv-grp",
                    button { class: "uis-btn ico", title: "{tip_excel}", onclick: move |_| exchange::export_excel(ui), Icon { width: 14, height: 14, icon: LdFileSpreadsheet } }
                    button { class: "uis-btn ico", title: "{tip_doc}", onclick: move |_| dialog.set(Dialog::Doc), Icon { width: 14, height: 14, icon: LdFilePlus } }
                }
            }
            div { class: "uis-pnl", style: "flex: 1; min-width: 0; overflow-y: auto; display: flex; flex-direction: column; padding: 12px 0;",
                if !envs.is_empty() {
                    EnvSelector {}
                }
                button { class: if nav == Nav::All { "uis-lvw-item active" } else { "uis-lvw-item" }, onclick: move |_| nav_s.set(Nav::All),
                    Icon { icon: LdFolder }
                    span { class: "uis-lvw-item-name", style: "flex: 1;", "{lbl_all}" }
                    span { class: "uis-lvw-item-badge", "{all_count}" }
                }
                if unsorted_count > 0 {
                    button { class: if nav == Nav::Unsorted { "uis-lvw-item active" } else { "uis-lvw-item" }, onclick: move |_| nav_s.set(Nav::Unsorted),
                        Icon { icon: LdFolder, style: "color: var(--uis-warn);" }
                        span { class: "uis-lvw-item-name", style: "flex: 1;", "{lbl_unsorted}" }
                        span { class: "uis-lvw-item-badge", style: "background: color-mix(in srgb, var(--uis-warn) 14%, transparent); color: var(--uis-warn);", "{unsorted_count}" }
                    }
                }
                if untagged_count > 0 {
                    button { class: if nav == Nav::Untagged { "uis-lvw-item active" } else { "uis-lvw-item" }, onclick: move |_| nav_s.set(Nav::Untagged),
                        Icon { icon: LdTag, style: "color: var(--uis-tag);" }
                        span { class: "uis-lvw-item-name", style: "flex: 1;", "{lbl_untagged}" }
                        span { class: "uis-lvw-item-badge", style: "background: color-mix(in srgb, var(--uis-tag) 14%, transparent); color: var(--uis-tag);", "{untagged_count}" }
                    }
                }
                if !roots.is_empty() {
                    div { style: "height: 1px; background: var(--uis-border); margin: 6px 8px;" }
                }
                if roots.is_empty() {
                    p { class: "uis-hnt", style: "padding: 8px 12px;", "{lbl_no_cards}" }
                } else {
                    // while a project is carried: the strip that takes it back to the top level
                    if matches!(ui.drag.read().live, Some((Dragged::Project(_), _))) {
                        div {
                            style: if root_over {
                                "margin: 0 8px 4px; padding: 5px 8px; border-radius: 6px; border: 1px dashed var(--uis-accent); background: transparent; font-size: 11px; color: var(--uis-accent); text-align: center; transition: all 0.15s;"
                            } else {
                                "margin: 0 8px 4px; padding: 5px 8px; border-radius: 6px; border: 1px dashed var(--uis-border-strong); background: transparent; font-size: 11px; color: var(--uis-text-mute); text-align: center; transition: all 0.15s;"
                            },
                            onmouseenter: move |_| ui.drag_over(DropAt::Root),
                            onmouseleave: move |_| ui.drag_leave(&DropAt::Root),
                            "{lbl_root}"
                        }
                    }
                    for p in roots {
                        ProjectRow { key: "{p.id}", project: p.clone(), depth: 0 }
                    }
                }
                if matches!(ui.drag.read().live, Some((Dragged::Cards(_), _))) {
                    div { style: "margin: 8px 8px 0; padding: 8px; border-radius: 8px; border: 1px dashed var(--uis-border-strong); font-size: 11px; color: var(--uis-text-mute); text-align: center;", "{lbl_drop_hint}" }
                }
            }
        }
    }
}

/// The environment box at the top of the panel: Electron's `uis-ddn-ext` button and the catalog popover that holds the list,
/// with the rename, delete and new rows inline.
#[component]
fn EnvSelector() -> Element {
    let ui = use_context::<Ui>();
    let envs = (ui.envs)();
    let env = (ui.env)();
    let (mut env_s, mut nav_s, mut selected) = (ui.env, ui.nav, ui.selected);
    let mut open = use_signal(|| false);
    let mut renaming = use_signal(|| Option::<String>::None);
    let mut rename_value = use_signal(String::new);
    let mut new_env = use_signal(|| Option::<String>::None);
    let active = envs.iter().find(|e| e.id == env).cloned();
    let active_name = active.as_ref().map(|e| e.name.clone()).unwrap_or_else(|| "-".to_owned());
    let (tip_edit, tip_del, plh_new, lbl_new) = (ui.t("btnCardEdit"), ui.t("tipCardDelete"), ui.t("plhPnlProjectEnvName"), ui.t("btnPnlProjectEnvNew"));
    rsx! {
        div { style: "position: relative; padding: 0 8px 8px; border-bottom: 1px solid var(--uis-border); margin-bottom: 6px;",
            button { id: "amlp-env-btn", class: if open() { "uis-ddn-ext open" } else { "uis-ddn-ext" }, onclick: move |_| open.toggle(),
                span { style: "display: flex; align-items: center; gap: 5px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;",
                    if active.as_ref().is_some_and(|e| e.is_global) {
                        Icon { class: "uis-ico-inline", style: "color: var(--uis-accent);", icon: LdGlobe }
                    }
                    "{active_name}"
                }
                Icon { class: "uis-ico-inline", style: "color: var(--uis-text-mute);", icon: LdChevronDown }
            }
            UisPopover {
                anchor_id: "amlp-env-btn",
                open: open(),
                min_width_anchor: true,
                on_close: move |_| {
                    open.set(false);
                    renaming.set(None);
                    new_env.set(None);
                },
                for (idx, e) in envs.iter().cloned().enumerate() {
                    div { key: "{e.id}",
                        if idx == 1 {
                            div { class: "uis-men-divider" }
                        }
                        if renaming().as_deref() == Some(e.id.as_str()) {
                            div { class: "uis-pop-item",
                                input {
                                    class: "uis-txt",
                                    style: "flex: 1;",
                                    value: "{rename_value}",
                                    onmounted: move |m| async move {
                                        let _ = m.set_focus(true).await;
                                    },
                                    oninput: move |ev| rename_value.set(ev.value()),
                                    onclick: move |ev| ev.stop_propagation(),
                                    onkeydown: {
                                        let id = e.id.clone();
                                        move |ev| {
                                            if ev.key() == Key::Enter && !rename_value().trim().is_empty() {
                                                match ui.st().rename_environment(&id, rename_value().trim()) {
                                                    Ok(()) => {
                                                        ui.reload();
                                                        renaming.set(None);
                                                        ui.ok(ui.t("tstPnlProjectEnvRenamed"));
                                                    }
                                                    Err(er) => ui.fail(er),
                                                }
                                            }
                                            if ev.key() == Key::Escape {
                                                renaming.set(None);
                                            }
                                        }
                                    },
                                }
                            }
                        } else {
                            div {
                                class: if e.id == env { "uis-pop-item active" } else { "uis-pop-item" },
                                role: "button",
                                onclick: {
                                    let id = e.id.clone();
                                    move |_| {
                                        env_s.set(id.clone());
                                        nav_s.set(Nav::All);
                                        selected.set(Default::default());
                                        open.set(false);
                                    }
                                },
                                if e.is_global {
                                    Icon { icon: LdGlobe, style: "color: var(--uis-accent);" }
                                }
                                if e.id == env {
                                    Icon { icon: LdCheck, style: "color: var(--uis-accent);" }
                                }
                                span { "{e.name}" }
                                if !e.is_global {
                                    button {
                                        class: "uis-btn ico small subtle",
                                        style: "margin-left: auto;",
                                        title: "{tip_edit}",
                                        onclick: {
                                            let (id, name) = (e.id.clone(), e.name.clone());
                                            move |ev| {
                                                ev.stop_propagation();
                                                renaming.set(Some(id.clone()));
                                                rename_value.set(name.clone());
                                            }
                                        },
                                        Icon { icon: LdTextCursor }
                                    }
                                    button {
                                        class: "uis-btn ico small subtle",
                                        title: "{tip_del}",
                                        onclick: {
                                            let id = e.id.clone();
                                            move |ev| {
                                                ev.stop_propagation();
                                                match ui.st().delete_environment(&id) {
                                                    Ok(()) => {
                                                        if *env_s.peek() == id {
                                                            let remaining: Vec<_> = ui.envs.peek().iter().filter(|x| x.id != id).cloned().collect();
                                                            let next = remaining.iter().find(|x| !x.is_global).or(remaining.first()).map(|x| x.id.clone()).unwrap_or_default();
                                                            env_s.set(next);
                                                        }
                                                        ui.reload();
                                                        ui.ok(ui.t("tstPnlProjectEnvDeleted"));
                                                    }
                                                    Err(_) => ui.fail(ui.t("tstPnlProjectEnvNotEmpty")),
                                                }
                                            }
                                        },
                                        Icon { icon: LdTrash2 }
                                    }
                                }
                            }
                        }
                    }
                }
                div { class: "uis-men-divider" }
                if let Some(typed) = new_env() {
                    div { class: "uis-pop-item",
                        input {
                            class: "uis-txt",
                            style: "flex: 1;",
                            placeholder: "{plh_new}",
                            value: "{typed}",
                            onmounted: move |m| async move {
                                let _ = m.set_focus(true).await;
                            },
                            oninput: move |ev| new_env.set(Some(ev.value())),
                            onkeydown: move |ev| {
                                let name = new_env().unwrap_or_default();
                                if ev.key() == Key::Enter && !name.trim().is_empty() {
                                    match ui.st().create_environment(name.trim()) {
                                        Ok(created) => {
                                            env_s.set(created.id);
                                            ui.reload();
                                            new_env.set(None);
                                            open.set(false);
                                            ui.ok(ui.t("tstPnlProjectEnvCreated"));
                                        }
                                        Err(er) => ui.fail(er),
                                    }
                                }
                                if ev.key() == Key::Escape {
                                    new_env.set(None);
                                }
                            },
                        }
                    }
                } else {
                    button { class: "uis-pop-item", style: "color: var(--uis-accent);", onclick: move |_| new_env.set(Some(String::new())),
                        Icon { icon: LdPlus }
                        "{lbl_new}"
                    }
                }
            }
        }
    }
}

#[component]
fn ProjectRow(project: Project, depth: usize) -> Element {
    let ui = use_context::<Ui>();
    let nav = (ui.nav)();
    let projects = ui.sorted_projects(ui.env_projects());
    let children: Vec<Project> = projects.iter().filter(|p| p.parent_id.as_deref() == Some(project.id.as_str())).cloned().collect();
    let open = ui.expanded.read().contains(&project.id);
    let active = nav == Nav::Project(project.id.clone());
    let (mut nav_s, mut dialog, mut expanded) = (ui.nav, ui.dialog, ui.expanded);
    let id = project.id.clone();
    let (id3, id4) = (id.clone(), id.clone());
    let pad = format!("display: flex; align-items: center; padding-left: {}px;", depth * 10);
    let count = {
        let tree = ui.project_tree(&project.id);
        projects.iter().filter(|p| tree.contains(&p.id)).flat_map(|p| p.card_ids.iter()).collect::<std::collections::HashSet<_>>().len()
    };
    let colour = project.color.clone();
    let tip_edit = ui.t("btnCardEdit");
    let (tip_tag, tip_translate, tip_import, tip_move) = (ui.t("tipSbrTag"), ui.t("tipSbrTranslate"), ui.t("tipSbrImportFolder"), ui.t("tipSbrMoveEnv"));
    let tip_del = ui.t("tipCardDelete");
    let at = DropAt::Project(project.id.clone());
    let (at_a, at_b) = (at.clone(), at.clone());
    let (drag_id, drag_name) = (project.id.clone(), project.name.clone());
    // Electron's two highlight styles: a project carried over it (`isDropTarget`) and cards carried over it (`dragOver`)
    let carrying_project = matches!(ui.drag.read().live, Some((Dragged::Project(_), _)));
    // ⚠ Dioxus does not clear an inline property that a later style string leaves out, so the dashed outline of a row the pointer
    // had crossed stayed on every row (all of them lit during a drag). A `key` does not help on a lone element (it only counts in a
    // list). The two highlights are classes (`ap-over-*`, app.css) that read the project's colour from `--ap-over`, a property that is
    // always present; a class is added and removed reliably.
    let cls = match (active, ui.is_over(&at), carrying_project) {
        (_, true, true) => "uis-lvw-item ap-over-project",
        (_, true, false) => "uis-lvw-item ap-over-cards",
        (true, _, _) => "uis-lvw-item active",
        _ => "uis-lvw-item",
    };
    let btn_style = format!("margin: 0; flex: 1; min-width: 0; --ap-over: {colour};");
    rsx! {
        div { "data-proj-id": "{project.id}", style: "{pad}",
            div { style: "width: 16px; flex-shrink: 0; display: flex; align-items: center; justify-content: center;",
                if !children.is_empty() {
                    button {
                        class: "uis-btn ico small subtle",
                        onclick: move |_| {
                            let mut set = expanded.write();
                            if !set.remove(&id4) {
                                set.insert(id4.clone());
                            }
                        },
                        if open {
                            Icon { icon: LdChevronDown }
                        } else {
                            Icon { icon: LdChevronRight }
                        }
                    }
                }
            }
            div { style: "flex: 1; min-width: 0;",
            div {
                class: "proj-item",
                style: "display: flex; align-items: center; margin: 1px 4px; position: relative;",
                onmouseenter: move |_| ui.drag_over(at_a.clone()),
                onmouseleave: move |_| ui.drag_leave(&at_b),
            div {
                style: "cursor: grab; padding: 0 4px; color: var(--uis-text-mute); display: flex; align-items: center; flex-shrink: 0; font-size: 10px;",
                onmousedown: move |e| {
                    e.stop_propagation();
                    ui.arm_drag(Dragged::Project(drag_id.clone()), format!("📁 {drag_name}"), &e);
                },
                onclick: move |e| e.stop_propagation(),
                "⠿"
            }
            button { class: "{cls}", style: "{btn_style}", onclick: move |_| nav_s.set(Nav::Project(id.clone())),
                Icon { icon: LdFolder, style: "color: {colour};" }
                span { class: "uis-lvw-item-name", style: "flex: 1;",
                    "{project.name}"
                    if project.version > 1 {
                        span { style: "font-size: 9px; font-weight: 700; color: {colour}; border: 1px solid {colour}55; border-radius: 4px; padding: 1px 4px; margin-left: 4px; opacity: 0.85;", "v{project.version}" }
                    }
                }
                span { class: "uis-lvw-item-badge", "{count}" }
            }
            div { class: "proj-acts",
                button {
                    class: "uis-btn ico small subtle",
                    title: "{tip_edit}",
                    onclick: move |e| {
                        e.stop_propagation();
                        dialog.set(Dialog::Project { id: Some(id3.clone()), parent: None });
                    },
                    Icon { icon: LdPencil }
                }
                button {
                    class: "uis-btn ico small subtle",
                    title: "{tip_tag}",
                    onclick: {
                        let (project_id, name) = (project.id.clone(), project.name.clone());
                        move |e| {
                            e.stop_propagation();
                            let ids = ui.tree_card_ids(&project_id);
                            dialog.set(Dialog::Tag { scope: format!("{} · {name}", ids.len()), ids });
                        }
                    },
                    Icon { icon: LdTag }
                }
                button {
                    class: "uis-btn ico small subtle",
                    title: "{tip_translate}",
                    onclick: {
                        let project_id = project.id.clone();
                        move |e| {
                            e.stop_propagation();
                            dialog.set(Dialog::Translate(ui.tree_card_ids(&project_id)));
                        }
                    },
                    Icon { icon: LdLanguages }
                }
                button {
                    class: "uis-btn ico small subtle",
                    title: "{tip_import}",
                    onclick: {
                        let project_id = project.id.clone();
                        move |e| {
                            e.stop_propagation();
                            let project = project_id.clone();
                            spawn(async move {
                                if let Some(dir) = rfd::AsyncFileDialog::new().set_parent(&dioxus::desktop::window().window).pick_folder().await {
                                    dialog.set(Dialog::Confirm {
                                        title: ui.t("cfmSbrImportSongsTitle"),
                                        sub: ui.t("cfmSbrImportSongsSub"),
                                        act: Act::ImportFolder { project, folder: dir.path().to_string_lossy().into_owned() },
                                    });
                                }
                            });
                        }
                    },
                    Icon { icon: LdFolderOpen }
                }
                button {
                    class: "uis-btn ico small subtle",
                    title: "{tip_move}",
                    onclick: {
                        let project_id = project.id.clone();
                        move |e| {
                            e.stop_propagation();
                            dialog.set(Dialog::MoveFolder(project_id.clone()));
                        }
                    },
                    Icon { icon: LdFolderUp }
                }
                button {
                    class: "uis-btn ico small subtle",
                    title: "{tip_del}",
                    onclick: {
                        let id = project.id.clone();
                        move |e| {
                            e.stop_propagation();
                            dialog.set(Dialog::DeleteProject(id.clone()));
                        }
                    },
                    Icon { icon: LdTrash2 }
                }
            }
            }
            }
        }
        if open {
            for c in children {
                ProjectRow { key: "{c.id}", project: c.clone(), depth: depth + 1 }
            }
        }
    }
}

// ─── Header ─────────────────────────────────────────────────────────────────────────────────────

/// The work buttons of the window, in the catalog's header (`UisAppHeader`, bare: the window bar above shows the name
/// and the version): the search, the welded icon groups and the new button. The tags and the selection are the strip at the top of
/// the content area (`TagStrip`), as in the Electron app.
#[component]
pub fn Header(app_name: String, app_version: String) -> Element {
    let ui = use_context::<Ui>();
    let (mut search, mut kind, mut fav, mut warn, mut sort_name, mut sort_asc, mut list_view, mut view, mut tags) =
        (ui.search, ui.kind, ui.fav, ui.warn, ui.sort_name, ui.sort_asc, ui.list_view, ui.view, ui.tags);
    let mut menu_import = use_signal(|| false);
    let mut menu_export = use_signal(|| false);
    let (plh, tip_help) = (ui.t("plhHdrSearch"), ui.t("tipHdrSearchHelp"));
    let current_kind = kind();
    let current_warn = warn();
    let on_cards = view() == View::Cards;
    let in_project = matches!(*ui.nav.read(), Nav::Project(_));
    let (grid_now, by_name, asc) = (!list_view(), sort_name(), sort_asc());
    let shown_count = ui.shown_cards().len();
    let kinds = [
        ("note", ui.t("lblStgColorNote")),
        ("music", ui.t("lblAppTypeMusic")),
        ("vocal", ui.t("lblAppTypeVocal")),
        ("song", ui.t("lblAppTypeSong")),
    ];
    let counts = ui.warn_counts();
    let warns = [
        ("tagged", ui.t("tipHdrWarningNoTagged"), "#a78bfa".to_owned(), counts[0]),
        ("media", ui.t("tipHdrWarningNoMedia"), "#f97316".to_owned(), counts[1]),
        ("lrc", ui.t("tipHdrWarningNoLrc"), "#22d3ee".to_owned(), counts[2]),
        ("ai", ui.t("tipHdrWarningNoAiLink"), ui.kind_colour("song"), counts[3]),
        ("published", ui.t("tipHdrWarningNoPublished"), ui.colour("SongStatePublished"), counts[4]),
    ];
    let (tip_fav, tip_grid, tip_list) = (ui.t("tipHdrFavorites"), ui.t("tipHdrGridView"), ui.t("tipHdrListView"));
    let (tip_sort_date, tip_sort_name) = (ui.t("tipHdrSortByDate"), ui.t("tipHdrSortByName"));
    let (tip_link, tip_media, tip_reorder) = (ui.t("tipHdrLinkView"), ui.t("tipHdrMediaLinkView"), ui.t("tipHdrSortView"));
    // Electron names the open project on the Export button and its first menu item
    let open_project = match ui.nav.read().clone() {
        Nav::Project(id) => ui.projects.read().iter().find(|p| p.id == id).map(|p| p.name.clone()),
        _ => None,
    };
    let (tip_export, export_cards_label) = match &open_project {
        Some(name) => (format!("{} \"{name}\"", ui.t("tipHdrExportProj")), format!("{} \"{name}\"", ui.t("tipHdrExportProj"))),
        None => (ui.t("tipHdrExportAll"), ui.t("tipHdrExportCards")),
    };
    let (btn_new, tip_import) = (ui.t("btnHdrNew"), ui.t("tipHdrImport"));
    rsx! {
        UisAppHeader { app_name, app_version, bare: true,
            div { class: "uis-txt-search uis-barh-search ap-search",
                span { class: "uis-txt-search-glyph", Icon { width: 14, height: 14, icon: LdSearch } }
                input {
                    class: "uis-txt-search-input",
                    placeholder: "{plh}",
                    title: "{tip_help}",
                    value: "{search}",
                    oninput: move |e| {
                        search.set(e.value());
                        tags.set(Vec::new());
                    },
                }
                if !search().is_empty() {
                    button { class: "uis-txt-search-clear", onclick: move |_| search.set(String::new()), Icon { width: 12, height: 12, icon: LdX } }
                }
            }
            span { class: "uis-count", style: "background:var(--uis-bg-row);color:var(--uis-text-mute);font-size:12px;padding:2px 8px", "{shown_count}" }
            div { class: "uis-barh-grp",
                for (k , label) in kinds {
                    button {
                        key: "{k}-{current_kind == k}",
                        class: if current_kind == k { "uis-btn ico active" } else { "uis-btn ico" },
                        style: if current_kind == k { format!("color:{0};--uis-accent:{0}", ui.kind_colour(k)) } else { String::new() },
                        title: "{label}",
                        disabled: !on_cards,
                        onclick: move |_| {
                            kind.set(if *kind.peek() == k { String::new() } else { k.to_owned() });
                            tags.set(Vec::new());
                        },
                        match k {
                            "note" => rsx! { Icon { width: 14, height: 14, icon: LdFile } },
                            "music" => rsx! { Icon { width: 14, height: 14, icon: LdMusic } },
                            "vocal" => rsx! { Icon { width: 14, height: 14, icon: LdMicVocal } },
                            _ => rsx! { Icon { width: 14, height: 14, icon: LdMusic2 } },
                        }
                    }
                }
                button {
                    class: if fav() { "uis-btn ico active ap-fav-on" } else { "uis-btn ico" },
                    title: "{tip_fav}",
                    disabled: !on_cards,
                    onclick: move |_| fav.toggle(),
                    Icon { width: 14, height: 14, icon: LdStar, style: if fav() { "fill:var(--uis-star)" } else { "fill:none" } }
                }
            }
            div { class: "uis-barh-grp",
                for (w , label , colour , count) in warns {
                    button {
                        key: "{w}-{current_warn == w}",
                        class: if current_warn == w { "uis-btn ico active" } else { "uis-btn ico" },
                        style: if current_warn == w { format!("--uis-accent:{colour}") } else { String::new() },
                        title: "{label}",
                        disabled: !on_cards || (count == 0 && current_warn != w),
                        onclick: move |_| warn.set(if *warn.peek() == w { String::new() } else { w.to_owned() }),
                        match w {
                            "tagged" => rsx! { Icon { width: 14, height: 14, icon: LdTag } },
                            "media" => rsx! { Icon { width: 14, height: 14, icon: LdDisc3 } },
                            "lrc" => rsx! { Icon { width: 14, height: 14, icon: LdFileMusic } },
                            "ai" => rsx! { Icon { width: 14, height: 14, icon: LdMusic2 } },
                            _ => rsx! { Icon { width: 14, height: 14, icon: LdGlobe } },
                        }
                    }
                }
            }
            div { class: "uis-barh-grp",
                button { class: if grid_now && on_cards { "uis-btn ico active" } else { "uis-btn ico" }, title: "{tip_grid}", onclick: move |_| { list_view.set(false); view.set(View::Cards) }, Icon { width: 14, height: 14, icon: LdLayoutGrid } }
                button { class: if !grid_now && on_cards { "uis-btn ico active" } else { "uis-btn ico" }, title: "{tip_list}", onclick: move |_| { list_view.set(true); view.set(View::Cards) }, Icon { width: 14, height: 14, icon: LdList } }
            }
            div { class: "uis-barh-grp",
                button { class: if view() == View::Link { "uis-btn ico active" } else { "uis-btn ico" }, title: "{tip_link}", disabled: !in_project, onclick: move |_| view.set(if view() == View::Link { View::Cards } else { View::Link }), Icon { width: 14, height: 14, icon: LdLink2 } }
                button { class: if view() == View::Media { "uis-btn ico active" } else { "uis-btn ico" }, title: "{tip_media}", disabled: !in_project, onclick: move |_| view.set(if view() == View::Media { View::Cards } else { View::Media }), Icon { width: 14, height: 14, icon: LdDisc3 } }
                button { class: if view() == View::Sort { "uis-btn ico active" } else { "uis-btn ico" }, title: "{tip_reorder}", disabled: !in_project, onclick: move |_| view.set(if view() == View::Sort { View::Cards } else { View::Sort }), Icon { width: 14, height: 14, icon: LdArrowUpDown } }
            }
            div { class: "uis-barh-grp",
                button {
                    class: if !by_name { "uis-btn ico active" } else { "uis-btn ico" },
                    title: "{tip_sort_date}",
                    onclick: move |_| {
                        if sort_name() {
                            sort_name.set(false);
                            sort_asc.set(false);
                        } else {
                            sort_asc.toggle();
                        }
                    },
                    if !by_name && asc {
                        Icon { width: 14, height: 14, icon: LdCalendarArrowUp }
                    } else {
                        Icon { width: 14, height: 14, icon: LdCalendarArrowDown }
                    }
                }
                button {
                    class: if by_name { "uis-btn ico active" } else { "uis-btn ico" },
                    title: "{tip_sort_name}",
                    onclick: move |_| {
                        if sort_name() {
                            sort_asc.toggle();
                        } else {
                            sort_name.set(true);
                            sort_asc.set(false);
                        }
                    },
                    if by_name && asc {
                        Icon { width: 14, height: 14, icon: LdArrowDownAZ }
                    } else {
                        Icon { width: 14, height: 14, icon: LdArrowDownZA }
                    }
                }
            }
            div { class: "uis-barh-grp",
                button { id: "ap-import-anchor", class: if menu_import() { "uis-btn ico active" } else { "uis-btn ico" }, title: "{tip_import}", onclick: move |_| menu_import.toggle(), Icon { width: 14, height: 14, icon: LdDownload } }
                button { id: "ap-export-anchor", class: if menu_export() { "uis-btn ico active" } else { "uis-btn ico" }, title: "{tip_export}", onclick: move |_| menu_export.toggle(), Icon { width: 14, height: 14, icon: LdUpload } }
            }
            button {
                class: "uis-btn ico primary",
                title: "{btn_new} (Ctrl+N)",
                disabled: ui.draft_busy.read().to_owned(),
                onclick: move |_| ui.open_new_card(),
                if *ui.draft_busy.read() {
                    Icon { width: 14, height: 14, icon: LdRotateCcw, class: "uis-spn" }
                } else {
                    Icon { width: 14, height: 14, icon: LdPlus }
                }
            }
            UisMenu { anchor_id: "ap-import-anchor", open: menu_import(), on_close: move |_| menu_import.set(false),
                UisMenuItem { label: "JSON".to_owned(), on_click: move |_| exchange::import_json(ui) }
                UisMenuItem { label: ".amlp".to_owned(), on_click: move |_| exchange::import_amlp(ui) }
            }
            UisMenu { anchor_id: "ap-export-anchor", open: menu_export(), on_close: move |_| menu_export.set(false),
                UisMenuItem { label: export_cards_label, on_click: move |_| exchange::export_cards(ui) }
                UisMenuItem { label: ui.t("btnHdrExportAmlp"), on_click: move |_| exchange::export_amlp(ui) }
                UisMenuItem { label: ui.t("tipHdrExportAi"), on_click: move |_| exchange::export_ai(ui) }
                UisMenuItem { label: ui.t("btnHdrExportBackup"), on_click: move |_| exchange::export_backup(ui) }
            }
        }
    }
}

/// The strip at the top of the content area (the catalog's `.uis-barh-view`, right of the side panel, on the content surface): the
/// actions on the selected cards (only while cards are selected), the tag chips, and the selection's own counter at the end. Drawn
/// only in the cards view, and only when there is a tag or a selection (an empty strip would be a blank band).
#[component]
fn TagStrip() -> Element {
    let ui = use_context::<Ui>();
    let tags = ui.all_tags();
    let ids = ui.selected_ids();
    if tags.is_empty() && ids.is_empty() {
        return rsx! {};
    }
    rsx! {
        div { class: "uis-barh-view", TagStripItems { tags, ids } }
    }
}

#[component]
fn TagStripItems(tags: Vec<String>, ids: Vec<String>) -> Element {
    let ui = use_context::<Ui>();
    let (mut selected, mut dialog, mut tag_sel) = (ui.selected, ui.dialog, ui.tags);
    let n = ids.len();
    let in_project = matches!(*ui.nav.read(), Nav::Project(_));
    let shown: Vec<String> = ui.shown_cards().into_iter().map(|c| c.id).collect();
    let all_label = format!("{} ({})", ui.t("btnHdrSelectAll"), shown.len());
    let (clear, sel) = (ui.t("btnHdrClearSelection"), ui.t("lblHdrSelected"));
    let (tip_tag, tip_add, tip_unlink, tip_move, tip_del) = (ui.t("tipSbrTag"), ui.t("tipSbrAddToProject"), ui.t("tipSbrUnlink"), ui.t("tipSbrMoveEnv"), ui.t("tipCardDelete"));
    let (ids_add, ids_move, ids_tag, ids_tr) = (ids.clone(), ids.clone(), ids.clone(), ids.clone());
    let tip_translate = ui.t("tipSbrTranslate");
    let sel_label = sel.clone();
    rsx! {
        if n > 0 {
            button { class: "uis-btn ico small subtle", title: "{tip_tag}", onclick: move |_| dialog.set(Dialog::Tag { ids: ids_tag.clone(), scope: format!("{n} {sel_label}") }), Icon { width: 14, height: 14, icon: LdTag } }
            button { class: "uis-btn ico small subtle", title: "{tip_translate}", onclick: move |_| dialog.set(Dialog::Translate(ids_tr.clone())), Icon { width: 14, height: 14, icon: LdLanguages } }
            button { class: "uis-btn ico small subtle", title: "{tip_add}", onclick: move |_| dialog.set(Dialog::AddToProject(ids_add.clone())), Icon { width: 14, height: 14, icon: LdFolderPlus } }
            if in_project {
                button {
                    class: "uis-btn ico small subtle",
                    title: "{tip_unlink}",
                    onclick: move |_| dialog.set(Dialog::Confirm { title: ui.t("cfmDlgUnlinkTitle"), sub: ui.t("cfmDlgUnlinkSub"), act: Act::UnlinkSelected }),
                    Icon { width: 14, height: 14, icon: LdUnlink }
                }
            }
            button { class: "uis-btn ico small subtle", title: "{tip_move}", onclick: move |_| dialog.set(Dialog::MoveEnv(ids_move.clone())), Icon { width: 14, height: 14, icon: LdFolderUp } }
            button {
                class: "uis-btn ico small subtle",
                style: "color:var(--uis-danger)",
                title: "{tip_del}",
                onclick: move |_| dialog.set(Dialog::Confirm { title: format!("{} {n} {}", ui.t("btnCardDelete"), ui.t("cfmDlgDeleteSelected")), sub: ui.t("cfmDlgDeleteCardSub"), act: Act::DeleteSelected }),
                Icon { width: 14, height: 14, icon: LdTrash2 }
            }
        }
        for tag in tags {
            {
                let on = tag_sel.read().contains(&tag);
                let colour = ui.tag_colour(&tag);
                let name = tag.clone();
                rsx! {
                    button {
                        key: "{tag}",
                        class: if on { "uis-chp on" } else { "uis-chp" },
                        style: "--chip:{colour}",
                        onclick: move |_| {
                            let mut set = tag_sel.write();
                            if let Some(i) = set.iter().position(|x| *x == name) {
                                set.remove(i);
                            } else {
                                set.push(name.clone());
                            }
                        },
                        "{tag}"
                    }
                }
            }
        }
        if n > 0 {
            div { class: "ap-push",
                Icon { width: 14, height: 14, icon: LdSquareCheckBig }
                span { "{n} {sel}" }
                button { class: "uis-chp", onclick: move |_| selected.set(shown.iter().cloned().collect()), "{all_label}" }
                button { class: "uis-chp", onclick: move |_| selected.set(Default::default()), Icon { width: 12, height: 12, icon: LdX }, "{clear}" }
            }
        }
    }
}

// ─── Status line ────────────────────────────────────────────────────────────────────────────────

#[component]
fn StatusBar() -> Element {
    let ui = use_context::<Ui>();
    let (tone, text) = (ui.status)();
    let mut status = ui.status;
    if text.is_empty() {
        return rsx! {};
    }
    rsx! {
        div { class: if tone == Tone::Err { "ap-status ap-status-err" } else { "ap-status" },
            span { class: "ap-status-text", "{text}" }
            button { class: "uis-btn ico small subtle", onclick: move |_| status.set((Tone::Plain, String::new())), Icon { width: 12, height: 12, icon: LdX } }
        }
    }
}
