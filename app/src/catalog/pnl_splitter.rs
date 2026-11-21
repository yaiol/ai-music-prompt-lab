// twin: uis-pnl-splitter.jsx jsx@20e6020e1377 rsx@a42cf55c8413
//! UisPnlSplitter — twin of `uis-pnl-splitter.jsx`: the resizable-panel divider. A 4px transparent grab strip,
//! `col-resize` cursor, accent tint at 40% on hover and full accent while dragging. Only the LOOK is
//! shared: the drag wiring stays the caller's (`on_resize_start`).
//!
//! The brand accent is the SAME in every app (only the theme differs), so both values live here and the
//! app just passes its current theme (`"light"` | `"dark"`, default dark). Like the JSX, the control
//! carries its own three CSS rules and puts them in the page once.
//! Differs: none.

use dioxus::prelude::*;

/// Canonical brand accent - identical across all apps, theme-keyed. Keep in lockstep with the palette
/// (light = #5a4fff, dark = #7c6fff).
fn accent(theme: &str) -> &'static str {
    if theme == "light" { "#5a4fff" } else { "#7c6fff" }
}

/// Puts the splitter's rules in the page once, whatever number of splitters are mounted.
const INJECT: &str = r#"
if (!document.getElementById('uis-pnl-splitter-style')) {
  const el = document.createElement('style');
  el.id = 'uis-pnl-splitter-style';
  el.textContent =
    '.uis-pnl-splitter{width:4px;flex-shrink:0;cursor:col-resize;background:transparent;position:relative;z-index:10;align-self:stretch}' +
    '.uis-pnl-splitter:hover{background:color-mix(in srgb,var(--uis-pnl-splitter-accent) 40%,transparent)}' +
    '.uis-pnl-splitter:active{background:var(--uis-pnl-splitter-accent)}';
  document.head.appendChild(el);
}
"#;

#[component]
pub fn UisPnlSplitter(
    /// `"light"` | `"dark"`; anything else is dark.
    #[props(default)] theme: String,
    on_resize_start: EventHandler<MouseEvent>,
    /// Extra inline style (a flex basis, a margin ...); the accent variable is added after it.
    #[props(default)] style: String,
    /// The tooltip, already translated. Empty = none.
    #[props(default)] title: String,
) -> Element {
    use_hook(|| {
        let _ = document::eval(INJECT);
    });
    let accent = accent(&theme);
    let style = if style.is_empty() {
        format!("--uis-pnl-splitter-accent: {accent}")
    } else {
        format!("{style}; --uis-pnl-splitter-accent: {accent}")
    };

    rsx! {
        div {
            class: "uis-pnl-splitter",
            title: if title.is_empty() { None } else { Some(title.clone()) },
            onmousedown: move |e| on_resize_start.call(e),
            style: "{style}",
        }
    }
}
