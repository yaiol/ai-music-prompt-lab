//! Translating songs with the model: one call for one song (`translate_song`, used by the card dialog too) and the dialog that
//! translates every song of a set of cards into one language, with progress and a way to stop.

use crate::{
    catalog::{UisCboStandard, UisComboItem},
    ui::Ui,
};
use dioxus::prelude::*;
use dioxus_free_icons::{
    icons::ld_icons::{LdLanguages, LdX},
    Icon,
};
use engine::{
    ai::{self, Generate},
    langs,
    cards::CardPatch,
    translate::{self, Translated},
};

/// Ask the model to translate one song. `source` / `target` are language codes; the source may be empty.
pub async fn translate_song(ui: Ui, name: &str, lyrics: &str, source: &str, target: &str) -> Result<Translated, String> {
    let api_key = ui.st().setting("gemini-key");
    let source_name = if source.is_empty() { String::new() } else { langs::english_name(source).unwrap_or(source).to_owned() };
    let target_name = langs::english_name(target).unwrap_or(target).to_owned();
    let request = translate::request(&ui.st(), name, lyrics, &source_name, &target_name)?;
    let got = tokio::task::spawn_blocking(move || ai::generate(&Generate { request, kind: "translate".to_owned(), api_key, raw: true, max_tokens: Some(8192), ..Default::default() })).await;
    match got {
        Ok(Ok(g)) => Ok(translate::read(&g.text)),
        Ok(Err(e)) => Err(e),
        Err(e) => Err(e.to_string()),
    }
}

/// Is this detected source one of the song languages? (the model may answer with anything)
pub fn known_code(code: &str) -> bool {
    langs::song_languages().iter().any(|l| l.code == code)
}

#[component]
pub fn TranslateDlg(ids: Vec<String>) -> Element {
    let ui = use_context::<Ui>();
    let mut query = use_signal(String::new);
    let mut target = use_signal(String::new);
    let mut progress = use_signal(|| None::<(usize, usize, String)>);
    let mut cancel = use_signal(|| false);
    let (title, lbl_target, lbl_progress, start, cancel_label) = (ui.t("ttlDlgTranslate"), ui.t("lblDlgTranslateTarget"), ui.t("lblDlgTranslateProgress"), ui.t("btnDlgTranslateStart"), ui.t("btnGlbCancel"));
    let has_key = !ui.st().setting("gemini-key").is_empty();
    let ui_lang = ui.lang.peek().clone();

    let q = query().trim().to_lowercase();
    let found: Vec<&langs::SongLanguage> = if q.is_empty() {
        Vec::new()
    } else {
        langs::song_languages().iter().filter(|l| l.name.to_lowercase().contains(&q) || langs::lang_name(&l.code, &ui_lang).to_lowercase().contains(&q) || l.code.to_lowercase().contains(&q)).collect()
    };
    let items: Vec<UisComboItem> = found.iter().map(|l| UisComboItem { key: l.code.clone(), label: format!("{} ({})", langs::lang_name(&l.code, &ui_lang), l.code), active: l.code == target(), header: false }).collect();
    let codes: Vec<String> = found.iter().map(|l| l.code.clone()).collect();
    let shown = if target().is_empty() { query() } else if q.is_empty() { format!("{} ({})", langs::lang_name(&target(), &ui_lang), target()) } else { query() };

    let run = {
        let ids = ids.clone();
        move |_| {
            let lang = target();
            if lang.is_empty() || !has_key {
                return;
            }
            let ids = ids.clone();
            spawn(async move {
                // the songs of the set that have lyrics and no version in this language yet
                let todo: Vec<_> = ids
                    .iter()
                    .filter_map(|id| ui.card(id))
                    .filter(|c| c.kind == "song" && !c.lyrics.trim().is_empty() && !ui.st().translations(&c.id).unwrap_or_default().iter().any(|t| t.lang == lang))
                    .collect();
                if todo.is_empty() {
                    ui.ok(ui.t("tstDlgTranslateAllDone"));
                    return;
                }
                cancel.set(false);
                let (mut done, mut errors) = (0usize, 0usize);
                for (i, card) in todo.iter().enumerate() {
                    if *cancel.peek() {
                        break;
                    }
                    progress.set(Some((i + 1, todo.len(), card.name.clone())));
                    match translate_song(ui, &card.name, &card.lyrics, &card.lang, &lang).await {
                        Ok(t) => {
                            if ui.st().create_translation(&card.id, &lang, &t.name, &t.content).is_err() {
                                errors += 1;
                                continue;
                            }
                            if card.lang.is_empty() && known_code(&t.source) {
                                let _ = ui.st().update_card(&card.id, &CardPatch { lang: Some(t.source), ..Default::default() });
                            }
                            done += 1;
                        }
                        Err(e) => {
                            errors += 1;
                            ui.fail(e);
                        }
                    }
                }
                progress.set(None);
                ui.reload();
                ui.close();
                let tail = if errors > 0 { format!(", {errors} error(s)") } else { String::new() };
                ui.ok(format!("{done} {}{tail}", ui.t("tstDlgTranslateDone")));
            });
        }
    };

    rsx! {
        div {
            class: "uis-dl-backdrop",
            onmousedown: move |_| {
                if progress.peek().is_none() {
                    ui.close();
                }
            },
            div { class: "uis-dlp", style: "max-width:300px", onmousedown: move |e| e.stop_propagation(),
                if let Some((current , total , name)) = progress() {
                    span { class: "uis-dlp-icon", Icon { width: 28, height: 28, icon: LdLanguages } }
                    p { class: "uis-dlp-title", "{lbl_progress} {current}/{total}" }
                    p { class: "uis-dlp-sub ap-ellipsis", "{name}" }
                    div { class: "uis-dlp-foot",
                        button { class: "uis-btn subtle", onclick: move |_| cancel.set(true),
                            Icon { width: 14, height: 14, icon: LdX }
                            " {cancel_label}"
                        }
                    }
                } else {
                    button { class: "uis-dl-close", onclick: move |_| ui.close(), Icon { width: 14, height: 14, icon: LdX } }
                    span { class: "uis-dlp-icon", Icon { width: 28, height: 28, icon: LdLanguages } }
                    p { class: "uis-dlp-title", "{title}" }
                    div { class: "uis-dlp-body",
                        div { style: "width:100%",
                            span { class: "uis-fld-labelled-label", "{lbl_target}" }
                            UisCboStandard {
                                value: shown,
                                auto_focus: true,
                                placeholder: lbl_target.clone(),
                                items,
                                on_change: move |v: String| {
                                    query.set(v);
                                    target.set(String::new());
                                },
                                on_pick: move |i: usize| {
                                    if let Some(code) = codes.get(i) {
                                        target.set(code.clone());
                                        query.set(String::new());
                                    }
                                },
                            }
                        }
                    }
                    div { class: "uis-dlp-foot",
                        button { class: "uis-btn primary", disabled: target().is_empty() || !has_key, onclick: run, "{start}" }
                    }
                }
            }
        }
    }
}
