// ⚠ NO HARDCODED UI STRINGS IN THIS FILE.
//   Every string a user can read MUST go through the t() lookup, and the key MUST be a string literal at the call site.

//! The song wizard of the card dialog (the Electron app's overlay): pick a wizard when there are several, name the artist (the
//! artists tried before are offered as the name is typed), read the artist's "DNA" the model gives back and pick one of its
//! subjects (or type one), and the model writes the song: its style, lyrics, title, description and translations.
//! The artist and what was learned about them reach the card as soon as the first step is done; the song at the end.

use crate::{
    catalog::{UisCboStandard, UisComboItem},
    ui::Ui,
};
use dioxus::prelude::*;
use dioxus_free_icons::{Icon, icons::ld_icons::{LdRotateCcw, LdWand}};
use engine::{
    ai::{self, Generate, Wizard},
    wizard::{parse_response, task_request},
};
use std::collections::HashMap;

/// The song the wizard wrote.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WizardSong {
    pub style: String,
    pub lyrics: String,
    pub name: String,
    pub lang: String,
    pub desc: String,
    /// The translations the model gave: (language code, title, lyrics).
    pub translations: Vec<(String, String, String)>,
}

/// Ask the model and read the answer into its keys.
async fn ask(request: String, api_key: String) -> Result<HashMap<String, String>, String> {
    let got = tokio::task::spawn_blocking(move || ai::generate(&Generate { request, kind: "lyrics".to_owned(), api_key, raw: true, max_tokens: Some(8192), ..Default::default() })).await;
    match got {
        Ok(Ok(g)) => Ok(parse_response(&g.text)),
        Ok(Err(e)) => Err(e),
        Err(e) => Err(e.to_string()),
    }
}

fn task_for(list: &[Wizard], selected: Option<usize>, category: &str) -> Option<String> {
    selected.and_then(|i| list.get(i)).and_then(|w| w.steps.iter().find(|s| s.category == category)).map(|s| s.task.clone())
}

fn artists(ui: Ui) -> Vec<String> {
    serde_json::from_str(&ui.st().setting("wizard-artists")).unwrap_or_default()
}

#[component]
pub fn WizardDlg(on_dna: EventHandler<(String, String)>, on_done: EventHandler<WizardSong>, on_close: EventHandler<()>) -> Element {
    let ui = use_context::<Ui>();
    let list: Vec<Wizard> = use_hook(|| ui.st().ai_wizards().unwrap_or_default());
    // 1 = which wizard, 2 = the artist, 3 = the subject
    let mut step = use_signal(|| if list.len() == 1 { 2 } else { 1 });
    let mut selected = use_signal(|| if list.len() == 1 { Some(0usize) } else { None });
    let mut artist = use_signal(String::new);
    let mut dna = use_signal(HashMap::<String, String>::new);
    let mut subject = use_signal(String::new);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(String::new);
    let history = use_signal(|| artists(ui));

    let several = list.len() > 1;
    let t = |k: &str| ui.t(k);
    let (title, l_artist, l_lang, l_subjects, plh_subject, back, next, generating, cancel) = (
        t("ttlDlgWizard"),
        t("lblDlgWizardArtist"),
        t("lblDlgWizardLanguage"),
        t("lblDlgWizardSubjects"),
        t("plhDlgWizardCustomSubject"),
        t("btnDlgWizardBack"),
        t("btnDlgWizardNext"),
        t("lblDlgWizardGenerating"),
        t("btnGlbCancel"),
    );
    let plh_artist = "Tiken Jah Fakoly, Idir, Cesária Évora…".to_owned();
    let counter = if several { format!("{}/3", step()) } else { format!("{}/2", step() - 1) };
    let typed = artist().trim().to_lowercase();
    let offered: Vec<UisComboItem> = if typed.is_empty() {
        Vec::new()
    } else {
        history().iter().filter(|a| a.to_lowercase().starts_with(&typed) && a.to_lowercase() != typed).take(8).map(|a| UisComboItem { key: a.clone(), label: a.clone(), active: false, header: false }).collect()
    };
    let offered_names: Vec<String> = offered.iter().map(|o| o.key.clone()).collect();
    let subjects: Vec<String> = dna()
        .get("SUBJECTS")
        .map(|s| s.split('\n').map(|l| l.trim_start_matches(|c: char| c.is_ascii_digit()).trim_start_matches('.').trim().to_owned()).filter(|l| !l.is_empty()).collect())
        .unwrap_or_default();

    // step 2 → 3: the artist's DNA
    let note_list = list.clone();
    let learn = use_callback(move |_: ()| {
        let name = artist().trim().to_owned();
        let api_key = ui.st().setting("gemini-key");
        if name.is_empty() || api_key.is_empty() || busy() {
            return;
        }
        let Some(task) = task_for(&note_list, selected(), "wzd-note") else { return error.set("Missing wzd-note task in wizard".to_owned()) };
        let request = match task_request(&ui.st(), ai::system_blocks(), &task, &HashMap::from([("ARTIST", name.clone())])) {
            Ok(r) => r,
            Err(e) => return error.set(e),
        };
        busy.set(true);
        error.set(String::new());
        spawn(async move {
            let got = ask(request, api_key).await;
            busy.set(false);
            match got {
                Ok(bag) => {
                    let get = |k: &str| bag.get(k).filter(|v| !v.is_empty()).cloned();
                    let mut parts: Vec<String> = Vec::new();
                    for key in ["ARTIST", "LANGUAGE_PRIMARY", "LANGUAGE_SECONDARY", "STANCE", "STRUCTURE", "STYLE"] {
                        if let Some(v) = get(key) {
                            parts.push(format!("{key}: {v}"));
                        }
                    }
                    if let Some(v) = get("SUBJECTS") {
                        parts.push(format!("SUBJECTS:\n{v}"));
                    }
                    on_dna.call((parts.join("\n\n"), name.clone()));
                    // the artists tried before, the latest first, 50 at most
                    let mut next: Vec<String> = vec![name.clone()];
                    next.extend(artists(ui).into_iter().filter(|a| a.to_lowercase() != name.to_lowercase()));
                    next.truncate(50);
                    ui.st().set_setting("wizard-artists", &serde_json::to_string(&next).unwrap_or_default());
                    dna.set(bag);
                    step.set(3);
                }
                Err(e) => error.set(e),
            }
        });
    });

    // step 3: the song
    let song_list = list.clone();
    let write_song = use_callback(move |_: ()| {
        let api_key = ui.st().setting("gemini-key");
        if subject().is_empty() || api_key.is_empty() || busy() {
            return;
        }
        let (Some(style_task), Some(lyrics_task)) = (task_for(&song_list, selected(), "wzd-style"), task_for(&song_list, selected(), "wzd-lyrics")) else { return error.set("Missing wzd-style or wzd-lyrics task in wizard".to_owned()) };
        let name = artist().trim().to_owned();
        let lang = dna().get("LANGUAGE_PRIMARY").filter(|l| !l.is_empty()).cloned().unwrap_or_else(|| "English".to_owned());
        let style_request = task_request(&ui.st(), ai::system_blocks(), &style_task, &HashMap::from([("ARTIST", name.clone())]));
        let lyrics_request = task_request(&ui.st(), ai::system_blocks(), &lyrics_task, &HashMap::from([("ARTIST", name), ("SUBJECT", subject()), ("LANGUAGE", lang)]));
        let (style_request, lyrics_request) = match (style_request, lyrics_request) {
            (Ok(a), Ok(b)) => (a, b),
            (Err(e), _) | (_, Err(e)) => return error.set(e),
        };
        busy.set(true);
        error.set(String::new());
        spawn(async move {
            // the style and the lyrics are asked together
            let (style_bag, lyrics_bag) = tokio::join!(ask(style_request, api_key.clone()), ask(lyrics_request, api_key));
            busy.set(false);
            match (style_bag, lyrics_bag) {
                (Ok(style_bag), Ok(lyrics_bag)) => {
                    let get = |k: &str| lyrics_bag.get(k).cloned().unwrap_or_default();
                    let mut song = WizardSong {
                        style: style_bag.get("CONTENT").cloned().unwrap_or_default(),
                        lyrics: get("LYRICS_1"),
                        name: get("TITLE_1"),
                        lang: get("LANGUAGE_1"),
                        desc: get("DESCRIPTION"),
                        translations: Vec::new(),
                    };
                    // a second and a third version of the song, when the model gave them
                    for (n, fallback) in [("2", "eng"), ("3", "fra")] {
                        let lyrics = get(&format!("LYRICS_{n}"));
                        if !lyrics.is_empty() {
                            let code = Some(get(&format!("LANGUAGE_{n}"))).filter(|c| !c.is_empty()).unwrap_or_else(|| fallback.to_owned());
                            song.translations.push((code, get(&format!("TITLE_{n}")), lyrics));
                        }
                    }
                    on_done.call(song);
                    on_close.call(());
                }
                (Err(e), _) | (_, Err(e)) => error.set(e),
            }
        });
    });

    let can_next = !busy() && ((step() == 2 && !artist().trim().is_empty()) || (step() == 3 && !subject().is_empty()));
    rsx! {
        div {
            class: "uis-dl-backdrop ap-nested",
            onmousedown: move |e| {
                e.stop_propagation();
                on_close.call(());
            },
            div { class: "uis-dlg", style: "width:min(420px,92vw)", onmousedown: move |e| e.stop_propagation(),
                div { class: "uis-dlg-head",
                    span { class: "uis-dlg-title",
                        Icon { width: 16, height: 16, icon: LdWand }
                        " {title}"
                    }
                    span { class: "uis-hnt", "{counter}" }
                }
                div { class: "uis-dlg-body ap-dlg-body",
                    if step() == 1 {
                        for (i , w) in list.iter().enumerate() {
                            button {
                                key: "{w.id}",
                                class: "uis-lvw-item",
                                onclick: move |_| {
                                    selected.set(Some(i));
                                    step.set(2);
                                },
                                span { class: "uis-lvw-item-name", "{w.name}" }
                            }
                        }
                    }
                    if step() == 2 {
                        if several {
                            if let Some(w) = selected().and_then(|i| list.get(i)) {
                                div { class: "uis-hnt", "{w.name}" }
                            }
                        }
                        label { class: "uis-fld-labelled-label", "{l_artist}" }
                        UisCboStandard {
                            value: artist(),
                            on_change: move |v: String| artist.set(v),
                            auto_focus: true,
                            placeholder: plh_artist.clone(),
                            input_style: "width:100%".to_owned(),
                            items: offered.clone(),
                            on_pick: move |i: usize| {
                                if let Some(a) = offered_names.get(i) {
                                    artist.set(a.clone());
                                }
                            },
                            on_key_down: move |e: KeyboardEvent| {
                                if e.key() == Key::Enter && !artist().trim().is_empty() {
                                    learn.call(());
                                }
                            },
                        }
                    }
                    if step() == 3 {
                        div { class: "ap-row",
                            div { class: "ap-grow",
                                label { class: "uis-fld-labelled-label", "{l_artist}" }
                                div { "{artist}" }
                            }
                            div { class: "ap-grow",
                                label { class: "uis-fld-labelled-label", "{l_lang}" }
                                div { "{dna().get(\"LANGUAGE_PRIMARY\").cloned().unwrap_or_else(|| \"-\".to_owned())}" }
                            }
                        }
                        label { class: "uis-fld-labelled-label", "{l_subjects}" }
                        for s in subjects {
                            {
                                let pick = s.clone();
                                rsx! {
                                    button {
                                        key: "{s}",
                                        class: if subject() == s { "uis-lvw-item active" } else { "uis-lvw-item" },
                                        onclick: move |_| subject.set(pick.clone()),
                                        span { class: "uis-lvw-item-name", "{s}" }
                                    }
                                }
                            }
                        }
                        input { class: "uis-txt", placeholder: "{plh_subject}", value: "{subject}", oninput: move |e| subject.set(e.value()) }
                    }
                    if !error().is_empty() {
                        div { class: "ap-hint ap-danger", "{error}" }
                    }
                }
                div { class: "uis-dlg-foot",
                    button { class: "uis-btn subtle", onclick: move |_| on_close.call(()), "{cancel}" }
                    if step() > 1 {
                        button {
                            class: "uis-btn subtle",
                            onclick: move |_| {
                                if step() == 2 && !several {
                                    on_close.call(());
                                } else {
                                    step.set(step() - 1);
                                }
                            },
                            "{back}"
                        }
                        button {
                            class: "uis-btn primary",
                            disabled: !can_next,
                            onclick: move |_| if step() == 2 { learn.call(()) } else { write_song.call(()) },
                            if busy() {
                                Icon { width: 14, height: 14, icon: LdRotateCcw }
                                " {generating}"
                            } else {
                                "{next}"
                            }
                        }
                    }
                }
            }
        }
    }
}
