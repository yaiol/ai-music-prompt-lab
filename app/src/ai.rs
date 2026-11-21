// ⚠ NO HARDCODED UI STRINGS IN THIS FILE.
//   Every string a user can read MUST go through the t() lookup, and the key MUST be a string literal at the call site.

//! "Generate with AI" on a card. The request is put together from a template's blocks (each with its preset's text)
//! followed by the system blocks, then sent to Gemini. SIMPLE mode shows the template and the request; EXPERT mode
//! also shows every block (preset, text, on/off) and lets the user keep presets and templates of their own.
//! What was typed is kept on the card (`ai_tweaks`) so the dialog opens where it was left.

use crate::{catalog::UisNum, ui::Ui};
use dioxus::prelude::*;
use dioxus_free_icons::{
    Icon,
    icons::ld_icons::{LdCheck, LdChevronDown, LdCopy, LdPlus, LdSave, LdSparkles, LdStar, LdTextCursor, LdTrash2, LdX},
};
use engine::ai::{self, Generate, Preset, Template};
use serde_json::{Value, json};
use std::collections::BTreeMap;

/// One block of the template as the dialog shows it.
#[derive(Clone, PartialEq)]
struct BlockState {
    block: String,
    name: String,
    prefix: String,
    preset: Option<String>,
    content: String,
    enabled: bool,
    open: bool,
}

/// A variable written in a block's text: `[name:type:default]`.
#[derive(Clone, PartialEq)]
struct Var {
    name: String,
    kind: String,
    default: String,
}

/// The variables in a text, in order, each name once.
fn variables_in(text: &str, into: &mut Vec<Var>) {
    let mut rest = text;
    while let Some(start) = rest.find('[') {
        let tail = &rest[start + 1..];
        let parsed = (|| {
            let end = tail.find(']')?;
            let (name, after) = tail[..end].split_once(':')?;
            let (kind, default) = after.split_once(':')?;
            (!name.is_empty() && !name.contains([':', ']']) && matches!(kind, "num" | "txt" | "ckb")).then(|| (Var { name: name.to_owned(), kind: kind.to_owned(), default: default.to_owned() }, end))
        })();
        match parsed {
            Some((v, end)) => {
                if !into.iter().any(|x| x.name == v.name) {
                    into.push(v);
                }
                rest = &tail[end + 1..];
            }
            None => rest = tail,
        }
    }
}

/// Replace the `{prompt-…}` markers by the limit, and every variable by what the user gave it (its default when nothing).
/// A tick box that was cleared gives nothing.
fn resolve_vars(content: &str, limit: i64, values: &BTreeMap<String, String>) -> String {
    let s = content
        .replace("{prompt-chars-max}", &limit.to_string())
        .replace("{prompt-chars-interval}", &format!("{}-{}", limit - 50, limit))
        .replace("{prompt-words-max}", &limit.to_string())
        .replace("{prompt-words-interval}", &format!("{}-{}", limit - 10, limit));
    let mut out = String::new();
    let mut rest = s.as_str();
    while let Some(start) = rest.find('[') {
        out.push_str(&rest[..start]);
        let tail = &rest[start + 1..];
        let mut found = Vec::new();
        let end = tail.find(']');
        if let Some(end) = end {
            variables_in(&format!("[{}]", &tail[..end]), &mut found);
        }
        match (found.first(), end) {
            (Some(v), Some(end)) => {
                let given = values.get(&v.name);
                if v.kind == "ckb" {
                    if given.map(String::as_str) != Some("off") {
                        out.push_str(&v.default);
                    }
                } else {
                    out.push_str(given.filter(|g| !g.is_empty()).unwrap_or(&v.default));
                }
                rest = &tail[end + 1..];
            }
            _ => {
                out.push('[');
                rest = tail;
            }
        }
    }
    out.push_str(rest);
    out
}

/// The default blocks of a task, when its template has none.
fn default_blocks(task: &str) -> &'static [&'static str] {
    match task {
        "lyrics" => &["role", "structure", "style", "rules"],
        "music" | "vocal" => &["role", "style", "rules"],
        _ => &["role", "rules", "style"],
    }
}

/// The blocks a template starts with: each on its preset, or on the block's default one when the preset is gone.
fn blocks_of(task: &str, tpl: &Template, presets: &[Preset]) -> Vec<BlockState> {
    let pick = |block: &str, id: Option<&String>| -> Option<Preset> {
        let of_block: Vec<&Preset> = presets.iter().filter(|p| p.block == block).collect();
        id.and_then(|id| of_block.iter().find(|p| &p.id == id)).or_else(|| of_block.iter().find(|p| p.is_default_pick)).or(of_block.first()).map(|p| (*p).clone())
    };
    if tpl.blocks.is_empty() {
        return default_blocks(task)
            .iter()
            .map(|b| {
                let p = pick(b, None);
                BlockState { block: (*b).to_owned(), name: (*b).to_owned(), prefix: b.to_uppercase(), preset: p.as_ref().map(|p| p.id.clone()), content: p.map(|p| p.content).unwrap_or_default(), enabled: true, open: true }
            })
            .collect();
    }
    tpl.blocks
        .iter()
        .map(|b| {
            let p = pick(&b.block, b.preset_id.as_ref());
            BlockState { block: b.block.clone(), name: b.name.clone(), prefix: b.prefix.clone(), preset: p.as_ref().map(|p| p.id.clone()), content: p.map(|p| p.content).unwrap_or_default(), enabled: b.enabled, open: true }
        })
        .collect()
}

/// What a template says about its blocks, to tell later whether the user changed them.
fn snapshot(blocks: &[BlockState]) -> Vec<(String, Option<String>, bool)> {
    blocks.iter().map(|b| (b.block.clone(), b.preset.clone(), b.enabled)).collect()
}

/// The text sent to the model.
fn assemble(task: &str, blocks: &[BlockState], request: &str, limit: i64, values: &BTreeMap<String, String>) -> String {
    let mut parts: Vec<String> = Vec::new();
    for b in blocks.iter().filter(|b| b.enabled && !b.content.trim().is_empty()) {
        let prefix = if b.prefix.is_empty() { format!("{}: ", b.block.to_uppercase()) } else { b.prefix.clone() };
        parts.push(format!("{prefix}{}", resolve_vars(b.content.trim(), limit, values)));
    }
    for sys in ai::system_blocks().as_array().into_iter().flatten() {
        let block = sys["block"].as_str().unwrap_or("");
        let prefix = sys["prefix"].as_str().map(str::to_owned).unwrap_or_else(|| format!("{}: ", block.to_uppercase()));
        let for_task = &sys["tasks"][task];
        if let Some(content) = for_task["content"].as_str() {
            parts.push(format!("{prefix}{}", resolve_vars(content, limit, values)));
        } else if let Some(vars) = for_task["vars"].as_array() {
            let lines: Vec<String> = vars
                .iter()
                .map(|v| {
                    let name = v["name"].as_str().unwrap_or("");
                    let value = if name == "USER_REQUEST" { request.trim().to_owned() } else { values.get(name).cloned().filter(|g| !g.is_empty()).unwrap_or_else(|| v["example"].as_str().unwrap_or("").to_owned()) };
                    format!("{name}: {value}")
                })
                .collect();
            parts.push(format!("{prefix}{}", lines.join("\n")));
        } else if sys.get("tasks").is_none() && !request.trim().is_empty() {
            parts.push(format!("{prefix}{}", request.trim()));
        }
    }
    parts.join("\n")
}

/// What is being typed in the little name row: a new template, a new preset, a new name for either.
#[derive(Clone, PartialEq)]
enum Naming {
    NewTemplate,
    RenameTemplate,
    NewPreset(String),
    RenamePreset(String),
}

/// What waits for a yes under a row.
#[derive(Clone, PartialEq)]
enum Asking {
    DeleteTemplate,
    DeletePreset(String),
}

#[component]
pub fn AiDlg(kind: String, tweaks: Value, on_tweaks: EventHandler<Value>, on_done: EventHandler<(String, Vec<String>)>, on_close: EventHandler<()>) -> Element {
    let ui = use_context::<Ui>();
    let (task, gen_kind, limit_key, default_limit) = match kind.as_str() {
        "vocal" => ("vocal", "vocal_style", "ai-limit-vocal", 300i64),
        "song" => ("lyrics", "lyrics", "ai-lyric-threshold", 5000),
        "songstyle" => ("music", "music_style", "ai-warn-limit", 1000),
        _ => ("music", "music_style", "ai-limit-music", 700),
    };
    let saved = tweaks[task].clone();
    let init_limit: i64 = ui.st().setting(limit_key).parse().unwrap_or(default_limit);
    let init_words: i64 = ui.st().setting("ai-style-word-threshold").parse().unwrap_or(60);

    let mut templates = use_signal(|| ui.st().ai_templates(Some(task)).unwrap_or_default());
    let mut presets = use_signal(|| ui.st().ai_presets(Some(task), None).unwrap_or_default());
    let mut default_id = use_signal(|| ui.st().setting(&format!("ai-default-template-{task}")));
    let mut chosen = use_signal({
        let templates = templates.peek().clone();
        let default_id = default_id.peek().clone();
        let saved_id = saved["templateId"].as_str().unwrap_or("").to_owned();
        move || {
            [saved_id, default_id]
                .iter()
                .find(|id| !id.is_empty() && templates.iter().any(|t| &t.id == *id))
                .cloned()
                .or_else(|| templates.first().map(|t| t.id.clone()))
                .unwrap_or_default()
        }
    });
    let mut blocks = use_signal({
        let tpl = templates.peek().iter().find(|t| t.id == *chosen.peek()).cloned();
        let ps = presets.peek().clone();
        let saved = saved.clone();
        move || {
            let mut b = tpl.map(|t| blocks_of(task, &t, &ps)).unwrap_or_default();
            if let Some(texts) = saved["blocks"].as_object().filter(|o| !o.is_empty()) {
                for x in b.iter_mut() {
                    if let Some(t) = texts.get(&x.block).and_then(Value::as_str) {
                        x.content = t.to_owned();
                    }
                }
            }
            b
        }
    });
    let mut template_blocks = use_signal(|| snapshot(&blocks.peek()));
    let mut expert = use_signal(|| ui.st().setting("ai-expert-mode") == "1");
    let mut request = use_signal(|| saved["request"].as_str().unwrap_or("").to_owned());
    let mut limit = use_signal(|| saved["limit"].as_i64().unwrap_or(init_limit));
    let mut values = use_signal(BTreeMap::<String, String>::new);
    let mut naming = use_signal(|| None::<(Naming, String)>);
    let mut asking = use_signal(|| None::<Asking>);
    let mut busy = use_signal(|| false);
    let mut add_pick = use_signal(String::new);
    let key = ui.st().setting("gemini-key");
    let no_key = key.is_empty();

    // ── derived ──
    let cur: Option<Template> = templates.read().iter().find(|t| t.id == chosen()).cloned();
    let cur_default = cur.as_ref().is_some_and(|t| t.is_default);
    let template_modified = !cur_default && {
        let was = template_blocks.read();
        blocks.read().iter().any(|b| was.iter().find(|w| w.0 == b.block).map(|w| (&w.1, w.2)).unwrap_or((&None, true)) != (&b.preset, b.enabled))
    };
    let mut all_vars: Vec<Var> = Vec::new();
    for b in blocks.read().iter().filter(|b| b.enabled) {
        variables_in(&b.content, &mut all_vars);
    }
    // the unit of the limit comes from what the text says, so it follows the template
    let length_mode: Option<&'static str> = {
        let mut texts: Vec<String> = blocks.read().iter().filter(|b| b.enabled).map(|b| b.content.clone()).collect();
        for sys in ai::system_blocks().as_array().into_iter().flatten() {
            if let Some(c) = sys["tasks"][task]["content"].as_str() {
                texts.push(c.to_owned());
            }
        }
        let all = texts.join("\n");
        if all.contains("{prompt-chars-max}") || all.contains("{prompt-chars-interval}") {
            Some("chars")
        } else if all.contains("{prompt-words-max}") || all.contains("{prompt-words-interval}") {
            Some("words")
        } else {
            None
        }
    };
    let mut seen_mode = use_signal(|| None::<&'static str>);
    let had_limit = saved["limit"].is_i64();
    use_effect(move || {
        let Some(mode) = length_mode else { return };
        let before = *seen_mode.peek();
        seen_mode.set(Some(mode));
        match before {
            None => {
                if !had_limit && mode == "words" && task != "lyrics" {
                    limit.set(init_words);
                }
            }
            Some(prev) if prev != mode => limit.set(if mode == "words" && task != "lyrics" { init_words } else { init_limit }),
            _ => {}
        }
    });

    // ── what the dialog says it will send, and what it remembers ──
    let build = move || assemble(task, &blocks.read(), &request.read(), limit(), &values.read());
    let remember = move || {
        let mut texts = serde_json::Map::new();
        for b in blocks.read().iter().filter(|b| !b.content.is_empty()) {
            texts.insert(b.block.clone(), Value::String(b.content.clone()));
        }
        let mut all = tweaks.clone();
        if !all.is_object() {
            all = json!({});
        }
        all[task] = json!({ "templateId": chosen(), "blocks": texts, "request": request(), "limit": limit() });
        on_tweaks.call(all);
    };
    let mut reload_templates = move || templates.set(ui.st().ai_templates(Some(task)).unwrap_or_default());
    let mut reload_presets = move || presets.set(ui.st().ai_presets(Some(task), None).unwrap_or_default());
    let mut load_template = move |id: &str| {
        let tpl = templates.peek().iter().find(|t| t.id == id).cloned();
        if let Some(tpl) = tpl {
            let b = blocks_of(task, &tpl, &presets.peek());
            template_blocks.set(snapshot(&b));
            blocks.set(b);
        }
    };

    let label_of = move |b: &BlockState| -> String {
        match b.block.as_str() {
            "role" if b.name == b.block => ui.t("lblDlgAiRole"),
            "structure" if b.name == b.block => ui.t("lblDlgAiStructure"),
            "rules" if b.name == b.block => ui.t("lblDlgAiRules"),
            "style" if b.name == b.block => ui.t("lblDlgCardStyle"),
            _ if b.name.is_empty() => b.block.clone(),
            _ => b.name.clone(),
        }
    };
    let block_label = move |block: &str| -> String {
        match block {
            "role" => ui.t("lblDlgAiRole"),
            "structure" => ui.t("lblDlgAiStructure"),
            "rules" => ui.t("lblDlgAiRules"),
            "style" => ui.t("lblDlgCardStyle"),
            other => other.to_owned(),
        }
    };

    // ── template actions ──
    let mut confirm_naming = move || {
        let Some((what, text)) = naming.peek().clone() else { return };
        let text = text.trim().to_owned();
        if text.is_empty() {
            return;
        }
        match what {
            Naming::NewTemplate => {
                let source = templates.peek().first().cloned();
                let blocks_in: Vec<ai::TemplateBlock> = match source.filter(|s| !s.blocks.is_empty()) {
                    Some(s) => s.blocks.iter().map(|b| ai::TemplateBlock { id: String::new(), block: b.block.clone(), preset_id: None, enabled: true, name: b.name.clone(), prefix: b.prefix.clone(), hidden: false }).collect(),
                    None => default_blocks(task).iter().map(|b| ai::TemplateBlock { id: String::new(), block: (*b).to_owned(), preset_id: None, enabled: true, name: (*b).to_owned(), prefix: b.to_uppercase(), hidden: false }).collect(),
                };
                match ui.st().create_ai_template(task, &text, &blocks_in) {
                    Ok(t) => {
                        reload_templates();
                        chosen.set(t.id.clone());
                        load_template(&t.id);
                        ui.ok(ui.t("tstDlgAiTemplateSaved"));
                    }
                    Err(e) => ui.fail(e),
                }
            }
            Naming::RenameTemplate => match ui.st().rename_ai_template(&chosen.peek(), &text) {
                Ok(_) => {
                    reload_templates();
                    ui.ok(ui.t("tstDlgAiTemplateSaved"));
                }
                Err(e) => ui.fail(e),
            },
            Naming::NewPreset(block) => {
                let content = blocks.peek().iter().find(|b| b.block == block).map(|b| b.content.clone()).unwrap_or_default();
                match ui.st().create_ai_preset(task, &block, &text, &content) {
                    Ok(p) => {
                        reload_presets();
                        if let Some(b) = blocks.write().iter_mut().find(|b| b.block == block) {
                            b.preset = Some(p.id);
                        }
                        ui.ok(ui.t("tstDlgAiPresetSaved"));
                    }
                    Err(e) => ui.fail(e),
                }
            }
            Naming::RenamePreset(block) => {
                let id = blocks.peek().iter().find(|b| b.block == block).and_then(|b| b.preset.clone());
                if let Some(id) = id {
                    match ui.st().update_ai_preset(&id, Some(&text), None) {
                        Ok(_) => {
                            reload_presets();
                            ui.ok(ui.t("tstDlgAiPresetSaved"));
                        }
                        Err(e) => ui.fail(e),
                    }
                }
            }
        }
        naming.set(None);
    };

    let mut set_naming_text = move |v: String| {
        let now = naming.peek().clone();
        naming.set(now.map(|(w, _)| (w, v)));
    };

    let s_tipDlgAiTweakBlock = ui.t("tipDlgAiTweakBlock");
    let s_tipDlgAiSaveAsNew = ui.t("tipDlgAiSaveAsNew");
    let s_tipDlgAiRenameTemplate = ui.t("tipDlgAiRenameTemplate");
    let s_tipDlgAiOverwrite = ui.t("tipDlgAiOverwrite");
    let s_tipDlgAiToggleBlock = ui.t("tipDlgAiToggleBlock");
    let s_tipDlgAiRemoveBlock = ui.t("tipDlgAiRemoveBlock");
    let s_tipDlgAiDeletePreset = ui.t("tipDlgAiDeletePreset");
    let s_btnDlgAiDeletePreset = ui.t("btnDlgAiDeletePreset");
    let s_tipDlgAiAddBlock = ui.t("tipDlgAiAddBlock");
    let s_btnDlgAiAddBlock = ui.t("btnDlgAiAddBlock");
    let s_msgStgAiApiKeyGetKey = ui.t("msgStgAiApiKeyGetKey");

    // a row of three icons or fewer: the little buttons of the dialog
    let small = "uis-btn ico small subtle";
    let (t_add_tpl, t_ren, t_save, t_del, t_def) = (ui.t("tipDlgAiSaveAsTemplate"), ui.t("tipDlgAiRenameTemplate"), ui.t("tipDlgAiOverwrite"), ui.t("tipDlgAiDeleteTemplate"), ui.t("tipDlgAiSetDefault"));
    let is_default_tpl = !chosen().is_empty() && default_id() == chosen();
    let (lbl_title, lbl_mode, btn_mode) = (ui.t("lblStgAiTemplates"), ui.t("lblStgAiTemplatesMode"), if expert() { ui.t("btnStgAiTemplatesModeSimple") } else { ui.t("btnStgAiTemplatesModeExpert") });
    let (tip_copy, tip_gen, lbl_request, ttl_blocks, ttl_vars) = (ui.t("tipDlgAiCopyRequest"), ui.t("tipDlgAiGenerate"), ui.t("lblDlgAiYourRequest"), ui.t("ttlDlgAiBlocks"), ui.t("ttlDlgAiVariables"));
    let placeholder = match task {
        "vocal" => ui.t("plhDlgCardAiVocal"),
        "lyrics" => ui.t("plhDlgCardAiSong"),
        _ => ui.t("plhDlgCardAiMusic"),
    };
    let blocks_now = blocks.read().clone();
    let presets_now = presets.read().clone();
    let free_blocks: Vec<String> = {
        let mut seen: Vec<String> = Vec::new();
        for p in &presets_now {
            if !seen.contains(&p.block) && !blocks_now.iter().any(|b| b.block == p.block) {
                seen.push(p.block.clone());
            }
        }
        seen
    };
    let add_value = if free_blocks.contains(&add_pick()) { add_pick() } else { free_blocks.first().cloned().unwrap_or_default() };
    let naming_now = naming();
    let asking_now = asking();
    let (can_copy, can_gen) = (!request().trim().is_empty(), !request().trim().is_empty() && !busy() && !no_key);
    let build_copy = build.clone();
    let remember_copy = remember.clone();
    let remember_gen = remember.clone();
    let remember_close = remember.clone();
    let remember_close2 = remember.clone();
    let mode_label = length_mode.map(|m| if m == "words" { ui.t("lblDlgCardWords") } else { ui.t("lblDlgCardChars") });
    let step = if length_mode == Some("words") { 10.0 } else { 50.0 };

    rsx! {
        div {
            class: "uis-dl-backdrop ap-nested",
            onmousedown: move |e| {
                e.stop_propagation();
                remember_close();
                on_close.call(());
            },
            div { class: "uis-dlg", style: "width:min(560px,92vw)", onmousedown: move |e| e.stop_propagation(),
                div { class: "uis-dlg-head",
                    span { class: "uis-dlg-title",
                        Icon { width: 16, height: 16, icon: LdSparkles }
                        "{lbl_title}"
                    }
                    button {
                        class: "uis-btn",
                        onclick: move |_| {
                            let next = !expert();
                            expert.set(next);
                        },
                        "{btn_mode}"
                    }
                    button {
                        class: "uis-dl-close",
                        onclick: move |_| {
                            remember_close2();
                            on_close.call(());
                        },
                        "×"
                    }
                }
                div { class: "uis-dlg-body ap-dlg-body",
                    // ── the template ──
                    div { class: "ap-row",
                        label { class: "uis-fld-labelled-label", "{lbl_mode}" }
                        select {
                            class: "uis-ddn-std ap-grow",
                            value: "{chosen}",
                            onchange: move |e| {
                                chosen.set(e.value());
                                load_template(&e.value());
                            },
                            for t in templates.read().iter() {
                                option { key: "{t.id}", value: "{t.id}", selected: t.id == chosen(), "{t.name}" }
                            }
                        }
                        button {
                            class: small,
                            title: "{t_add_tpl}",
                            onclick: move |_| {
                                naming.set(Some((Naming::NewTemplate, String::new())));
                            },
                            Icon { width: 14, height: 14, icon: LdPlus }
                        }
                        button {
                            class: small,
                            title: "{t_ren}",
                            disabled: cur_default,
                            onclick: move |_| {
                                let name = templates.peek().iter().find(|t| t.id == *chosen.peek()).map(|t| t.name.clone()).unwrap_or_default();
                                naming.set(Some((Naming::RenameTemplate, name)));
                            },
                            Icon { width: 14, height: 14, icon: LdTextCursor }
                        }
                        button {
                            class: small,
                            title: "{t_save}",
                            disabled: !template_modified,
                            onclick: move |_| {
                                let id = chosen.peek().clone();
                                let rows = blocks.peek().clone();
                                for b in &rows {
                                    if let Err(e) = ui.st().set_ai_template_block(&id, &b.block, b.preset.as_deref(), b.enabled, Some(&b.name), Some(&b.prefix)) {
                                        ui.fail(e);
                                        return;
                                    }
                                }
                                template_blocks.set(snapshot(&rows));
                                reload_templates();
                                ui.ok(ui.t("tstDlgAiTemplateSaved"));
                            },
                            Icon { width: 14, height: 14, icon: LdSave }
                        }
                        button {
                            class: small,
                            title: "{t_del}",
                            disabled: cur_default,
                            onclick: move |_| asking.set(Some(Asking::DeleteTemplate)),
                            Icon { width: 14, height: 14, icon: LdTrash2 }
                        }
                        button {
                            class: if is_default_tpl { format!("{small} ap-on-star") } else { small.to_string() },
                            title: "{t_def}",
                            onclick: move |_| {
                                let id = chosen.peek().clone();
                                if id.is_empty() {
                                    return;
                                }
                                ui.st().set_setting(&format!("ai-default-template-{task}"), &id);
                                default_id.set(id);
                            },
                            Icon { width: 14, height: 14, icon: LdStar, style: if is_default_tpl { "fill:var(--uis-star)" } else { "fill:none" } }
                        }
                    }
                    if let Some((what, text)) = naming_now.clone() {
                        if matches!(what, Naming::NewTemplate | Naming::RenameTemplate) {
                            NameRow {
                                value: text,
                                on_input: move |v: String| set_naming_text(v),
                                on_ok: move |_| confirm_naming(),
                                on_cancel: move |_| naming.set(None),
                            }
                        }
                    }
                    if asking_now == Some(Asking::DeleteTemplate) {
                        AskRow {
                            question: ui.t("cfmDlgAiDeleteTemplateQ"),
                            yes: ui.t("btnCardDelete"),
                            no: ui.t("btnGlbCancel"),
                            on_yes: move |_| {
                                let id = chosen.peek().clone();
                                match ui.st().delete_ai_template(&id) {
                                    Ok(_) => {
                                        if *default_id.peek() == id {
                                            ui.st().set_setting(&format!("ai-default-template-{task}"), "");
                                            default_id.set(String::new());
                                        }
                                        reload_templates();
                                        let next = templates.peek().first().map(|t| t.id.clone()).unwrap_or_default();
                                        chosen.set(next.clone());
                                        load_template(&next);
                                        ui.ok(ui.t("tstDlgAiTemplateDeleted"));
                                    }
                                    Err(e) => ui.fail(e),
                                }
                                asking.set(None);
                            },
                            on_no: move |_| asking.set(None),
                        }
                    }

                    // ── the blocks (expert) ──
                    if expert() {
                        div { class: "uis-fld-labelled",
                            label { class: "uis-fld-labelled-label", "{ttl_blocks}" }
                            div { class: "ap-ai-blocks",
                                for b in blocks_now.iter().cloned() {
                                    {
                                        let block = b.block.clone();
                                        let of_block: Vec<Preset> = presets_now.iter().filter(|p| p.block == block).cloned().collect();
                                        let cur_preset = b.preset.as_ref().and_then(|id| of_block.iter().find(|p| &p.id == id)).cloned();
                                        let modified = match &cur_preset {
                                            Some(p) => b.content != p.content,
                                            None => !b.content.is_empty(),
                                        };
                                        let preset_is_default = cur_preset.as_ref().is_some_and(|p| p.is_default);
                                        let can_overwrite = modified && cur_preset.is_some() && !preset_is_default;
                                        let label = label_of(&b);
                                        let name_here = naming_now.clone().filter(|(w, _)| matches!(w, Naming::NewPreset(x) | Naming::RenamePreset(x) if *x == block));
                                        let delete_here = asking_now == Some(Asking::DeletePreset(block.clone()));
                                        let (b1, b2, b3, b4, b5, b6, b7, b8, b9) = (block.clone(), block.clone(), block.clone(), block.clone(), block.clone(), block.clone(), block.clone(), block.clone(), block.clone());
                                        rsx! {
                                            div { key: "{block}", class: "ap-ai-block", style: if b.enabled { "" } else { "opacity:.5" },
                                                div { class: "ap-ai-block-head",
                                                    button {
                                                        class: small,
                                                        title: "{s_tipDlgAiTweakBlock}",
                                                        onclick: move |_| {
                                                            if let Some(x) = blocks.write().iter_mut().find(|x| x.block == b1) {
                                                                x.open = !x.open;
                                                            }
                                                        },
                                                        Icon { width: 14, height: 14, icon: LdChevronDown }
                                                    }
                                                    span { class: "uis-hnt", "{label}" }
                                                    select {
                                                        class: "uis-ddn-std ap-grow",
                                                        disabled: !b.enabled,
                                                        value: "{b.preset.clone().unwrap_or_default()}",
                                                        onchange: move |e| {
                                                            let id = e.value();
                                                            let text = presets.peek().iter().find(|p| p.id == id).map(|p| p.content.clone()).unwrap_or_default();
                                                            if let Some(x) = blocks.write().iter_mut().find(|x| x.block == b2) {
                                                                x.preset = if id.is_empty() { None } else { Some(id) };
                                                                x.content = text;
                                                            }
                                                        },
                                                        for p in of_block.iter() {
                                                            option { key: "{p.id}", value: "{p.id}", selected: Some(&p.id) == b.preset.as_ref(), "{p.name}" }
                                                        }
                                                    }
                                                    button {
                                                        class: small,
                                                        title: "{s_tipDlgAiSaveAsNew}",
                                                        onclick: move |_| {
                                                            naming.set(Some((Naming::NewPreset(b3.clone()), String::new())));
                                                            if let Some(x) = blocks.write().iter_mut().find(|x| x.block == b3) {
                                                                x.open = true;
                                                            }
                                                        },
                                                        Icon { width: 14, height: 14, icon: LdPlus }
                                                    }
                                                    button {
                                                        class: small,
                                                        title: "{s_tipDlgAiRenameTemplate}",
                                                        disabled: cur_preset.is_none() || preset_is_default,
                                                        onclick: move |_| {
                                                            let name = blocks.peek().iter().find(|x| x.block == b4).and_then(|x| x.preset.clone()).and_then(|id| presets.peek().iter().find(|p| p.id == id).map(|p| p.name.clone())).unwrap_or_default();
                                                            naming.set(Some((Naming::RenamePreset(b4.clone()), name)));
                                                            if let Some(x) = blocks.write().iter_mut().find(|x| x.block == b4) {
                                                                x.open = true;
                                                            }
                                                        },
                                                        Icon { width: 14, height: 14, icon: LdTextCursor }
                                                    }
                                                    button {
                                                        class: small,
                                                        title: "{s_tipDlgAiOverwrite}",
                                                        disabled: !can_overwrite,
                                                        onclick: move |_| {
                                                            let row = blocks.peek().iter().find(|x| x.block == b5).cloned();
                                                            if let Some((id, text)) = row.and_then(|r| r.preset.map(|id| (id, r.content))) {
                                                                match ui.st().update_ai_preset(&id, None, Some(&text)) {
                                                                    Ok(_) => {
                                                                        reload_presets();
                                                                        ui.ok(ui.t("tstDlgAiPresetSaved"));
                                                                    }
                                                                    Err(e) => ui.fail(e),
                                                                }
                                                            }
                                                        },
                                                        Icon { width: 14, height: 14, icon: LdSave }
                                                    }
                                                    button {
                                                        key: "tog-{b.enabled}",
                                                        class: small,
                                                        title: "{s_tipDlgAiToggleBlock}",
                                                        style: if b.enabled { "color:var(--uis-ok)" } else { "" },
                                                        onclick: move |_| {
                                                            if let Some(x) = blocks.write().iter_mut().find(|x| x.block == b6) {
                                                                x.enabled = !x.enabled;
                                                            }
                                                        },
                                                        if b.enabled {
                                                            Icon { width: 14, height: 14, icon: LdCheck }
                                                        } else {
                                                            Icon { width: 14, height: 14, icon: LdX }
                                                        }
                                                    }
                                                    button {
                                                        class: small,
                                                        title: "{s_tipDlgAiRemoveBlock}",
                                                        disabled: cur_default,
                                                        onclick: move |_| {
                                                            let id = chosen.peek().clone();
                                                            match ui.st().remove_ai_template_block(&id, &b7) {
                                                                Ok(_) => {
                                                                    blocks.write().retain(|x| x.block != b7);
                                                                    template_blocks.set(snapshot(&blocks.peek()));
                                                                    reload_templates();
                                                                }
                                                                Err(e) => ui.fail(e),
                                                            }
                                                        },
                                                        Icon { width: 14, height: 14, icon: LdTrash2 }
                                                    }
                                                }
                                                if b.open {
                                                    div { class: "ap-ai-block-body",
                                                        if let Some((what, text)) = name_here {
                                                            {
                                                                let _ = what;
                                                                rsx! {
                                                                    NameRow {
                                                                        value: text,
                                                                        on_input: move |v: String| set_naming_text(v),
                                                                        on_ok: move |_| confirm_naming(),
                                                                        on_cancel: move |_| naming.set(None),
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        if delete_here {
                                                            AskRow {
                                                                question: ui.t("cfmDlgAiDeletePresetQ"),
                                                                yes: ui.t("btnCardDelete"),
                                                                no: ui.t("btnGlbCancel"),
                                                                on_yes: {
                                                                    let block = block.clone();
                                                                    move |_| {
                                                                        let id = blocks.peek().iter().find(|x| x.block == block).and_then(|x| x.preset.clone());
                                                                        if let Some(id) = id {
                                                                            match ui.st().delete_ai_preset(&id) {
                                                                                Ok(_) => {
                                                                                    reload_presets();
                                                                                    let next = presets.peek().iter().find(|p| p.block == block).cloned();
                                                                                    if let Some(x) = blocks.write().iter_mut().find(|x| x.block == block) {
                                                                                        x.preset = next.as_ref().map(|p| p.id.clone());
                                                                                        x.content = next.map(|p| p.content).unwrap_or_default();
                                                                                    }
                                                                                    ui.ok(ui.t("tstDlgAiPresetDeleted"));
                                                                                }
                                                                                Err(e) => ui.fail(e),
                                                                            }
                                                                        }
                                                                        asking.set(None);
                                                                    }
                                                                },
                                                                on_no: move |_| asking.set(None),
                                                            }
                                                        }
                                                        textarea {
                                                            class: "uis-txt ap-area",
                                                            rows: "4",
                                                            value: "{b.content}",
                                                            oninput: move |e| {
                                                                if let Some(x) = blocks.write().iter_mut().find(|x| x.block == b8) {
                                                                    x.content = e.value();
                                                                }
                                                            },
                                                        }
                                                        if cur_preset.is_some() && !preset_is_default {
                                                            div { class: "ap-ai-del",
                                                                button {
                                                                    class: "uis-btn small subtle",
                                                                    title: "{s_tipDlgAiDeletePreset}",
                                                                    onclick: move |_| {
                                                                        asking.set(Some(Asking::DeletePreset(b9.clone())));
                                                                    },
                                                                    Icon { width: 14, height: 14, icon: LdTrash2 }
                                                                    span { "{s_btnDlgAiDeletePreset}" }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                if !cur_default && !free_blocks.is_empty() {
                                    div { class: "ap-row",
                                        select { class: "uis-ddn-std ap-grow", value: "{add_value}", onchange: move |e| add_pick.set(e.value()),
                                            for f in free_blocks.iter() {
                                                option { key: "{f}", value: "{f}", selected: *f == add_value, "{block_label(f)}" }
                                            }
                                        }
                                        button {
                                            class: "uis-btn",
                                            title: "{s_tipDlgAiAddBlock}",
                                            onclick: {
                                                let add_value = add_value.clone();
                                                move |_| {
                                                    let block = add_value.clone();
                                                    let first = presets.peek().iter().find(|p| p.block == block).cloned();
                                                    let name = block_label(&block);
                                                    let id = chosen.peek().clone();
                                                    match ui.st().set_ai_template_block(&id, &block, first.as_ref().map(|p| p.id.as_str()), true, Some(&name), Some(&block.to_uppercase())) {
                                                        Ok(_) => {
                                                            blocks.write().push(BlockState { block: block.clone(), name, prefix: block.to_uppercase(), preset: first.as_ref().map(|p| p.id.clone()), content: first.map(|p| p.content).unwrap_or_default(), enabled: true, open: true });
                                                            template_blocks.set(snapshot(&blocks.peek()));
                                                            reload_templates();
                                                        }
                                                        Err(e) => ui.fail(e),
                                                    }
                                                }
                                            },
                                            Icon { width: 14, height: 14, icon: LdPlus }
                                            span { "{s_btnDlgAiAddBlock}" }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // ── the variables the blocks ask for ──
                    if !all_vars.is_empty() {
                        div { class: "uis-fld-labelled",
                            label { class: "uis-fld-labelled-label", "{ttl_vars}" }
                            div { class: "ap-ai-vars",
                                for v in all_vars.iter().cloned() {
                                    {
                                        let name = v.name.clone();
                                        let given = values.read().get(&v.name).cloned();
                                        rsx! {
                                            div { key: "{v.name}", class: "uis-fld-labelled",
                                                label { class: "uis-fld-labelled-label", "{v.name}" }
                                                if v.kind == "ckb" {
                                                    label { class: "uis-chk-lbl",
                                                        input {
                                                            r#type: "checkbox",
                                                            class: "uis-chk-box",
                                                            checked: given.as_deref() != Some("off"),
                                                            onchange: move |e| {
                                                                values.write().insert(name.clone(), if e.checked() { "on".to_owned() } else { "off".to_owned() });
                                                            },
                                                        }
                                                        span { "{v.default}" }
                                                    }
                                                } else {
                                                    input {
                                                        class: "uis-txt",
                                                        r#type: if v.kind == "num" { "number" } else { "text" },
                                                        value: "{given.clone().unwrap_or_else(|| v.default.clone())}",
                                                        oninput: move |e| {
                                                            values.write().insert(name.clone(), e.value());
                                                        },
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // ── the request ──
                    div { class: "uis-fld-labelled",
                        label { class: "uis-fld-labelled-label", "{lbl_request}" }
                        textarea {
                            class: "uis-txt ap-area",
                            rows: "4",
                            placeholder: "{placeholder}",
                            value: "{request}",
                            onmounted: move |e| async move {
                                let _ = e.set_focus(true).await;
                            },
                            oninput: move |e| request.set(e.value()),
                        }
                    }
                    if no_key {
                        div { class: "ap-hint", "{s_msgStgAiApiKeyGetKey}" }
                    }
                }
                div { class: "uis-dlg-foot",
                    if let Some(unit) = mode_label {
                        UisNum {
                            value: Some(limit() as f64),
                            width: "72px",
                            min: Some(step),
                            step,
                            decimals: Some(0),
                            on_change: move |v: Option<f64>| {
                                if let Some(v) = v {
                                    limit.set(v.round() as i64);
                                }
                            },
                        }
                        span { class: "uis-hnt", "{unit}" }
                    }
                    div { class: "ap-grow" }
                    button {
                        class: "uis-btn",
                        title: "{tip_copy}",
                        disabled: !can_copy,
                        onclick: move |_| {
                            remember_copy();
                            let _ = arboard::Clipboard::new().and_then(|mut b| b.set_text(build_copy()));
                            ui.ok(ui.t("tstDlgAiCardCopied"));
                        },
                        Icon { width: 14, height: 14, icon: LdCopy }
                    }
                    button {
                        class: "uis-btn primary",
                        title: "{tip_gen}",
                        disabled: !can_gen,
                        onclick: move |_| {
                            remember_gen();
                            let text = build();
                            busy.set(true);
                            let api_key = ui.st().setting("gemini-key");
                            let gen_kind = gen_kind.to_owned();
                            spawn(async move {
                                let got = tokio::task::spawn_blocking(move || ai::generate(&Generate { request: text, kind: gen_kind, api_key, raw: true, max_tokens: Some(8192), ..Default::default() })).await;
                                busy.set(false);
                                match got {
                                    Ok(Ok(g)) => {
                                        let read = ai::read_answer(&g.text, 0);
                                        on_done.call((read.text, read.name_suggestions));
                                        on_close.call(());
                                    }
                                    Ok(Err(e)) => ui.fail(format!("{}: {e}", ui.t("tstAppAiError"))),
                                    Err(e) => ui.fail(e.to_string()),
                                }
                            });
                        },
                        if busy() { "…" } else { "{tip_gen}" }
                    }
                }
            }
        }
    }
}

/// The little row where a name is typed: the field and its two buttons.
#[component]
fn NameRow(value: String, on_input: EventHandler<String>, on_ok: EventHandler<()>, on_cancel: EventHandler<()>) -> Element {
    let ok = !value.trim().is_empty();
    rsx! {
        div { class: "ap-row",
            input {
                class: "uis-txt ap-grow",
                value: "{value}",
                onmounted: move |e| async move {
                    let _ = e.set_focus(true).await;
                },
                oninput: move |e| on_input.call(e.value()),
                onkeydown: move |e| match e.key() {
                    Key::Enter => on_ok.call(()),
                    Key::Escape => on_cancel.call(()),
                    _ => {}
                },
            }
            button { class: if ok { "uis-btn ico small subtle ap-on-ok" } else { "uis-btn ico small subtle" }, disabled: !ok, onclick: move |_| on_ok.call(()),
                Icon { width: 14, height: 14, icon: LdCheck }
            }
            button { class: "uis-btn ico small subtle", onclick: move |_| on_cancel.call(()),
                Icon { width: 14, height: 14, icon: LdX }
            }
        }
    }
}

/// A question under a row, with the two answers.
#[component]
fn AskRow(question: String, yes: String, no: String, on_yes: EventHandler<()>, on_no: EventHandler<()>) -> Element {
    rsx! {
        div { class: "ap-ai-ask",
            span { class: "ap-grow", "{question}" }
            button { class: "uis-btn primary", style: "--uis-accent:var(--uis-danger)", onclick: move |_| on_yes.call(()), "{yes}" }
            button { class: "uis-btn subtle", onclick: move |_| on_no.call(()), "{no}" }
        }
    }
}
