// twin: uis-pnl-toggle.jsx jsx@88ba285babf7 rsx@978618cf873c
//! UisPnlToggle — twin of `uis-pnl-toggle.jsx`: the little button welded to a side panel's open
//! edge that shows/hides it. Accent when the panel is open, neutral when hidden, a chevron pointing
//! the collapse direction. It is ONLY the toggle - the UisPnlSplitter and the panel/content layout stay the
//! caller's. Same classes and element order, so the `.uis-pnl-toggle*` rules in `uis-app.css` skin it.
//!
//! Controlled: `open` + `on_toggle`. `side` is which side the panel is on: `"left"` (default - the button
//! on the panel's right edge, chevron left when open) or `"right"`. `title` is the tooltip, already
//! translated.
//! Differs: none.

use dioxus::prelude::*;
use dioxus_free_icons::{
    Icon,
    icons::ld_icons::{LdChevronLeft, LdChevronRight},
};

#[component]
pub fn UisPnlToggle(
    open: bool,
    on_toggle: EventHandler<()>,
    /// `"left"` (default) or `"right"`.
    #[props(default)] side: String,
    #[props(default)] title: String,
) -> Element {
    let left = side != "right";
    let side_name = if left { "left" } else { "right" };
    // The chevron points toward the collapse action: a left panel collapses left when open and expands
    // right when hidden; a right panel is the mirror.
    let points_left = left == open;
    let class = if open {
        format!("uis-pnl-toggle uis-pnl-toggle-{side_name} on")
    } else {
        format!("uis-pnl-toggle uis-pnl-toggle-{side_name}")
    };

    rsx! {
        div { class: "uis-pnl-toggle-wrap",
            button {
                r#type: "button",
                class: "{class}",
                title: if title.is_empty() { None } else { Some(title.clone()) },
                onclick: move |_| on_toggle.call(()),
                if points_left {
                    Icon { width: 14, height: 14, icon: LdChevronLeft }
                } else {
                    Icon { width: 14, height: 14, icon: LdChevronRight }
                }
            }
        }
    }
}
