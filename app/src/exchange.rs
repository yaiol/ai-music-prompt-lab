// ⚠ NO HARDCODED UI STRINGS IN THIS FILE.
//   Every string a user can read MUST go through the t() lookup, and the key MUST be a string literal at the call site.

//! Files in and out: the JSON and `.amlp` imports, and every export. The engine builds the content; here the
//! window asks for the file and says what happened.

use crate::{
    APP_VERSION,
    ui::{Dialog, Nav, Ui},
};
use dioxus::prelude::*;
use engine::{
    exports::{DocOptions, NoSongs},
    transfer::AmlpRefused,
};
use std::{fs, path::PathBuf};

fn parent() -> rfd::AsyncFileDialog {
    rfd::AsyncFileDialog::new().set_parent(&dioxus::desktop::window().window)
}

fn day() -> String {
    engine::dates::today()
}

/// Ask where to write, then write. `dir` is the folder the dialog opens in ("" = the system's choice).
async fn save(ui: Ui, name: String, dir: String, label: &str, ext: &str, content: impl AsRef<[u8]>) -> Option<PathBuf> {
    let mut dialog = parent().set_file_name(&name).add_filter(label, &[ext]);
    if !dir.is_empty() && std::path::Path::new(&dir).is_dir() {
        dialog = dialog.set_directory(&dir);
    }
    let file = dialog.save_file().await?;
    match fs::write(file.path(), content) {
        Ok(()) => Some(file.path().to_owned()),
        Err(e) => {
            ui.fail(e.to_string());
            None
        }
    }
}

async fn pick(ext: &str, label: &str) -> Option<(PathBuf, String)> {
    let file = parent().add_filter(label, &[ext]).pick_file().await?;
    let text = fs::read_to_string(file.path()).ok()?;
    Some((file.path().to_owned(), text))
}

pub fn import_json(ui: Ui) {
    spawn(async move {
        let Some((_, text)) = pick("json", "JSON").await else { return };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else { return ui.fail(ui.t("tstAppImportJSONJsonInvalid")) };
        // a file with neither cards / projects nor AI presets / templates is not one of ours (as in the Electron app)
        let has_cards = !value["cards"].is_null() || !value["projects"].is_null();
        let has_ai = !value["aiPresets"].is_null() || !value["aiTemplates"].is_null();
        if !has_cards && !has_ai {
            return ui.fail(ui.t("tstAppImportJSONJsonInvalid"));
        }
        let env = ui.env.peek().clone();
        match ui.st().import_json(&value, Some(&env)) {
            Ok(r) => {
                ui.reload();
                if has_ai && !has_cards {
                    return ui.ok(ui.t("tstAppImportJSONAiImported"));
                }
                let mut lines: Vec<String> = Vec::new();
                let mut add = |n: usize, key: String| {
                    if n > 0 {
                        lines.push(format!("{n} {key}"));
                    }
                };
                add(r.cards_created, ui.t("msgAppImportCardsCreated"));
                add(r.cards_updated, ui.t("msgAppImportJSONCardsUpdated"));
                add(r.cards_skipped, ui.t("msgAppImportJSONCardsUnchanged"));
                add(r.projects_created, ui.t("msgAppImportJSONProjectsCreated"));
                add(r.projects_updated, ui.t("msgAppImportJSONProjectsUpdated"));
                add(r.song_links_added, ui.t("msgAppImportJSONSongLinksAdded"));
                add(r.song_urls_added, ui.t("msgAppImportJSONSongUrlsAdded"));
                if lines.is_empty() && r.validation_errors.is_empty() {
                    return ui.ok(ui.t("tstAppImportJSONNothing"));
                }
                if value["type"] == "backup" {
                    ui.ok(ui.t("tstAppImportJSONBackupRestored"));
                }
                if !r.validation_errors.is_empty() {
                    lines.push(String::new());
                    lines.extend(r.validation_errors);
                    let mut dialog = ui.dialog;
                    dialog.set(Dialog::Info { title: ui.t("ttlAppImportJSONWarnings"), lines });
                } else if value["type"] != "backup" {
                    let mut dialog = ui.dialog;
                    dialog.set(Dialog::Info { title: ui.t("tipHdrImport"), lines });
                }
            }
            Err(e) => ui.fail(format!("{}: {e}", ui.t("tstAppImportJSONJsonFailed"))),
        }
    });
}

pub fn import_amlp(ui: Ui) {
    spawn(async move {
        let Some((_, text)) = pick("amlp", ".amlp").await else { return };
        import_amlp_text(ui, &text);
    });
}

pub fn import_amlp_text(ui: Ui, text: &str) {
    let env = ui.env.peek().clone();
    let project = match ui.nav.peek().clone() {
        Nav::Project(p) => Some(p),
        _ => None,
    };
    match ui.st().import_amlp(text, Some(&env), project.as_deref()) {
        Ok(Ok(r)) => {
            ui.reload();
            ui.ok(format!("{} - {} {}", r.project_name, r.tracks, ui.t("msgAppExportAmlpSongs")));
        }
        Ok(Err(AmlpRefused::GlobalEnv)) => ui.fail(ui.t("tstAppImportAmlpGlobalEnv")),
        Ok(Err(AmlpRefused::NoTracks)) => ui.fail(ui.t("tstAppImportAmlpNoTracks")),
        Ok(Err(AmlpRefused::Empty)) => ui.fail(ui.t("tstAppImportAmlpParse")),
        Err(e) => ui.fail(format!("{}: {e}", ui.t("tstAppImportAmlpFailed"))),
    }
}

/// What an export is about: the ticked cards, or everything in view.
fn in_view(ui: Ui) -> Vec<String> {
    let picked = ui.selected_ids();
    if picked.is_empty() { ui.shown_cards().into_iter().map(|c| c.id).collect() } else { picked }
}

pub fn export_cards(ui: Ui) {
    spawn(async move {
        let ids = in_view(ui);
        let project_ids: Vec<String> = match ui.nav.peek().clone() {
            Nav::Project(p) => ui.project_tree(&p),
            _ => ui.env_projects().into_iter().map(|p| p.id).collect(),
        };
        match ui.st().export_json(&ids, &project_ids, false, APP_VERSION) {
            Ok(v) => {
                let text = serde_json::to_string_pretty(&v).unwrap_or_default();
                save(ui, format!("ampl-cards-{}.json", day()), String::new(), "JSON", "json", text).await;
            }
            Err(e) => ui.fail(e),
        }
    });
}

pub fn export_backup(ui: Ui) {
    spawn(async move {
        let st = ui.st();
        let (Ok(cards), Ok(projects)) = (st.cards(), st.projects()) else { return };
        let (card_ids, project_ids): (Vec<String>, Vec<String>) = (cards.into_iter().map(|c| c.id).collect(), projects.into_iter().map(|p| p.id).collect());
        match st.export_json(&card_ids, &project_ids, true, APP_VERSION) {
            Ok(v) => {
                let text = serde_json::to_string_pretty(&v).unwrap_or_default();
                save(ui, format!("ampl-backup-{}.json", day()), String::new(), "JSON", "json", text).await;
            }
            Err(e) => ui.fail(e),
        }
    });
}

pub fn export_ai(ui: Ui) {
    spawn(async move {
        match ui.st().export_ai_json(APP_VERSION) {
            Ok(v) => {
                let text = serde_json::to_string_pretty(&v).unwrap_or_default();
                save(ui, format!("ampl-ai-templates-{}.json", day()), String::new(), "JSON", "json", text).await;
            }
            Err(e) => ui.fail(e),
        }
    });
}

pub fn export_amlp(ui: Ui) {
    spawn(async move {
        let ids: Vec<String> = in_view(ui).into_iter().filter(|id| ui.card(id).is_some_and(|c| c.kind == "song")).collect();
        let project = match ui.nav.peek().clone() {
            Nav::Project(p) => Some(p),
            _ => None,
        };
        match ui.st().export_amlp(&ids, project.as_deref()) {
            Ok(Some((content, name, dir, n))) => {
                if save(ui, format!("{name}.amlp"), dir, ".amlp", "amlp", content).await.is_some() {
                    ui.ok(format!("{n} {}", ui.t("msgAppExportAmlpSongs")));
                }
            }
            Ok(None) => ui.fail(ui.t("msgAppExportAmlpNone")),
            Err(e) => ui.fail(e),
        }
    });
}

/// The formats of a songbook.
#[derive(Clone, Copy, PartialEq)]
pub enum DocFormat {
    Docx,
    Md,
    Adoc,
}

/// The songbook: a Word file, Markdown or AsciiDoc.
pub fn export_doc(ui: Ui, o: DocOptions, format: DocFormat) {
    spawn(async move {
        let found = match format {
            DocFormat::Docx => ui.st().export_songbook_docx(&o),
            DocFormat::Md => ui.st().export_songbook(&o, false).map(|r| r.map(|(c, n, d)| (c.into_bytes(), n, d))),
            DocFormat::Adoc => ui.st().export_songbook(&o, true).map(|r| r.map(|(c, n, d)| (c.into_bytes(), n, d))),
        };
        match found {
            Ok(Ok((content, name, dir))) => {
                let (label, ext) = match format {
                    DocFormat::Docx => ("DOCX", "docx"),
                    DocFormat::Md => ("Markdown", "md"),
                    DocFormat::Adoc => ("AsciiDoc", "adoc"),
                };
                save(ui, format!("{name}.{ext}"), dir, label, ext, content).await;
            }
            Ok(Err(NoSongs::None | NoSongs::Filtered)) => ui.fail(ui.t("msgAppExportAmlpNone")),
            Err(e) => ui.fail(e),
        }
    });
}

/// The spreadsheet: the open project's cards, or everything when no project is open.
pub fn export_excel(ui: Ui) {
    spawn(async move {
        let (cards, projects, name): (Vec<_>, Vec<_>, String) = match ui.nav.peek().clone() {
            Nav::Project(id) => match ui.projects.peek().iter().find(|p| p.id == id).cloned() {
                Some(p) => (ui.cards.peek().iter().filter(|c| p.card_ids.contains(&c.id)).cloned().collect(), vec![p.clone()], format!("-{}", p.name)),
                None => return,
            },
            _ => (ui.cards.peek().clone(), ui.projects.peek().clone(), String::new()),
        };
        match engine::excel::workbook(&cards, &projects) {
            Ok(bytes) => {
                save(ui, format!("ampl{name}-{}.xlsx", day()), String::new(), "Excel", "xlsx", bytes).await;
            }
            Err(e) => ui.fail(e.to_string()),
        }
    });
}

/// Fill the open project's songs' music-site links from its playlist page. The page is read off the UI
/// thread; the store is only touched here.
pub fn sync_music_urls(ui: Ui, project: String) {
    spawn(async move {
        let plan = match ui.st().plan_sync(&project) {
            Ok(p) => p,
            Err(e) => return ui.fail(e),
        };
        let url = plan.playlist.clone();
        let tracks = match tokio::task::spawn_blocking(move || engine::soundcloud::fetch_playlist(&url)).await {
            Ok(Ok(t)) => t,
            Ok(Err(e)) => return ui.fail(e),
            Err(e) => return ui.fail(e.to_string()),
        };
        match ui.st().apply_sync(&project, &plan, &tracks) {
            Ok(r) => {
                ui.reload();
                ui.ok(ui.t("tstAppSyncUrlsDone").replace("{matched}", &r.matched.to_string()).replace("{total}", &r.total_songs.to_string()).replace("{added}", &r.added.to_string()).replace("{updated}", &r.updated.to_string()).replace("{skipped}", &r.skipped.to_string()));
            }
            Err(e) => ui.fail(e),
        }
    });
}
