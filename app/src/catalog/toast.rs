// twin: uis-fx-toast.jsx jsx@8dfbeeafaa74 rsx@fbe521e35ce2
//! LbsToast — twin of `uis-fx-toast.jsx` (`LbsHokToast`): the transient "it happened" message. A centered pill
//! that floats over the bottom of the window, self-dismisses, and never takes a click. The look is the
//! shared `.uis-ovl-toast` rule in `uis-app.css`; the behaviour (the message, the self-dismiss timer, replacing an
//! on-screen toast instead of stacking) is here.
//!
//! Usage - render `toast.view()` once, at the end of the app root:
//!
//!   let toast = use_toast(None);
//!   toast.show(t("tstAppSaved"));          // default dwell
//!   toast.show_for(message, 6000);         // longer, for something the user must read
//!   ... rsx! { div { class: "uis-app-root", /* ... */ {toast.view()} } }
//!
//! An app whose bottom chrome (a player bar, a statusbar) would cover the pill lifts it with `bottom`
//! (px) - a per-instance OFFSET, never a re-shape. The timer is the page's `setTimeout`, awaited from Rust.
//! Differs: behaviour none. Form only: a `Copy` handle (`show` / `show_for` / `view`) because Rust has no hook
//! returning a tuple; `bottom` is given to `use_toast`, as in the JSX.

use dioxus::prelude::*;

const DEFAULT_MS: u32 = 3000;

/// The handle `use_toast` returns. `Copy`, so it can be captured by any number of handlers.
#[derive(Clone, Copy)]
pub struct LbsToast {
    msg: Signal<Option<String>>,
    /// Bumped by every toast, so a dismiss scheduled by an earlier one knows it has been replaced.
    epoch: Signal<u32>,
    bottom: Option<i32>,
}

impl LbsToast {
    /// Show `text` for the default dwell, replacing any toast already on screen.
    pub fn show(&self, text: impl Into<String>) {
        self.show_for(text, DEFAULT_MS);
    }

    /// Show `text` for `ms` milliseconds. The default is a floor, not a ceiling: a short confirmation
    /// reads in 3 s, but a message the user must act on (an error, a name, a count) deserves longer.
    pub fn show_for(&self, text: impl Into<String>, ms: u32) {
        let (mut msg, mut epoch) = (self.msg, self.epoch);
        epoch += 1;
        let mine = epoch();
        msg.set(Some(text.into()));
        spawn(async move {
            let _ = document::eval(&format!("await new Promise((r) => setTimeout(r, {ms})); dioxus.send(1);"))
                .recv::<i32>()
                .await;
            if epoch() == mine {
                msg.set(None);
            }
        });
    }

    /// The pill, or nothing. Render it once, at the end of the app root.
    pub fn view(&self) -> Element {
        match (self.msg)() {
            Some(text) => {
                let style = self.bottom.map(|b| format!("--uis-ovl-toast-bottom: {b}px"));
                rsx! {
                    div { class: "uis-ovl-toast", style, "{text}" }
                }
            }
            None => rsx! {},
        }
    }
}

pub fn use_toast(bottom: Option<i32>) -> LbsToast {
    let msg = use_signal(|| Option::<String>::None);
    let epoch = use_signal(|| 0u32);
    LbsToast { msg, epoch, bottom }
}
