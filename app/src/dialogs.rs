// ⚠ NO HARDCODED UI STRINGS IN THIS FILE.
//   Every string a user can read MUST go through the t() lookup, and the key MUST be a string literal at the call site.

//! The dialogs: the card, the project, the environment, the question, tags, moving cards and the songbook.

use crate::{
    exchange,
    catalog::{UisCboStandard, UisComboItem, UisPkrColor, UisPkrDate},
    translate::TranslateDlg,
    ui::{Act, Dialog, Nav, Ui},
};
use dioxus::prelude::*;
use dioxus_free_icons::{
    Icon,
    icons::ld_icons::{LdSquare, LdSquareCheckBig, LdArrowUpDown, LdTag, LdCopy, LdFolderOpen, LdMusic2, LdPlus, LdTrash2, LdUnlink, LdX},
};
use engine::{
    exports::DocOptions,
    langs,
    projects::{NewProject, ProjectPatch},
    store::{Card, Project, new_id},
};

#[component]
pub fn Dialogs() -> Element {
    let ui = use_context::<Ui>();
    let dialog = (ui.dialog)();
    match dialog {
        Dialog::None => rsx! {},
        Dialog::Card { id, kind, draft } => rsx! { crate::card_dlg::CardDlg { key: "{id:?}{kind}", id, kind, draft } },
        Dialog::Project { id, parent } => rsx! { ProjectDlg { key: "{id:?}{parent:?}", id, parent } },
        Dialog::DeleteProject(id) => rsx! { DeleteProjectDlg { id } },
        Dialog::Confirm { title, sub, act } => rsx! { ConfirmDlg { title, sub, act } },
        Dialog::Tag { ids, scope } => rsx! { TagDlg { ids, scope } },
        Dialog::Translate(ids) => rsx! { TranslateDlg { ids } },
        Dialog::MoveEnv(ids) => rsx! { MoveEnvDlg { ids, folder: None } },
        Dialog::MoveFolder(id) => rsx! { MoveEnvDlg { ids: Vec::new(), folder: Some(id) } },
        Dialog::AddToProject(ids) => rsx! { AddToProjectDlg { ids } },
        Dialog::Doc => rsx! { DocDlg {} },
        Dialog::Info { title, lines } => rsx! { InfoDlg { title, lines } },
        Dialog::MediaPaths(plan) => rsx! { MediaPathsDlg { plan } },
    }
}

/// A collapsible list of named files with a copy button, shared by "not found" and "outside the root" (Electron's `fileList`).
#[component]
fn FileList(label: String, items: Vec<engine::extras::NamedPath>) -> Element {
    let ui = use_context::<Ui>();
    if items.is_empty() {
        return rsx! {};
    }
    let n = if items.len() >= 300 { "300+".to_owned() } else { items.len().to_string() };
    let summary = label.replace("{n}", &n);
    let (tip, copied) = (ui.t("btnDlgMediaPathsCopyList"), ui.t("tstDlgMediaPathsListCopied"));
    let text = items.iter().map(|m| format!("{}\t{}", m.name, m.path)).collect::<Vec<_>>().join("\n");
    rsx! {
        details { style: "margin-top: 4px;",
            summary { class: "uis-hnt", style: "cursor: pointer;", "{summary}" }
            div { style: "max-height: 140px; overflow-y: auto; font-size: 11px; word-break: break-all;",
                for (i , m) in items.iter().enumerate() {
                    div { key: "{i}",
                        strong { "{m.name}" }
                        " "
                        span { class: "uis-hnt", "{m.path}" }
                    }
                }
            }
            button {
                class: "uis-btn ico small subtle",
                title: "{tip}",
                onclick: move |_| {
                    let _ = arboard::Clipboard::new().and_then(|mut b| b.set_text(text.clone()));
                    ui.ok(copied.clone());
                },
                Icon { width: 14, height: 14, icon: LdCopy }
            }
        }
    }
}

/// The stored-paths audit: what the switch WOULD do, shown before anything is written; Apply writes it (Electron's `mediaAudit` dialog).
#[component]
fn MediaPathsDlg(plan: engine::extras::MediaPlan) -> Element {
    let ui = use_context::<Ui>();
    let (title, template_key) = match plan.kind.as_str() {
        "toRelative" => (ui.t("ttlDlgMediaPathsRelative"), "msgDlgMediaPathsToRelative"),
        "toAbsolute" => (ui.t("ttlDlgMediaPathsAbsolute"), "msgDlgMediaPathsToAbsolute"),
        _ => (ui.t("ttlDlgMediaPathsRoot"), "msgDlgMediaPathsChangeRoot"),
    };
    let template = ui.t(template_key);
    let line = |c: &engine::extras::PathCounts| {
        template
            .replace("{total}", &c.total.to_string())
            .replace("{rewrite}", &c.rewrite.to_string())
            .replace("{outside}", &c.outside.to_string())
            .replace("{missing}", &c.missing.to_string())
            .replace("{foundNew}", &c.found_new.to_string())
            .replace("{foundOld}", &c.found_old.to_string())
            .replace("{relative}", &c.relative.to_string())
    };
    // Relative means EVERY path is relative to the root, so a file outside it BLOCKS the switch (the engine refuses it too)
    let outside_count = if plan.kind == "toRelative" { plan.cards.outside + plan.projects.outside } else { 0 };
    let blocked = ui.t("msgDlgMediaPathsBlocked").replace("{n}", &outside_count.to_string());
    let has_missing = !plan.cards.not_found.is_empty() || !plan.projects.not_found.is_empty();
    let sections = [(ui.t("lblDlgMediaPathsCards"), plan.cards.clone()), (ui.t("lblDlgMediaPathsProjects"), plan.projects.clone())];
    let (lbl_outside, lbl_missing) = (ui.t("lblDlgMediaPathsShowOutside"), ui.t("lblDlgMediaPathsShowMissing"));
    let (cancel, choose, apply) = (ui.t("btnGlbCancel"), ui.t("btnDlgMediaPathsChooseRoot"), ui.t("btnDlgMediaPathsApply"));
    let root_note = ui.t("msgDlgMediaPathsRootNote");
    let missing_note = if plan.kind == "changeRoot" { ui.t("msgDlgMediaPathsNotFoundNote") } else { ui.t("msgDlgMediaPathsMissingNote") };
    let relative = plan.mode == "relative";
    let (apply_root, kind) = (plan.root.clone(), plan.kind.clone());
    let kind_foot = kind.clone();
    rsx! {
        div { class: "uis-dl-backdrop", onmousedown: move |_| ui.close(),
            div { class: "uis-dlp", style: "max-width: 460px;", onmousedown: move |e| e.stop_propagation(),
                button { class: "uis-dl-close", onclick: move |_| ui.close(), Icon { width: 14, height: 14, icon: LdX } }
                Icon { class: "uis-dlp-icon", icon: LdFolderOpen, style: "color: var(--uis-accent)" }
                p { class: "uis-dlp-title", "{title}" }
                if kind != "toAbsolute" {
                    p { class: "uis-dlp-sub", "{plan.root}" }
                }
                div { class: "uis-dlp-body", style: "text-align: left; display: flex; flex-direction: column; gap: 8px;",
                    for (label , c) in sections {
                        if c.total > 0 {
                            div { key: "{label}",
                                strong { "{label}" }
                                br {}
                                span { class: "uis-hnt", "{line(&c)}" }
                                FileList { label: lbl_outside.clone(), items: c.outside_list.clone() }
                                FileList { label: lbl_missing.clone(), items: c.not_found.clone() }
                            }
                        }
                    }
                    if outside_count > 0 {
                        p { class: "uis-hnt", style: "color: var(--uis-danger);", "{blocked}" }
                    }
                    if kind == "changeRoot" {
                        p { class: "uis-hnt", "{root_note}" }
                    }
                    if has_missing {
                        p { class: "uis-hnt", "{missing_note}" }
                    }
                }
                div { class: "uis-dlp-foot",
                    button { class: "uis-btn subtle", onclick: move |_| ui.close(), "{cancel}" }
                    if kind_foot != "toAbsolute" {
                        button { class: "uis-btn", onclick: move |_| crate::settings_view::choose_media_root(ui),
                            Icon { width: 14, height: 14, icon: LdFolderOpen }
                            "{choose}"
                        }
                    }
                    button {
                        class: "uis-btn primary",
                        disabled: outside_count > 0,
                        onclick: move |_| crate::settings_view::apply_media_paths(ui, relative, &apply_root),
                        "{apply}"
                    }
                }
            }
        }
    }
}

/// The frame of a dialog: a title with a close button, a body, and a foot of buttons.
#[component]
pub fn Dlg(title: String, #[props(default)] wide: bool, #[props(default)] foot: Option<Element>, children: Element) -> Element {
    let ui = use_context::<Ui>();
    let style = if wide { "width:min(900px,94vw)" } else { "width:min(520px,94vw)" };
    rsx! {
        div { class: "uis-dl-backdrop", onmousedown: move |_| ui.close(),
            div { class: "uis-dlg", style, onmousedown: move |e| e.stop_propagation(),
                div { class: "uis-dlg-head",
                    span { class: "uis-dlg-title", "{title}" }
                    button { class: "uis-dl-close", onclick: move |_| ui.close(), Icon { width: 14, height: 14, icon: LdX } }
                }
                div { class: "uis-dlg-body ap-dlg-body", {children} }
                if let Some(foot) = foot {
                    div { class: "uis-dlg-foot", {foot} }
                }
            }
        }
    }
}

/// A caption over its control; `accent` recolours the caption (the card dialog uses its type's colour).
#[component]
pub(crate) fn Field(label: String, #[props(default)] accent: String, children: Element) -> Element {
    rsx! {
        div { class: "uis-fld-labelled",
            if !label.is_empty() {
                label { class: "uis-fld-labelled-label", style: (!accent.is_empty()).then(|| format!("color:{accent}")), "{label}" }
            }
            {children}
        }
    }
}

/// The question dialogs of the Electron app (`deleteConfirm`, `bulkDeleteConfirm`, `unlinkConfirm`, `importFolderPending`): an icon in
/// the colour of what is asked, the title, the sub line, and a confirm button that names the act with the same icon.
#[component]
fn ConfirmDlg(title: String, sub: String, act: Act) -> Element {
    let ui = use_context::<Ui>();
    let cancel = ui.t("btnGlbCancel");
    // the cards that link to a card about to be deleted: they stay, and lose the link
    let doomed: Vec<String> = match &act {
        Act::DeleteCard(id) => vec![id.clone()],
        Act::DeleteSelected => ui.selected_ids(),
        _ => Vec::new(),
    };
    let orphans: Vec<(String, String, i64)> = if doomed.is_empty() {
        Vec::new()
    } else {
        ui.cards.read().iter().filter(|c| !doomed.contains(&c.id) && c.linked_cards.iter().any(|l| doomed.contains(l))).map(|c| (c.id.clone(), c.name.clone(), c.version)).collect()
    };
    let warn = ui.t("msgDlgDeleteLinkedWarning");
    let created = ui.colour("SongStateCreated");
    // (icon colour, the confirm button's label, its accent)
    let (icon_colour, yes, accent) = match &act {
        Act::DeleteCard(_) => ("var(--uis-danger)".to_owned(), ui.t("btnCardDelete"), "var(--uis-danger)".to_owned()),
        Act::DeleteSelected => ("var(--uis-danger)".to_owned(), ui.t("btnDlgDeleteAll"), "var(--uis-danger)".to_owned()),
        Act::UnlinkSelected => ("var(--uis-warn)".to_owned(), ui.t("btnHdrUnlink"), "var(--uis-warn)".to_owned()),
        Act::ImportFolder { .. } => (created.clone(), ui.t("btnSbrImportConfirm"), created.clone()),
    };
    let sub = if matches!(act, Act::UnlinkSelected) { format!("{sub} — {} {}", ui.selected_ids().len(), ui.t("lblHdrSelected")) } else { sub };
    let btn_style = format!("--uis-accent: {accent}; --uis-accent-hov: color-mix(in srgb, {accent} 82%, black);");
    let act_icon = act.clone();
    rsx! {
        div { class: "uis-dl-backdrop", onmousedown: move |_| ui.answer(false),
            div { class: "uis-dlp", onmousedown: move |e| e.stop_propagation(),
                button { class: "uis-dl-close", onclick: move |_| ui.answer(false), Icon { width: 14, height: 14, icon: LdX } }
                match &act_icon {
                    Act::UnlinkSelected => rsx! { Icon { class: "uis-dlp-icon", icon: LdUnlink, style: "color: {icon_colour}" } },
                    Act::ImportFolder { .. } => rsx! { Icon { class: "uis-dlp-icon", icon: LdFolderOpen, style: "color: {icon_colour}" } },
                    _ => rsx! { Icon { class: "uis-dlp-icon", icon: LdTrash2, style: "color: {icon_colour}" } },
                }
                p { class: "uis-dlp-title", "{title}" }
                if !orphans.is_empty() {
                    div { style: "margin-bottom: 12px; text-align: left; width: 100%;",
                        p { class: "uis-dlp-sub", style: "margin-bottom: 6px; color: var(--uis-danger);", "⚠ {warn}" }
                        div { style: "max-height: 180px; overflow-y: auto; display: flex; flex-direction: column; gap: 3px;",
                            for (id , name , version) in orphans {
                                span { key: "{id}", style: "font-size: 12px; color: var(--uis-text); padding: 2px 0;",
                                    "• {name}"
                                    span { style: "font-size: 10px; font-weight: 700; opacity: 0.6; margin-left: 4px;", "v{version}" }
                                }
                            }
                        }
                    }
                } else {
                    p { class: "uis-dlp-sub", "{sub}" }
                }
                div { class: "uis-dlp-foot",
                    button {
                        class: "uis-btn subtle",
                        onmounted: move |e| async move {
                            let _ = e.set_focus(true).await;
                        },
                        onclick: move |_| ui.answer(false),
                        "{cancel}"
                    }
                    button { class: "uis-btn primary", style: "{btn_style}", onclick: move |_| ui.answer(true),
                        match &act_icon {
                            Act::UnlinkSelected => rsx! { Icon { width: 14, height: 14, icon: LdUnlink, style: "margin-right: 6px" } },
                            Act::ImportFolder { .. } => rsx! { Icon { width: 14, height: 14, icon: LdMusic2, style: "margin-right: 6px" } },
                            _ => rsx! { Icon { width: 14, height: 14, icon: LdTrash2, style: "margin-right: 6px" } },
                        }
                        "{yes}"
                    }
                }
            }
        }
    }
}

#[component]
fn InfoDlg(title: String, lines: Vec<String>) -> Element {
    let ui = use_context::<Ui>();
    let close = ui.t("btnNewsClose");
    rsx! {
        Dlg {
            title,
            foot: rsx! {
                button { class: "uis-btn primary", onclick: move |_| ui.close(), "{close}" }
            },
            div { class: "ap-lines",
                for line in lines {
                    div { "{line}" }
                }
            }
        }
    }
}

// ─── Project ────────────────────────────────────────────────────────────────────────────────────

/// The colour a new project starts with: the apps' accent.
const PROJECT_COLOUR: &str = "#5a4fff";

#[component]
fn ProjectDlg(id: Option<String>, parent: Option<String>) -> Element {
    let ui = use_context::<Ui>();
    let existing = id.as_ref().and_then(|i| ui.projects.read().iter().find(|p| &p.id == i).cloned());
    let date_start = existing.as_ref().map(|p| engine::dates::ymd(p.date)).unwrap_or_default();
    let (n0, c0, p0, f0, u0) = existing
        .as_ref()
        .map(|p| (p.name.clone(), p.color.clone(), p.path.clone(), p.finalized, p.urls.iter().map(|u| (u.label.clone(), u.href.clone())).collect::<Vec<_>>()))
        .unwrap_or((String::new(), PROJECT_COLOUR.to_owned(), String::new(), false, Vec::new()));
    let (mut name, mut colour, mut path, mut finalized) = (use_signal(move || n0.clone()), use_signal(move || c0.clone()), use_signal(move || p0.clone()), use_signal(move || f0));
    let mut urls = use_signal(move || u0.clone());
    let (mut url_label, mut url_href) = (use_signal(String::new), use_signal(String::new));
    let mut name_err = use_signal(|| false);
    let mut date_val = use_signal({
        let v = date_start.clone();
        move || v.clone()
    });
    let title = match (&id, &parent) {
        (Some(_), _) => ui.t("ttlDlgProjectEdit"),
        (None, Some(_)) => ui.t("ttlDlgProjectNewSub"),
        _ => ui.t("ttlDlgProjectNew"),
    };
    let (lbl_name, lbl_colour, lbl_path, lbl_fin, tip_fin, plh_name, plh_path) =
        (ui.t("lblDlgProjectName"), ui.t("lblDlgProjectColor"), ui.t("lblDlgProjectPath"), ui.t("lblDlgProjectFinalized"), ui.t("tipDlgProjectFinalized"), ui.t("plhDlgProjectName"), ui.t("plhDlgProjectPath"));
    let (cancel, apply, pick, optional, l_links, l_today, l_clear, l_required) =
        (ui.t("btnGlbCancel"), ui.t("btnDlgCardApply"), ui.t("tipGlbPickFromScreen"), ui.t("msgDlgCardOptional"), ui.t("lblDlgCardExternalLinks"), ui.t("btnGlbToday"), ui.t("btnGlbClear"), ui.t("msgDlgProjectNameRequired"));
    let save_label = if id.is_some() { ui.t("btnDlgProjectSave") } else { ui.t("btnDlgProjectCreate") };
    let version = existing.as_ref().map(|p| p.version.max(1));
    let accent = colour();
    let id2 = id.clone();
    rsx! {
        Dlg {
            title,
            foot: rsx! {
                if let Some(v) = version {
                    div { class: "ap-foot-hint",
                        UisPkrDate { value: date_val(), on_change: move |v: String| date_val.set(v), locale: ui.lang.peek().clone(), today_label: l_today.clone(), clear_label: l_clear.clone() }
                        span { class: "ap-version", style: "color:{accent}", "v{v}" }
                        button { class: "uis-btn small subtle", title: "{tip_fin}", onclick: move |_| finalized.toggle(),
                            if finalized() {
                                Icon { width: 14, height: 14, icon: LdSquareCheckBig }
                            } else {
                                Icon { width: 14, height: 14, icon: LdSquare }
                            }
                            span { "{lbl_fin}" }
                        }
                    }
                }
                button { class: "uis-btn subtle", onclick: move |_| ui.close(), "{cancel}" }
                button {
                    class: "uis-btn primary",
                    style: "--uis-accent:{accent};--uis-accent-hov:{accent}",
                    onclick: move |_| {
                        let nm = name().trim().to_owned();
                        if nm.is_empty() {
                            name_err.set(true);
                            return;
                        }
                        let st = ui.st();
                        let want = urls();
                        let day = if date_val() != date_start { engine::dates::from_ymd(&date_val()) } else { None };
                        let result = match &id2 {
                            Some(i) => st.update_project(i, &ProjectPatch { name: Some(nm), color: Some(colour()), path: Some(path()), finalized: Some(finalized()), date: day, ..Default::default() }).map(|_| {
                                // links: what was removed goes, what is new comes
                                if let Ok(Some(p)) = st.project(i) {
                                    for u in p.urls.iter().filter(|u| !want.iter().any(|(l, h)| *l == u.label && *h == u.href)) {
                                        let _ = st.delete_project_url(i, &u.id);
                                    }
                                    for (l, h) in want.iter().filter(|(l, h)| !p.urls.iter().any(|u| u.label == *l && u.href == *h)) {
                                        let _ = st.add_project_url(i, l, h);
                                    }
                                }
                                ui.t("tstAppProjectUpdated")
                            }),
                            None => st
                                .create_project(&NewProject { id: new_id(), name: nm, color: Some(colour()), parent_id: parent.clone(), env: Some(ui.env.peek().clone()), path: path(), urls: want, finalized: finalized(), ..Default::default() })
                                .map(|p| {
                                    let mut nav = ui.nav;
                                    if let Some(parent_id) = p.parent_id.clone() {
                                        let mut expanded = ui.expanded;
                                        expanded.write().insert(parent_id);
                                    }
                                    nav.set(Nav::Project(p.id));
                                    ui.t("tstAppProjectCreated")
                                }),
                        };
                        match result {
                            Ok(msg) => {
                                ui.reload();
                                ui.close();
                                ui.ok(msg);
                            }
                            Err(e) => ui.fail(e),
                        }
                    },
                    "{save_label}"
                }
            },
            Field { label: lbl_name,
                input {
                    class: if name_err() { "uis-txt ap-input-err" } else { "uis-txt" },
                    placeholder: "{plh_name}",
                    value: "{name}",
                    onmounted: move |e| async move {
                        let _ = e.set_focus(true).await;
                    },
                    oninput: move |e| {
                        name.set(e.value());
                        name_err.set(false);
                    },
                }
                if name_err() {
                    span { class: "ap-err", "{l_required}" }
                }
            }
            Field { label: lbl_colour,
                UisPkrColor { color: colour(), on_change: move |c: String| colour.set(c), cancel_label: cancel.clone(), apply_label: apply.clone(), pick_title: pick.clone() }
            }
            Field { label: format!("{l_links} ({optional})"),
                div { class: "ap-urls",
                    for (i , (label , href)) in urls().into_iter().enumerate() {
                        div { key: "{i}", class: "ap-row",
                            span { class: "ap-chip", "{label}" }
                            span { class: "ap-url", "{href}" }
                            button { class: "uis-btn ico small ap-danger", onclick: move |_| { urls.write().remove(i); }, Icon { width: 12, height: 12, icon: LdTrash2 } }
                        }
                    }
                    div { class: "ap-row",
                        input { class: "uis-txt ap-url-label", placeholder: "{ui.t(\"plhDlgCardUrlLabel\")}", value: "{url_label}", oninput: move |e| url_label.set(e.value()) }
                        input {
                            class: "uis-txt ap-grow",
                            placeholder: "https://",
                            value: "{url_href}",
                            oninput: move |e| url_href.set(e.value()),
                            onkeydown: move |e| {
                                if e.key() == Key::Enter && !url_href.peek().trim().is_empty() {
                                    add_project_link(&mut urls, &mut url_label, &mut url_href);
                                }
                            },
                        }
                        button {
                            class: "uis-btn small primary",
                            style: "--uis-accent:{accent};--uis-accent-hov:{accent}",
                            disabled: url_href().trim().is_empty(),
                            onclick: move |_| add_project_link(&mut urls, &mut url_label, &mut url_href),
                            "{ui.t(\"btnDlgCardAdd\")}"
                        }
                    }
                }
            }
            Field { label: lbl_path,
                div { class: "ap-row",
                    input { class: "uis-txt ap-grow", placeholder: "{plh_path}", value: "{path}", oninput: move |e| path.set(e.value()) }
                    button {
                        class: "uis-btn subtle",
                        onclick: move |_| {
                            spawn(async move {
                                if let Some(dir) = rfd::AsyncFileDialog::new().set_parent(&dioxus::desktop::window().window).pick_folder().await {
                                    path.set(dir.path().to_string_lossy().into_owned());
                                }
                            });
                        },
                        Icon { width: 14, height: 14, icon: LdFolderOpen }
                    }
                }
            }
        }
    }
}

/// Take what was typed in the link line of the project dialog: an address without a scheme gets `https://`, a missing label is made from the site.
fn add_project_link(urls: &mut Signal<Vec<(String, String)>>, label: &mut Signal<String>, href: &mut Signal<String>) {
    let typed = href.peek().trim().to_owned();
    if typed.is_empty() {
        return;
    }
    let address = if typed.starts_with("http") { typed } else { format!("https://{typed}") };
    let name = if label.peek().trim().is_empty() { engine::transfer::song_url_label(&address) } else { label.peek().trim().to_owned() };
    urls.write().push((name, address));
    label.set(String::new());
    href.set(String::new());
}

#[component]
fn DeleteProjectDlg(id: String) -> Element {
    let ui = use_context::<Ui>();
    let mut with_cards = use_signal(|| false);
    let (title, lbl, cancel, del) = (ui.t("cfmDlgDeleteProjectTitle"), ui.t("lblDlgDeleteProjectCards"), ui.t("btnGlbCancel"), ui.t("btnCardDelete"));
    // Electron's `deleteProjectConfirm`: the question dialog, with the checkbox in its body
    rsx! {
        div { class: "uis-dl-backdrop", onmousedown: move |_| ui.close(),
            div { class: "uis-dlp", onmousedown: move |e| e.stop_propagation(),
                button { class: "uis-dl-close", onclick: move |_| ui.close(), Icon { width: 14, height: 14, icon: LdX } }
                Icon { class: "uis-dlp-icon", icon: LdTrash2, style: "color: var(--uis-danger)" }
                p { class: "uis-dlp-title", "{title}" }
                div { class: "uis-dlp-body",
                    label { class: "uis-chk-lbl",
                        input { r#type: "checkbox", class: "uis-chk-box", checked: with_cards(), onchange: move |e| with_cards.set(e.checked()) }
                        span { "{lbl}" }
                    }
                }
                div { class: "uis-dlp-foot",
                    button { class: "uis-btn subtle", onclick: move |_| ui.close(), "{cancel}" }
                    button {
                        class: "uis-btn primary",
                        style: "--uis-accent: var(--uis-danger); --uis-accent-hov: color-mix(in srgb, var(--uis-danger) 82%, black);",
                        onclick: {
                            let id = id.clone();
                            move |_| {
                                ui.close();
                                ui.delete_project(&id, with_cards());
                            }
                        },
                        Icon { width: 14, height: 14, icon: LdTrash2, style: "margin-right: 6px" }
                        "{del}"
                    }
                }
            }
        }
    }
}

// ─── Tags, moving, songbook ─────────────────────────────────────────────────────────────────────

#[component]
fn TagDlg(ids: Vec<String>, scope: String) -> Element {
    let ui = use_context::<Ui>();
    let mut input = use_signal(String::new);
    let (title, plh, add, rem, quick, tip_chip) = (ui.t("ttlDlgTags"), ui.t("plhDlgTagsInput"), ui.t("btnDlgTagsAdd"), ui.t("btnDlgTagsRemove"), ui.t("lblDlgTagsQuickRemove"), ui.t("tipDlgTagsRemoveChip"));
    let all_tags: Vec<String> = ui.cards.read().iter().flat_map(|c| c.tags.clone()).collect::<std::collections::BTreeSet<_>>().into_iter().collect();
    let targets: Vec<Card> = ids.iter().filter_map(|id| ui.card(id)).collect();
    let on_all: Vec<String> = all_tags.iter().filter(|t| !targets.is_empty() && targets.iter().all(|c| c.tags.contains(t))).cloned().collect();
    let on_some: Vec<String> = all_tags.iter().filter(|t| targets.iter().any(|c| c.tags.contains(t)) && !on_all.contains(t)).cloned().collect();
    // the word being typed after the last comma, and the tags that complete it
    let typed = input();
    let parts: Vec<String> = typed.split(',').map(|s| s.trim().to_lowercase()).collect();
    let current = parts.last().cloned().unwrap_or_default();
    let entered: Vec<String> = parts[..parts.len().saturating_sub(1)].iter().filter(|s| !s.is_empty()).cloned().collect();
    let suggestions: Vec<String> = if current.is_empty() { Vec::new() } else { all_tags.iter().filter(|t| t.contains(&current) && **t != current && !entered.contains(t)).cloned().collect() };
    let items: Vec<UisComboItem> = suggestions.iter().map(|t| UisComboItem { key: t.clone(), label: t.clone(), active: false, header: false }).collect();
    let picks = suggestions.clone();
    let act = {
        let ids = ids.clone();
        move |adding: bool| {
            let tags: Vec<String> = input.peek().split(',').map(|t| t.trim().to_lowercase()).filter(|t| !t.is_empty()).collect();
            if tags.is_empty() {
                return;
            }
            let (to_add, to_remove) = if adding { (tags.clone(), Vec::new()) } else { (Vec::new(), tags.clone()) };
            let n = ui.tag_cards(&ids, &to_add, &to_remove);
            if n == 0 {
                ui.ok(if adding { ui.t("tstDlgTagsPresent") } else { ui.t("tstDlgTagsAbsent") });
            } else {
                let label = if tags.len() == 1 { format!("\"{}\"", tags[0]) } else { format!("{} tags", tags.len()) };
                let text = if adding { ui.t("tstDlgTagsAdded") } else { ui.t("tstDlgTagsRemoved") };
                ui.ok(text.replace("{tags}", &label).replace("{n}", &n.to_string()));
            }
            input.set(String::new());
        }
    };
    let (mut act_add, mut act_rem, mut act_enter) = (act.clone(), act.clone(), act.clone());
    let ids_chip = ids.clone();
    rsx! {
        div { class: "uis-dl-backdrop", onmousedown: move |_| ui.close(),
            div { class: "uis-dlp", style: "max-width:340px", onmousedown: move |e| e.stop_propagation(),
                button { class: "uis-dl-close", onclick: move |_| ui.close(), Icon { width: 14, height: 14, icon: LdX } }
                span { class: "uis-dlp-icon", Icon { width: 28, height: 28, icon: LdTag } }
                p { class: "uis-dlp-title", "{title} ({scope})" }
                div { class: "uis-dlp-body",
                    div {
                        style: "width:100%",
                        onkeydown: move |e| {
                            if e.key() == Key::Enter && !input.peek().trim().is_empty() {
                                act_enter(true);
                            }
                        },
                        UisCboStandard {
                            value: typed.clone(),
                            auto_focus: true,
                            placeholder: plh.clone(),
                            items,
                            on_change: move |v: String| input.set(v),
                            on_pick: move |i: usize| {
                                if let Some(tag) = picks.get(i) {
                                    let mut keep: Vec<String> = input.peek().split(',').map(|s| s.trim().to_owned()).collect();
                                    keep.pop();
                                    keep.retain(|s| !s.is_empty());
                                    keep.push(tag.clone());
                                    input.set(format!("{}, ", keep.join(", ")));
                                }
                            },
                        }
                    }
                    if !on_all.is_empty() || !on_some.is_empty() {
                        div { class: "ap-chip-row",
                            span { class: "uis-hnt", "{quick}" }
                            for (tag , all) in on_all.iter().map(|t| (t.clone(), true)).chain(on_some.iter().map(|t| (t.clone(), false))) {
                                button {
                                    key: "{tag}",
                                    class: if all { "uis-chp on" } else { "uis-chp" },
                                    title: "{tip_chip}",
                                    onclick: {
                                        let (ids, tag) = (ids_chip.clone(), tag.clone());
                                        move |_| {
                                            ui.tag_cards(&ids, &[], std::slice::from_ref(&tag));
                                        }
                                    },
                                    "{tag}"
                                    Icon { width: 12, height: 12, icon: LdX }
                                }
                            }
                        }
                    }
                }
                div { class: "uis-dlp-foot",
                    button { class: "uis-btn primary ap-btn-ok", disabled: typed.trim().is_empty(), onclick: move |_| act_add(true),
                        Icon { width: 14, height: 14, icon: LdPlus }
                        " {add}"
                    }
                    button { class: "uis-btn primary ap-btn-danger", disabled: typed.trim().is_empty(), onclick: move |_| act_rem(false),
                        Icon { width: 14, height: 14, icon: LdX }
                        " {rem}"
                    }
                }
            }
        }
    }
}

/// The projects of one environment, depth first, each with its depth.
fn folder_tree(all: &[Project], parent: &str, depth: usize, out: &mut Vec<(Project, usize)>) {
    let mut kids: Vec<&Project> = all.iter().filter(|p| p.parent_id.clone().unwrap_or_default() == parent).collect();
    kids.sort_by_key(|p| p.name.to_lowercase());
    for p in kids {
        out.push((p.clone(), depth));
        folder_tree(all, &p.id, depth + 1, out);
    }
}

/// Moving cards to another environment (`folder = None`) or a whole project with its sub-projects (`folder = Some(id)`).
#[component]
fn MoveEnvDlg(ids: Vec<String>, folder: Option<String>) -> Element {
    let ui = use_context::<Ui>();
    let mut target = use_signal(String::new);
    let mut project = use_signal(String::new);
    let mut warning = use_signal(|| None::<Vec<String>>);
    let current = ui.env.read().clone();
    let envs: Vec<_> = ui.envs.read().iter().filter(|e| e.id != current && !e.is_global).cloned().collect();
    let is_cards = folder.is_none();
    let mut tree: Vec<(Project, usize)> = Vec::new();
    if is_cards && !target().is_empty() {
        let all: Vec<Project> = ui.projects.read().iter().filter(|p| p.env == target()).cloned().collect();
        folder_tree(&all, "", 0, &mut tree);
    }
    let (title, sel, lbl_project, root, mv, warn_head, warn_text, force, cancel) = (
        ui.t("ttlDlgMoveEnv"),
        ui.t("optDlgMoveEnvSelect"),
        ui.t("lblDlgMoveEnvProject"),
        ui.t("optDlgMoveEnvRoot"),
        ui.t("btnDlgMoveEnvMove"),
        ui.t("msgDlgMoveEnvUnlink"),
        ui.t("msgDlgMoveEnvMergeWarning"),
        ui.t("btnDlgMoveEnvForce"),
        ui.t("btnGlbCancel"),
    );
    let heading = if is_cards { format!("{title} ({})", ids.len()) } else { title.clone() };
    let go = {
        let (ids, folder) = (ids.clone(), folder.clone());
        move |confirmed: bool| {
            let env = target();
            if env.is_empty() {
                return;
            }
            if let Some(f) = &folder {
                match ui.st().move_folder(f, &env) {
                    Ok(_) => {
                        ui.reload();
                        ui.close();
                        ui.ok(ui.t("tstAppFolderMoved"));
                    }
                    Err(e) => ui.fail(e),
                }
                return;
            }
            // Moving a music / vocal card whose songs stay behind unlinks them: say so first.
            if !confirmed {
                let has_song = ids.iter().filter_map(|id| ui.card(id)).any(|c| c.kind == "song");
                if !has_song {
                    let affected: Vec<String> = ui.env_cards().iter().filter(|s| s.kind == "song" && ids.iter().any(|id| s.linked_cards.contains(id))).map(|s| s.name.clone()).collect();
                    if !affected.is_empty() {
                        warning.set(Some(affected));
                        return;
                    }
                }
            }
            warning.set(None);
            let p = project();
            match ui.st().move_cards(&ids, &env, if p.is_empty() { None } else { Some(p.as_str()) }) {
                Ok(m) => {
                    let mut selected = ui.selected;
                    selected.set(Default::default());
                    ui.reload();
                    ui.close();
                    let mut msg = ui.t("tstAppCardMoved").replace("{n}", &m.moved_ids.len().to_string());
                    if m.linked_moved_count + m.duplicated_ids.len() > 0 {
                        msg.push_str(" · ");
                        msg.push_str(&ui.t("tstAppMoveLinkedInfo").replace("{n}", &m.linked_moved_count.to_string()).replace("{d}", &m.duplicated_ids.len().to_string()).replace("{s}", "0"));
                    }
                    ui.ok(msg);
                }
                Err(e) => ui.fail(e),
            }
        }
    };
    let (mut go_main, mut go_force) = (go.clone(), go.clone());
    let head = warning().map(|n| warn_head.replace("{n}", &n.len().to_string())).unwrap_or_default();
    rsx! {
        div { class: "uis-dl-backdrop", onmousedown: move |_| ui.close(),
            div { class: "uis-dlp", style: if is_cards { "max-width:380px" } else { "max-width:320px" }, onmousedown: move |e| e.stop_propagation(),
                button { class: "uis-dl-close", onclick: move |_| ui.close(), Icon { width: 14, height: 14, icon: LdX } }
                span { class: "uis-dlp-icon", Icon { width: 28, height: 28, icon: LdArrowUpDown } }
                p { class: "uis-dlp-title", "{heading}" }
                div { class: "uis-dlp-body",
                    select {
                        class: "uis-ddn-std",
                        value: "{target}",
                        onchange: move |e| {
                            target.set(e.value());
                            project.set(String::new());
                            warning.set(None);
                        },
                        option { value: "", "{sel}" }
                        for e in envs {
                            option { key: "{e.id}", value: "{e.id}", "{e.name}" }
                        }
                    }
                    if is_cards && !target().is_empty() {
                        div { class: "ap-line",
                            span { class: "uis-fld-labelled-label", "{lbl_project}" }
                            select { class: "uis-ddn-std", value: "{project}", onchange: move |e| project.set(e.value()),
                                option { value: "", "📁 {root}" }
                                for (p , depth) in tree {
                                    option { key: "{p.id}", value: "{p.id}", "{folder_label(&p.name, depth)}" }
                                }
                            }
                        }
                    }
                }
                if let Some(names) = warning() {
                    div { class: "ap-warn-box",
                        span { class: "ap-warn-head", "⚠ {head}" }
                        span { class: "uis-hnt", "{names.join(\", \")}" }
                        span { class: "uis-hnt", "{warn_text}" }
                        div { class: "ap-line",
                            button { class: "uis-btn primary ap-btn-warn", onclick: move |_| go_force(true),
                                Icon { width: 14, height: 14, icon: LdArrowUpDown }
                                " {force}"
                            }
                            button { class: "uis-btn subtle", onclick: move |_| warning.set(None),
                                Icon { width: 14, height: 14, icon: LdX }
                                " {cancel}"
                            }
                        }
                    }
                }
                div { class: "uis-dlp-foot",
                    button { class: "uis-btn primary", disabled: target().is_empty(), onclick: move |_| go_main(false),
                        Icon { width: 14, height: 14, icon: LdArrowUpDown }
                        " {mv}"
                    }
                }
            }
        }
    }
}

/// A folder in a list of folders: indented, with an arrow below the first level.
fn folder_label(name: &str, depth: usize) -> String {
    format!("{}{}{name}", "\u{a0}\u{a0}".repeat(depth), if depth > 0 { "↳ " } else { "📁 " })
}

/// What the Create document dialog remembers (`doc-create-settings`, the Electron app's JSON: the same keys and values).
#[derive(Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct DocChoices {
    doc_format: String,
    content_lyrics: String,
    content_images: String,
    content_desc: String,
    filter_lyrics: String,
    filter_published: String,
    content_langs: Vec<String>,
}

impl Default for DocChoices {
    fn default() -> Self {
        Self {
            doc_format: "docx".to_owned(),
            content_lyrics: "clean".to_owned(),
            content_images: "with".to_owned(),
            content_desc: "with".to_owned(),
            filter_lyrics: "withLyrics".to_owned(),
            filter_published: "all".to_owned(),
            content_langs: Vec::new(),
        }
    }
}

/// A welded row of choices, each button taking an equal share; the chosen one is active.
#[component]
fn Choice(options: Vec<(String, String)>, value: String, on_pick: EventHandler<String>) -> Element {
    rsx! {
        div { class: "uis-barh-grp",
            for (key , label) in options {
                {
                    let k = key.clone();
                    rsx! {
                        button { key: "{key}", class: if value == key { "uis-btn ap-grow active" } else { "uis-btn ap-grow" }, onclick: move |_| on_pick.call(k.clone()), "{label}" }
                    }
                }
            }
        }
    }
}

#[component]
fn DocDlg() -> Element {
    let ui = use_context::<Ui>();
    let mut choices = use_signal(|| serde_json::from_str::<DocChoices>(&ui.st().setting("doc-create-settings")).unwrap_or_default());
    use_effect(move || ui.st().set_setting("doc-create-settings", &serde_json::to_string(&choices()).unwrap_or_default()));
    let project_ids: Vec<String> = match ui.nav.peek().clone() {
        Nav::Project(p) => ui.project_tree(&p),
        _ => Vec::new(),
    };
    let available = ui.st().translation_languages(&project_ids).unwrap_or_default();
    let ui_lang = ui.lang.peek().clone();
    let t = |k: &str| ui.t(k);
    let (title, create, cancel) = (t("ttlDlgDoc"), t("btnDlgDocCreate"), t("btnGlbCancel"));
    let (l_fmt, l_img, l_desc, l_pub, l_lyr, l_content, l_tr) = (
        t("lblDlgDocFormat"),
        t("lblDlgDocContentImages"),
        t("lblDlgDocContentDesc"),
        t("lblDlgDocFilterPublished"),
        t("lblDlgDocFilterLyrics"),
        t("lblDlgDocContentLyrics"),
        t("lblDlgDocContentLangs"),
    );
    let (o_orig, o_clean, o_pub, o_all, o_with) = (t("btnDlgDocContentLyricsOriginal"), t("btnDlgDocContentLyricsCleaned"), t("btnDlgDocFilterPublishedOnly"), t("btnDlgDocFilterAll"), t("btnDlgDocFilterWithLyrics"));
    let (w, wo, hint) = (t("btnDlgDocContentWith"), t("btnDlgDocContentWithout"), t("msgDlgDocSongCount"));
    let (d_orig, d_clean) = (t("msgDlgDocContentLyricsOriginalDesc"), t("msgDlgDocContentLyricsCleanedDesc"));
    let c = choices();
    let options = DocOptions {
        project_ids: project_ids.clone(),
        clean_lyrics: c.content_lyrics != "original",
        langs: c.content_langs.clone(),
        images: c.content_images != "without",
        desc: c.content_desc != "without",
        with_lyrics_only: c.filter_lyrics != "all",
        published_only: c.filter_published == "publishedOnly",
        ui_lang: ui_lang.clone(),
        translation_label: ui.t("lblDlgCardTranslation"),
        instrumental_label: ui.t("lblDlgDocLyricsInstrumental"),
    };
    let count = ui.st().doc_song_count(&options);
    let format = match c.doc_format.as_str() {
        "md" => exchange::DocFormat::Md,
        "adoc" => exchange::DocFormat::Adoc,
        _ => exchange::DocFormat::Docx,
    };
    rsx! {
        Dlg {
            title,
            foot: rsx! {
                span { class: if count == 0 { "uis-hnt warn ap-grow" } else { "uis-hnt ap-grow" }, "{count} {hint}" }
                button { class: "uis-btn subtle", onclick: move |_| ui.close(), "{cancel}" }
                button {
                    class: "uis-btn primary",
                    disabled: count == 0,
                    onclick: move |_| {
                        ui.close();
                        exchange::export_doc(ui, options.clone(), format);
                    },
                    "{create}"
                }
            },
            Field { label: l_fmt,
                Choice {
                    options: vec![("docx".to_owned(), "DOCX".to_owned()), ("md".to_owned(), "Markdown".to_owned()), ("adoc".to_owned(), "AsciiDoc".to_owned())],
                    value: c.doc_format.clone(),
                    on_pick: move |v| choices.write().doc_format = v,
                }
            }
            Field { label: l_img,
                Choice { options: vec![("with".to_owned(), w.clone()), ("without".to_owned(), wo.clone())], value: c.content_images.clone(), on_pick: move |v| choices.write().content_images = v }
            }
            Field { label: l_desc,
                Choice { options: vec![("with".to_owned(), w), ("without".to_owned(), wo)], value: c.content_desc.clone(), on_pick: move |v| choices.write().content_desc = v }
            }
            Field { label: l_pub,
                Choice {
                    options: vec![("publishedOnly".to_owned(), o_pub), ("all".to_owned(), o_all.clone())],
                    value: c.filter_published.clone(),
                    on_pick: move |v| choices.write().filter_published = v,
                }
            }
            Field { label: l_lyr,
                Choice { options: vec![("withLyrics".to_owned(), o_with), ("all".to_owned(), o_all)], value: c.filter_lyrics.clone(), on_pick: move |v| choices.write().filter_lyrics = v }
            }
            Field { label: l_content,
                div { title: if c.content_lyrics == "original" { "{d_orig}" } else { "{d_clean}" },
                    Choice { options: vec![("original".to_owned(), o_orig), ("clean".to_owned(), o_clean)], value: c.content_lyrics.clone(), on_pick: move |v| choices.write().content_lyrics = v }
                }
            }
            if !available.is_empty() {
                Field { label: l_tr,
                    div { class: "ap-tags",
                        for code in available {
                            {
                                let on = c.content_langs.contains(&code);
                                let name = langs::lang_name(&code, &ui_lang);
                                let code2 = code.clone();
                                rsx! {
                                    span {
                                        key: "{code}",
                                        class: if on { "ap-tag ap-tag-on" } else { "ap-tag" },
                                        onclick: move |_| {
                                            let mut set = choices.write();
                                            if let Some(i) = set.content_langs.iter().position(|x| *x == code2) {
                                                set.content_langs.remove(i);
                                            } else {
                                                set.content_langs.push(code2.clone());
                                            }
                                        },
                                        "{name}"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn AddToProjectDlg(ids: Vec<String>) -> Element {
    let ui = use_context::<Ui>();
    let mut project = use_signal(String::new);
    let projects = ui.env_projects();
    let (title, sel) = (ui.t("tipSbrAddToProject"), ui.t("optDlgMoveEnvSelect"));
    rsx! {
        Dlg {
            title: title.clone(),
            foot: rsx! {
                button {
                    class: "uis-btn primary",
                    disabled: project().is_empty(),
                    onclick: {
                        let ids = ids.clone();
                        move |_| {
                            let p = project();
                            for id in &ids {
                                if let Err(e) = ui.st().add_card_to_project(&p, id) {
                                    ui.fail(e);
                                    return;
                                }
                            }
                            ui.reload();
                            ui.close();
                            ui.ok(ui.t("tstAppAddedToProject"));
                        }
                    },
                    "{title}"
                }
            },
            select { class: "uis-ddn-std", value: "{project}", onchange: move |e| project.set(e.value()),
                option { value: "", "{sel}" }
                for p in projects {
                    option { key: "{p.id}", value: "{p.id}", "{p.name}" }
                }
            }
        }
    }
}
