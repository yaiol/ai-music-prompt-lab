// twin: uis-fld-value.jsx jsx@e7a9591bcc31 rsx@b2b0e5abf768
//! UisFldValue — twin of `uis-fld-value.jsx`: a read-only row that shows a NAMED value with actions beside
//! it, in a bordered rounded shell. It owns the shell (surface, border, label, value, action slot); the
//! actions are the caller's and the value is the children. Same classes and element order, so the
//! `.uis-fld-value*` rules in `uis-app.css` skin it.
//!
//! `label` is the name; `accent` a CSS colour that recolours it for this one row (set as the row's
//! `--uis-accent`); `mono` puts the value in the monospace font; `wrap` is a long value, which drops under the
//! label and wraps instead of being cut with an ellipsis. `actions` holds the buttons.
//! Differs: none.

use dioxus::prelude::*;

#[component]
pub fn UisFldValue(
    label: String,
    #[props(default)] accent: String,
    #[props(default)] wrap: bool,
    #[props(default)] mono: bool,
    #[props(default)] actions: Option<Element>,
    children: Element,
) -> Element {
    let class = if wrap { "uis-fld-value wrap" } else { "uis-fld-value" };
    let text_class = if mono { "uis-fld-value-text mono" } else { "uis-fld-value-text" };
    let style = if accent.is_empty() { String::new() } else { format!("--uis-accent: {accent}") };

    rsx! {
        div { class: "{class}", style: "{style}",
            span { class: "uis-fld-value-label", "{label}" }
            span { class: "{text_class}", {children} }
            if let Some(actions) = actions {
                span { class: "uis-fld-value-actions", {actions} }
            }
        }
    }
}
