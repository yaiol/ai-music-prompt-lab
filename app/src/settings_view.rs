// ⚠ NO HARDCODED UI STRINGS IN THIS FILE.
//   Every string a user can read MUST go through the t() lookup, and the key MUST be a string literal at the call site.

//! The settings pages of this app: Cards (which sites mean "created" and "published"), Media (the tag, the sort
//! default, the stored paths and their audit) and AI (the key, the limits).

use crate::{
    catalog::{UisCboStandard, UisComboItem, UisNum, UisPkrColor},
    ui::{Dialog, Ui},
};
use dioxus::prelude::*;
use dioxus_free_icons::{Icon, icons::ld_icons::{LdFolderOpen, LdMic, LdMusic, LdMusic2, LdSparkles, LdStickyNote, LdTag, LdX}};
use engine::extras::MediaRefused;

/// A line of text kept in a setting as the user types.
#[component]
fn TextSetting(name: String, label: String, #[props(default)] note: String, #[props(default)] secret: bool, #[props(default)] placeholder: String) -> Element {
    let ui = use_context::<Ui>();
    let key = name.clone();
    let mut value = use_signal({
        let v = ui.st().setting(&name);
        move || v.clone()
    });
    rsx! {
        div { class: "uis-fld-labelled",
            label { class: "uis-fld-labelled-label", "{label}" }
            input {
                class: "uis-txt",
                r#type: if secret { "password" } else { "text" },
                placeholder: "{placeholder}",
                value: "{value}",
                oninput: move |e| {
                    value.set(e.value());
                    ui.st().set_setting(&key, &e.value());
                },
            }
            if !note.is_empty() {
                div { class: "ap-hint", "{note}" }
            }
        }
    }
}

#[component]
fn NumSetting(name: String, label: String, default: i64, #[props(default)] note: String) -> Element {
    let ui = use_context::<Ui>();
    let key = name.clone();
    let mut value = use_signal({
        let v = ui.st().setting(&name).parse::<i64>().unwrap_or(default);
        move || v
    });
    rsx! {
        div { class: "uis-fld-labelled",
            label { class: "uis-fld-labelled-label", "{label}" }
            UisNum {
                value: Some(value() as f64),
                width: "100px",
                min: Some(0.0),
                decimals: Some(0),
                on_change: move |v: Option<f64>| {
                    if let Some(v) = v {
                        value.set(v.round() as i64);
                        ui.st().set_setting(&key, &(v.round() as i64).to_string());
                    }
                },
            }
            if !note.is_empty() {
                div { class: "ap-hint", "{note}" }
            }
        }
    }
}

#[component]
fn FlagSetting(name: String, label: String, #[props(default)] note: String, on_value: String) -> Element {
    let ui = use_context::<Ui>();
    let key = name.clone();
    let on = on_value.clone();
    let mut value = use_signal({
        let v = ui.st().setting(&name) == on_value;
        move || v
    });
    rsx! {
        div { class: "uis-fld-labelled",
            label { class: "uis-chk-lbl",
                input {
                    r#type: "checkbox",
                    class: "uis-chk-box",
                    checked: value(),
                    onchange: move |e| {
                        value.set(e.checked());
                        ui.st().set_setting(&key, if e.checked() { &on } else { "0" });
                    },
                }
                "{label}"
            }
            if !note.is_empty() {
                div { class: "ap-hint", "{note}" }
            }
        }
    }
}

/// A number kept in a setting, in the field and with the bounds the Electron app gives it.
#[component]
fn LimitField(name: String, label: String, default: i64, min: f64, max: f64, step: f64) -> Element {
    let ui = use_context::<Ui>();
    let key = name.clone();
    let mut value = use_signal({
        let v = ui.st().setting(&name).parse::<i64>().unwrap_or(default);
        move || v
    });
    rsx! {
        div { class: "ap-limit",
            label { class: "uis-hnt", "{label}" }
            UisNum {
                value: Some(value() as f64),
                width: "90px",
                min: Some(min),
                max: Some(max),
                step,
                decimals: Some(0),
                on_change: move |v: Option<f64>| {
                    if let Some(v) = v {
                        value.set(v.round() as i64);
                        ui.st().set_setting(&key, &(v.round() as i64).to_string());
                    }
                },
            }
        }
    }
}

/// A number with its explanation beside it.
#[component]
fn ThresholdField(name: String, label: String, note: String, default: i64, min: f64, max: f64, step: f64) -> Element {
    let ui = use_context::<Ui>();
    let key = name.clone();
    let mut value = use_signal({
        let v = ui.st().setting(&name).parse::<i64>().unwrap_or(default);
        move || v
    });
    rsx! {
        div { class: "uis-fld-labelled",
            label { class: "uis-fld-labelled-label", "{label}" }
            div { class: "ap-row",
                UisNum {
                    value: Some(value() as f64),
                    width: "90px",
                    min: Some(min),
                    max: Some(max),
                    step,
                    decimals: Some(0),
                    on_change: move |v: Option<f64>| {
                        if let Some(v) = v {
                            value.set(v.round() as i64);
                            ui.st().set_setting(&key, &(v.round() as i64).to_string());
                        }
                    },
                }
                p { class: "uis-hnt", "{note}" }
            }
        }
    }
}

#[component]
pub fn SettingsCards() -> Element {
    let ui = use_context::<Ui>();
    let rows = [
        ("note", ui.t("lblStgColorNote")),
        ("music", ui.t("lblStgColorMusic")),
        ("vocal", ui.t("lblStgColorVocal")),
        ("SongStateUndefined", ui.t("lblStgColorSongUndefined")),
        ("SongStateCreated", ui.t("lblStgColorSongCreated")),
        ("SongStatePublished", ui.t("lblStgColorSongPublished")),
    ];
    let (cancel, apply, pick, reset) = (ui.t("btnGlbCancel"), ui.t("btnDlgCardApply"), ui.t("tipGlbPickFromScreen"), ui.t("btnStgCardsReset"));
    let lbl_colours = ui.t("lblStgCardsColors");
    rsx! {
        div { class: "ap-stg",
            div { class: "uis-fld-labelled",
                label { class: "uis-fld-labelled-label", "{lbl_colours}" }
                for (key , label) in rows {
                    div { key: "{key}", class: "ap-colour-row",
                        span { class: "ap-colour-name", style: "color:{ui.colour(key)}",
                            match key {
                                "note" => rsx! { Icon { width: 14, height: 14, icon: LdStickyNote } },
                                "music" => rsx! { Icon { width: 14, height: 14, icon: LdMusic } },
                                "vocal" => rsx! { Icon { width: 14, height: 14, icon: LdMic } },
                                _ => rsx! { Icon { width: 14, height: 14, icon: LdMusic2 } },
                            }
                            "{label}"
                        }
                        UisPkrColor {
                            color: ui.colour(key),
                            cancel_label: cancel.clone(),
                            apply_label: apply.clone(),
                            pick_title: pick.clone(),
                            on_change: move |c: String| {
                                let mut map = ui.colours.peek().clone();
                                map.insert(key.to_owned(), c);
                                ui.set_colours(map);
                            },
                        }
                        button {
                            class: "uis-btn subtle ap-end",
                            onclick: move |_| {
                                let mut map = ui.colours.peek().clone();
                                map.remove(key);
                                ui.set_colours(map);
                            },
                            "{reset}"
                        }
                    }
                }
            }
            TextSetting { name: "ai-sites", label: ui.t("lblStgCardsAiSites"), placeholder: "udio, suno, producer, tunee" }
            TextSetting { name: "music-sites", label: ui.t("lblStgCardsMusicSites"), placeholder: "soundcloud" }
        }
    }
}

/// The colours the user gives to tags; every other tag keeps its automatic colour.
#[component]
pub fn SettingsTags() -> Element {
    let ui = use_context::<Ui>();
    let mut input = use_signal(String::new);
    let mut pending = use_signal(|| "#7c6fff".to_owned());
    let (cancel, apply, pick) = (ui.t("btnGlbCancel"), ui.t("btnDlgCardApply"), ui.t("tipGlbPickFromScreen"));
    let tags = ui.tag_colours.read().clone();
    let known: Vec<String> = {
        let mut all: Vec<String> = ui.cards.read().iter().flat_map(|c| c.tags.iter().cloned()).collect::<std::collections::BTreeSet<_>>().into_iter().collect();
        all.retain(|t| !tags.contains_key(t));
        all
    };
    let needle = input().trim().to_lowercase();
    let suggestions: Vec<UisComboItem> = if needle.is_empty() {
        Vec::new()
    } else {
        known.iter().filter(|t| t.to_lowercase().contains(&needle)).take(10).map(|t| UisComboItem { key: t.clone(), label: t.clone(), active: false, header: false }).collect()
    };
    let picks = suggestions.clone();
    let mut add = move || {
        let tag = input.peek().trim().to_lowercase();
        if tag.is_empty() {
            return;
        }
        let mut map = ui.tag_colours.peek().clone();
        map.insert(tag, pending.peek().clone());
        ui.set_tag_colours(map);
        input.set(String::new());
        pending.set("#7c6fff".to_owned());
    };
    let (ttl, desc, plh, btn_add, empty, tip_remove) = (ui.t("ttlStgTagColors"), ui.t("msgStgTagColorsDesc"), ui.t("plhStgTagColor"), ui.t("btnStgTagColorAdd"), ui.t("msgStgTagColorEmpty"), ui.t("tipStgTagColorRemove"));
    rsx! {
        div { class: "ap-stg",
            div { class: "uis-fld-labelled",
                label { class: "uis-fld-labelled-label",
                    Icon { width: 14, height: 14, icon: LdTag }
                    " {ttl}"
                }
                p { class: "uis-hnt", "{desc}" }
                div { class: "ap-row",
                    div { class: "ap-grow",
                        UisCboStandard {
                            value: input(),
                            on_change: move |v: String| input.set(v),
                            placeholder: plh,
                            items: suggestions,
                            on_pick: move |i: usize| {
                                if let Some(item) = picks.get(i) {
                                    input.set(item.label.clone());
                                }
                            },
                            on_key_down: move |e: KeyboardEvent| {
                                if e.key() == Key::Enter {
                                    add();
                                }
                            },
                        }
                    }
                    UisPkrColor { color: pending(), on_change: move |c: String| pending.set(c), cancel_label: cancel.clone(), apply_label: apply.clone(), pick_title: pick.clone() }
                    button { class: "uis-btn primary", onclick: move |_| add(), "{btn_add}" }
                }
                div { class: "ap-tag-list",
                    if tags.is_empty() {
                        p { class: "uis-hnt", "{empty}" }
                    }
                    for (tag , colour) in tags.clone() {
                        div { key: "{tag}", class: "ap-tag-row",
                            span { class: "ap-grow", style: "color:{colour};font-weight:600;font-size:12px", "{tag}" }
                            UisPkrColor {
                                color: colour.clone(),
                                cancel_label: cancel.clone(),
                                apply_label: apply.clone(),
                                pick_title: pick.clone(),
                                on_change: {
                                    let tag = tag.clone();
                                    move |c: String| {
                                        let mut map = ui.tag_colours.peek().clone();
                                        map.insert(tag.clone(), c);
                                        ui.set_tag_colours(map);
                                    }
                                },
                            }
                            button {
                                class: "uis-btn ico small subtle",
                                title: "{tip_remove}",
                                onclick: {
                                    let tag = tag.clone();
                                    move |_| {
                                        let mut map = ui.tag_colours.peek().clone();
                                        map.remove(&tag);
                                        ui.set_tag_colours(map);
                                    }
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

#[component]
pub fn SettingsAi() -> Element {
    let ui = use_context::<Ui>();
    let mut key = use_signal({
        let v = ui.st().setting("gemini-key");
        move || v.clone()
    });
    let (lbl_key, plh_key, btn_clear, lbl_limits) = (ui.t("lblStgAiApiKey"), ui.t("plhStgAiApiKey"), ui.t("btnStgAiApiKeyClear"), ui.t("lblStgAiLimits"));
    let (get_key, key_on) = (ui.t("msgStgAiApiKeyGetKey"), ui.t("msgStgAiApiKeyEnabled"));
    let (lbl_templates, lbl_mode, btn_simple, btn_expert) = (ui.t("lblStgAiTemplates"), ui.t("lblStgAiTemplatesMode"), ui.t("btnStgAiTemplatesModeSimple"), ui.t("btnStgAiTemplatesModeExpert"));
    let mut expert = use_signal(|| ui.st().setting("ai-expert-mode") == "1");
    let (lbl_history, lbl_history_count, btn_history_clear) = (ui.t("lblStgAiHistory"), ui.t("lblStgAiHistoryCount"), ui.t("btnStgAiHistoryClear"));
    let mut history = use_signal(|| serde_json::from_str::<Vec<serde_json::Value>>(&ui.st().setting("ai-history")).map(|v| v.len()).unwrap_or(0));
    rsx! {
        div { class: "ap-stg",
            div { class: "uis-fld-labelled",
                label { class: "uis-fld-labelled-label",
                    Icon { width: 14, height: 14, icon: LdSparkles }
                    " {lbl_key}"
                }
                div { class: "ap-row",
                    input {
                        class: "uis-txt ap-mono",
                        r#type: "password",
                        placeholder: "{plh_key}",
                        value: "{key}",
                        oninput: move |e| {
                            key.set(e.value());
                            ui.st().set_setting("gemini-key", &e.value());
                        },
                    }
                    if !key().is_empty() {
                        button {
                            class: "uis-btn",
                            onclick: move |_| {
                                key.set(String::new());
                                ui.st().set_setting("gemini-key", "");
                            },
                            "{btn_clear}"
                        }
                    }
                }
                if key().is_empty() {
                    p { class: "uis-hnt", "{get_key}" }
                } else {
                    p { class: "uis-hnt", style: "color:var(--uis-ok)", "{key_on}" }
                }
            }
            div { class: "uis-fld-labelled ap-divider",
                label { class: "uis-fld-labelled-label", "{lbl_templates}" }
                div { class: "ap-row",
                    span { class: "uis-hnt ap-grow", "{lbl_mode} - {btn_simple} / {btn_expert}" }
                    div { class: "uis-barh-grp",
                        button {
                            class: if !expert() { "uis-btn ap-grow active" } else { "uis-btn ap-grow" },
                            onclick: move |_| {
                                expert.set(false);
                                ui.st().set_setting("ai-expert-mode", "0");
                            },
                            "{btn_simple}"
                        }
                        button {
                            class: if expert() { "uis-btn ap-grow active" } else { "uis-btn ap-grow" },
                            onclick: move |_| {
                                expert.set(true);
                                ui.st().set_setting("ai-expert-mode", "1");
                            },
                            "{btn_expert}"
                        }
                    }
                }
            }
            div { class: "uis-fld-labelled ap-divider",
                label { class: "uis-fld-labelled-label", "{lbl_limits}" }
                div { class: "ap-row",
                    LimitField { name: "ai-limit-music", label: ui.t("lblStgAiLimitsMusic"), default: 700, min: 100.0, max: 3000.0, step: 50.0 }
                    LimitField { name: "ai-limit-vocal", label: ui.t("lblStgAiLimitsVocal"), default: 300, min: 100.0, max: 3000.0, step: 50.0 }
                }
            }
            ThresholdField { name: "ai-warn-limit", label: ui.t("lblStgAiStyleThreshold"), note: ui.t("msgStgAiStyleThresholdDesc"), default: 1000, min: 100.0, max: 5000.0, step: 50.0 }
            ThresholdField { name: "ai-style-word-threshold", label: ui.t("lblStgAiStyleWordThreshold"), note: ui.t("msgStgAiStyleWordThresholdDesc"), default: 60, min: 10.0, max: 1000.0, step: 10.0 }
            ThresholdField { name: "ai-lyric-threshold", label: ui.t("lblStgAiLyricsThreshold"), note: ui.t("msgStgAiLyricsThresholdDesc"), default: 5000, min: 500.0, max: 20000.0, step: 50.0 }
            div { class: "uis-fld-labelled",
                label { class: "uis-fld-labelled-label", "{lbl_history}" }
                div { class: "ap-row",
                    p { class: "uis-hnt ap-grow", "{history()} {lbl_history_count}" }
                    button {
                        class: "uis-btn subtle",
                        onclick: move |_| {
                            ui.st().set_setting("ai-history", "");
                            history.set(0);
                            ui.ok(ui.t("tstStgAiHistoryCleared"));
                        },
                        "{btn_history_clear}"
                    }
                }
            }
        }
    }
}

/// Audit what a switch of the stored paths would do and show it in the dialog (nothing is written yet).
pub fn ask_media_paths(ui: Ui, relative: bool, root: &str) {
    match ui.st().audit_media_paths(relative, root.trim()) {
        Ok(Ok(plan)) => {
            let mut dialog = ui.dialog;
            dialog.set(Dialog::MediaPaths(plan));
        }
        Ok(Err(MediaRefused::NotAbsolute)) => ui.fail(ui.t("tstStgMediaRootInvalid")),
        Ok(Err(MediaRefused::OutsideRoot(n))) => ui.fail(ui.t("msgDlgMediaPathsBlocked").replace("{n}", &n.to_string())),
        Err(e) => ui.fail(e),
    }
}

/// The dialog's "choose another root": pick a folder, then audit again against it.
pub fn choose_media_root(ui: Ui) {
    spawn(async move {
        if let Some(dir) = rfd::AsyncFileDialog::new().set_parent(&dioxus::desktop::window().window).pick_folder().await {
            ask_media_paths(ui, true, &dir.path().to_string_lossy());
        }
    });
}

/// The dialog's Apply: write the switch.
pub fn apply_media_paths(ui: Ui, relative: bool, root: &str) {
    match ui.st().apply_media_paths(relative, root) {
        Ok(Ok(_)) => {
            ui.close();
            ui.reload();
            ui.ok(ui.t("tstStgMediaPathsApplied"));
        }
        Ok(Err(MediaRefused::NotAbsolute)) => ui.fail(ui.t("tstStgMediaRootInvalid")),
        Ok(Err(MediaRefused::OutsideRoot(n))) => ui.fail(ui.t("msgDlgMediaPathsBlocked").replace("{n}", &n.to_string())),
        Err(e) => ui.fail(e),
    }
}

#[component]
pub fn SettingsMedia() -> Element {
    let ui = use_context::<Ui>();
    let cfg = ui.st().media_cfg();
    let mut relative = use_signal(move || cfg.relative);
    let mut root = use_signal({
        let r = ui.st().media_cfg().root;
        move || r.clone()
    });
    // after a switch was applied the stored values changed: the drafts follow them
    use_effect(move || {
        let _ = ui.cards.read();
        let c = ui.st().media_cfg();
        relative.set(c.relative);
        root.set(c.root);
    });
    let (lbl_paths, btn_abs, btn_rel, note_paths, lbl_root, note_root) =
        (ui.t("lblStgMediaPaths"), ui.t("btnStgMediaPathsAbsolute"), ui.t("btnStgMediaPathsRelative"), ui.t("msgStgMediaPaths"), ui.t("lblStgMediaRoot"), ui.t("msgStgMediaRoot"));
    let check = move || ask_media_paths(ui, relative(), &root());
    rsx! {
        div { class: "ap-stg",
            div { class: "uis-fld-labelled",
                label { class: "uis-fld-labelled-label", "{lbl_paths}" }
                // the same row as AI > Templates (Simple / Expert): the explanation on the left, the welded pair sized to
                // its text on the right (owner, 2026-11-22; the pair used to stretch over the whole panel)
                div { class: "ap-row",
                    span { class: "uis-hnt ap-grow", "{note_paths}" }
                    div { class: "uis-barh-grp",
                        button {
                            class: if !relative() { "uis-btn ap-grow active" } else { "uis-btn ap-grow" },
                            onclick: move |_| {
                                if relative() {
                                    ask_media_paths(ui, false, &root());
                                }
                            },
                            "{btn_abs}"
                        }
                        button {
                            class: if relative() { "uis-btn ap-grow active" } else { "uis-btn ap-grow" },
                            onclick: move |_| {
                                if !relative() {
                                    relative.set(true);
                                    if !root().trim().is_empty() {
                                        ask_media_paths(ui, true, &root());
                                    }
                                }
                            },
                            "{btn_rel}"
                        }
                    }
                }
            }
            if relative() {
            div { class: "uis-fld-labelled",
                label { class: "uis-fld-labelled-label", "{lbl_root}" }
                div { class: "ap-row",
                    input {
                        class: "uis-txt ap-grow",
                        value: "{root}",
                        oninput: move |e| root.set(e.value()),
                        onkeydown: move |e| {
                            if e.key() == Key::Enter {
                                check();
                            }
                        },
                    }
                    button {
                        class: "uis-btn ico",
                        title: "{ui.t(\"tipStgMediaRootChoose\")}",
                        onclick: move |_| choose_media_root(ui),
                        Icon { width: 14, height: 14, icon: LdFolderOpen }
                    }
                }
                div { class: "ap-hint", "{note_root}" }
            }
            }
            FlagSetting { name: "use-yaiol-aiid", label: ui.t("lblStgMediaAiid"), note: ui.t("msgStgMediaAiid"), on_value: "1".to_owned() }
            FlagSetting { name: "sort-rename-media-default", label: ui.t("lblStgMediaRenameOnSortDefault"), note: ui.t("msgStgMediaRenameOnSortDefault"), on_value: "true".to_owned() }
        }
    }
}
