// ⚠ NO HARDCODED UI STRINGS IN THIS FILE.
//   Every string a user can read MUST go through the t() lookup, and the key MUST be a string literal at the call site.

//! Dragging inside the window, and what the system drops on it.
//!
//! ⚠ in-page drags are made of POINTER EVENTS, not of HTML5 drag and drop. The window's own file-drop handler
//! (Dioxus desktop on Windows) swallows the page's HTML5 drags, so `draggable` rows never start one. A press on a
//! draggable thing only ARMS a drag; moving the pointer 5 pixels turns it into a live one; targets light up from
//! `mouseenter` while it lives; the release on the root performs the drop on the target under the pointer.
//! The word `Dragged` is what is carried, `DropAt` is where it can land.

use crate::{
    exchange,
    ui::{Nav, Ui},
    work,
};
use dioxus::html::input_data::MouseButton;
use dioxus::prelude::*;
use engine::{
    cards::CardPatch,
    media,
    projects::ProjectPatch,
};
use std::path::PathBuf;

/// What is being carried.
#[derive(Clone, PartialEq)]
pub enum Dragged {
    /// One or more cards (the selection when the dragged card is part of it).
    Cards(Vec<String>),
    Project(String),
    /// A media file of the Media view (its path).
    Media(String),
}

/// Where a drag can land.
#[derive(Clone, PartialEq)]
pub enum DropAt {
    Project(String),
    /// The "move to root level" strip, shown while a project is carried.
    Root,
    /// A song: the cards linked to it, or the media file it plays.
    Song(String),
    /// A row of the Sort view: put the carried song before or after it.
    Sort { id: String, after: bool },
}

#[derive(Clone, Default, PartialEq)]
pub struct DragState {
    /// Pressed, not yet moved far enough: what, its label, where the pointer was.
    pub arm: Option<(Dragged, String, f64, f64)>,
    /// Being dragged now: what and its label.
    pub live: Option<(Dragged, String)>,
    pub over: Option<DropAt>,
}

/// The pointer has to travel this far before a press becomes a drag (so a click stays a click).
const THRESHOLD: f64 = 5.0;

/// The cards a drag of this card carries: the whole selection when the card is in it, else the card.
pub fn cards_of(ui: Ui, id: &str) -> Vec<String> {
    let selected = ui.selected_ids();
    if selected.len() > 1 && selected.iter().any(|s| s == id) { selected } else { vec![id.to_owned()] }
}

impl Ui {
    pub fn arm_drag(self, what: Dragged, label: String, e: &MouseEvent) {
        if e.trigger_button() != Some(MouseButton::Primary) {
            return;
        }
        let at = e.client_coordinates();
        let mut drag = self.drag;
        drag.write().arm = Some((what, label, at.x, at.y));
    }

    pub fn dragging(self) -> bool {
        self.drag.read().live.is_some()
    }

    /// The pointer moved anywhere in the window.
    pub fn drag_move(self, e: &MouseEvent) {
        let (armed, live) = {
            let d = self.drag.peek();
            (d.arm.clone(), d.live.is_some())
        };
        if armed.is_none() && !live {
            return;
        }
        // the button was released where the window did not hear it: the drag is over
        if !e.held_buttons().contains(MouseButton::Primary) {
            let mut drag = self.drag;
            drag.set(DragState::default());
            return;
        }
        let at = e.client_coordinates();
        if !live {
            let Some((what, label, x, y)) = armed else { return };
            if (at.x - x).hypot(at.y - y) < THRESHOLD {
                return;
            }
            let mut drag = self.drag;
            let mut d = drag.write();
            d.arm = None;
            d.live = Some((what, label));
        }
        let mut pos = self.drag_at;
        pos.set((at.x, at.y));
    }

    /// Can what is carried land on this?
    pub fn drag_accepts(self, at: &DropAt) -> bool {
        let Some((what, _)) = self.drag.read().live.clone() else { return false };
        match (what, at) {
            (Dragged::Cards(_), DropAt::Project(_)) => true,
            (Dragged::Cards(ids), DropAt::Song(song)) => {
                !ids.contains(song) && ids.iter().all(|id| self.card(id).is_some_and(|c| c.kind == "music" || c.kind == "vocal")) && self.card(song).is_some_and(|c| c.kind == "song")
            }
            (Dragged::Cards(ids), DropAt::Sort { id, .. }) => ids.len() == 1 && ids[0] != *id && *self.view.peek() == crate::ui::View::Sort,
            (Dragged::Project(p), DropAt::Project(t)) => p != *t && !self.project_tree(&p).contains(t),
            (Dragged::Project(p), DropAt::Root) => self.projects.read().iter().any(|x| x.id == p && x.parent_id.is_some()),
            (Dragged::Media(_), DropAt::Song(_)) => true,
            _ => false,
        }
    }

    pub fn drag_over(self, at: DropAt) {
        if self.drag_accepts(&at) && self.drag.peek().over.as_ref() != Some(&at) {
            let mut drag = self.drag;
            drag.write().over = Some(at);
        }
    }

    /// The pointer left a target. A Sort target is the same one whichever half it is over.
    pub fn drag_leave(self, at: &DropAt) {
        let same = match (&self.drag.peek().over, at) {
            (Some(DropAt::Sort { id: a, .. }), DropAt::Sort { id: b, .. }) => a == b,
            (Some(o), at) => o == at,
            (None, _) => false,
        };
        if same {
            let mut drag = self.drag;
            drag.write().over = None;
        }
    }

    pub fn is_over(self, at: &DropAt) -> bool {
        self.drag.read().over.as_ref() == Some(at)
    }

    /// The release: what is carried lands on what is under the pointer, if it accepts it.
    pub fn drag_end(self) {
        let state = self.drag.peek().clone();
        let mut drag = self.drag;
        drag.set(DragState::default());
        let (Some((what, _)), Some(at)) = (state.live, state.over) else { return };
        match (what, at) {
            (Dragged::Cards(ids), DropAt::Project(p)) => {
                for id in &ids {
                    if let Err(e) = self.st().add_card_to_project(&p, id) {
                        return self.fail(e);
                    }
                }
                self.reload();
                self.ok(self.t("tstAppAddedToProject"));
            }
            (Dragged::Project(p), DropAt::Project(t)) => self.move_project(&p, Some(t)),
            (Dragged::Project(p), DropAt::Root) => self.move_project(&p, None),
            (Dragged::Cards(ids), DropAt::Song(song)) => {
                if work::finalized(self) {
                    return;
                }
                let Some(card) = self.card(&song) else { return };
                let mut links = card.linked_cards.clone();
                for id in &ids {
                    if !links.contains(id) {
                        links.push(id.clone());
                    }
                }
                match self.st().update_card(&song, &CardPatch { linked_cards: Some(links), ..Default::default() }) {
                    Ok(_) => {
                        self.reload();
                        self.ok(format!("{} {}", ids.len(), self.t("tstAppCardsLinked")));
                    }
                    Err(e) => self.fail(e),
                }
            }
            (Dragged::Media(path), DropAt::Song(song)) => {
                if work::finalized(self) {
                    return;
                }
                match self.st().update_card(&song, &CardPatch { media_path: Some(path), ..Default::default() }) {
                    Ok(_) => self.reload(),
                    Err(e) => self.fail(e),
                }
            }
            (Dragged::Cards(ids), DropAt::Sort { id, after }) => {
                if work::finalized(self) {
                    return;
                }
                work::reorder_song(self, &ids[0], &id, after);
            }
            _ => {}
        }
    }

    fn move_project(self, project: &str, parent: Option<String>) {
        match self.st().update_project(project, &ProjectPatch { parent_id: Some(parent.clone()), ..Default::default() }) {
            Ok(_) => {
                if let Some(p) = parent {
                    let mut expanded = self.expanded;
                    expanded.write().insert(p);
                }
                self.reload();
            }
            Err(e) => self.fail(e),
        }
    }

    /// Files dropped on the window by the system: a `.amlp` is imported (into the open project, as the Import
    /// button does); anything else is expanded to its tracks: one opens the card editor on it, several are made into
    /// cards at once. A drop with nothing playable in it does nothing.
    pub fn os_drop(self, files: Vec<PathBuf>) {
        if let Some(lp) = files.iter().find(|f| f.extension().is_some_and(|e| e.eq_ignore_ascii_case("amlp"))) {
            match std::fs::read_to_string(lp) {
                Ok(text) => exchange::import_amlp_text(self, &text),
                Err(e) => self.fail(e.to_string()),
            }
            return;
        }
        let mut tracks = media::expand_drop(&files);
        match tracks.len() {
            0 => {}
            1 => self.open_card_from_media(tracks.remove(0)),
            _ => {
                let project = match self.nav.peek().clone() {
                    Nav::Project(id) => Some(id),
                    _ => None,
                };
                spawn(async move { self.create_cards_from_media(tracks, project).await });
            }
        }
    }
}
