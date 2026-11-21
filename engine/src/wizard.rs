//! The song wizard's two halves that are not the window: building the request of one wizard step out of the templates, presets
//! and system blocks, and reading the model's answer (`KEY: value` lines, the value inline or on the lines after).

use crate::store::{Res, Store};
use regex::Regex;
use serde_json::Value;
use std::{collections::HashMap, sync::OnceLock};

/// Read an answer into its keys. A key is an UPPER_CASE word at the start of a line followed by a colon, so a line such as
/// `Tiken Jah: Fakoly` never starts one. Code fences the model wrapped the answer in are dropped.
pub fn parse_response(raw: &str) -> HashMap<String, String> {
    static RES: OnceLock<[Regex; 3]> = OnceLock::new();
    let [open_fence, close_fence, key_line] = RES.get_or_init(|| [Regex::new(r"(?m)^```[^\n]*\n?").unwrap(), Regex::new(r"(?m)```$").unwrap(), Regex::new(r"^([A-Z][A-Z0-9_]*):\s*(.*)$").unwrap()]);
    let clean = open_fence.replace_all(raw, "");
    let clean = close_fence.replace_all(&clean, "");
    let mut bag = HashMap::new();
    let mut current: Option<(String, Vec<String>)> = None;
    for line in clean.trim().split('\n') {
        let line = line.strip_suffix('\r').unwrap_or(line);
        if let Some(m) = key_line.captures(line) {
            if let Some((key, lines)) = current.take() {
                bag.insert(key, lines.join("\n").trim().to_owned());
            }
            let first = m[2].to_owned();
            current = Some((m[1].to_owned(), if first.is_empty() { Vec::new() } else { vec![first] }));
        } else if let Some((_, lines)) = current.as_mut() {
            lines.push(line.to_owned());
        }
    }
    if let Some((key, lines)) = current {
        bag.insert(key, lines.join("\n").trim().to_owned());
    }
    bag
}

/// The request of one wizard task: the enabled blocks of the task's template with their presets, then the system blocks of the task
/// (its fixed text, or its variables filled from `vars`, else from the example).
pub fn task_request(store: &Store, system: &Value, task: &str, vars: &HashMap<&str, String>) -> Res<String> {
    let templates = store.ai_templates(Some(task))?;
    let presets = store.ai_presets(Some(task), None)?;
    let Some(template) = templates.first() else { return Ok(String::new()) };
    let mut parts: Vec<String> = Vec::new();
    for b in template.blocks.iter().filter(|b| b.enabled) {
        let Some(preset) = b.preset_id.as_ref().and_then(|id| presets.iter().find(|p| &p.id == id)) else { continue };
        if preset.content.trim().is_empty() {
            continue;
        }
        let prefix = if b.prefix.is_empty() { format!("{}: ", b.block.to_uppercase()) } else { b.prefix.clone() };
        parts.push(format!("{prefix}{}", preset.content.trim()));
    }
    for sys in system.as_array().into_iter().flatten() {
        let block = sys["block"].as_str().unwrap_or("");
        let prefix = sys["prefix"].as_str().filter(|p| !p.is_empty()).map(str::to_owned).unwrap_or_else(|| format!("{}: ", block.to_uppercase()));
        let for_task = &sys["tasks"][task];
        if let Some(content) = for_task["content"].as_str().filter(|c| !c.is_empty()) {
            parts.push(format!("{prefix}{content}"));
        }
        if let Some(list) = for_task["vars"].as_array() {
            let lines: Vec<String> = list
                .iter()
                .map(|v| {
                    let name = v["name"].as_str().unwrap_or("");
                    let value = vars.get(name).filter(|g| !g.is_empty()).cloned().unwrap_or_else(|| v["example"].as_str().unwrap_or("").to_owned());
                    format!("{name}: {value}")
                })
                .collect();
            parts.push(format!("{prefix}{}", lines.join("\n")));
        }
    }
    Ok(parts.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_answer_is_read_by_its_upper_case_keys() {
        let bag = parse_response("```\nARTIST: Tiken Jah Fakoly\nSTANCE:\nDefiance\nof the powerful\nSUBJECTS:\n1. Roots\n2. Exile\nTiken Jah: Fakoly stays text\n```");
        assert_eq!(bag["ARTIST"], "Tiken Jah Fakoly");
        assert_eq!(bag["STANCE"], "Defiance\nof the powerful");
        assert_eq!(bag["SUBJECTS"], "1. Roots\n2. Exile\nTiken Jah: Fakoly stays text");
    }
}

#[cfg(test)]
mod store_tests {
    use super::*;

    #[test]
    fn a_step_request_holds_its_blocks_and_the_artist() {
        let dir = std::env::temp_dir().join(format!("ampl-wizard-{}", crate::store::new_id()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Store::open(&dir.join("t.db"), &dir.join("s.json")).unwrap();
        store.reconcile_ai_seed().unwrap();
        let wizards = store.ai_wizards().unwrap();
        let world = wizards.iter().find(|w| w.steps.iter().any(|s| s.category == "wzd-note")).expect("a wizard with a note step");
        let note = world.steps.iter().find(|s| s.category == "wzd-note").unwrap();
        let vars = HashMap::from([("ARTIST", "Idir".to_owned())]);
        let request = task_request(&store, crate::ai::system_blocks(), &note.task, &vars).unwrap();
        assert!(request.contains("ROLE: Ethnomusicologist"), "{request}");
        assert!(request.contains("ARTIST: Idir"), "{request}");
        // a variable that is not given takes its example
        let lyrics = world.steps.iter().find(|s| s.category == "wzd-lyrics").unwrap();
        let request = task_request(&store, crate::ai::system_blocks(), &lyrics.task, &vars).unwrap();
        assert!(request.contains("ARTIST: Idir") && request.contains("LANGUAGE: Tamazight"), "{request}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
