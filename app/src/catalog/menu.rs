// twin: uis-ctl-menu.jsx jsx@544be514068f rsx@920135146d66
//! UisMenu — twin of `uis-ctl-menu.jsx`: the dropdown that drops from a toolbar button. A themed panel of
//! rows (`.uis-pop-item`) with optional section labels, dividers, and one level of hover-opened submenus.
//! Same classes and element order, so `uis-app.css` skins it unchanged. Built on `UisPopover`, so the
//! trigger is named by its element id (`anchor_id`).
//!
//! Controlled: `anchor_id` + `open` + `on_close`. A `UisMenuItem` runs its action then closes the menu; one
//! with a `submenu` becomes a hover-flyout parent. `UisMenuLabel` is a section sub-label, `UisMenuDivider` a
//! separator line. The menu must render OUTSIDE any `.uis-barh-grp` weld (it would inherit the weld's
//! `height: 100%`): put the trigger in the weld and the `UisMenu` beside it.
//! Differs: behaviour none. Form only: the trigger is named by id (`anchor_id`) because rsx has no element refs; the
//! close delay is the page's timer awaited from Rust.

use super::popover::{UisPopAlign, UisPopPlacement, UisPopover};
use dioxus::prelude::*;
use std::sync::atomic::{AtomicU32, Ordering};

static NEXT_ID: AtomicU32 = AtomicU32::new(1);

/// How long the pointer may be away from a parent row before its flyout closes: just long enough to
/// cross into the flyout, which cancels it.
const CLOSE_DELAY_MS: u32 = 180;

/// What a menu's rows share. The MENU owns which submenu is open - a single id, not a flag per row - so
/// "only one submenu, ever" is structural: sweeping down a column of parent rows cannot leave several
/// flyouts on screen at once.
#[derive(Clone, Copy)]
struct MenuCtx {
    close: Callback<()>,
    open_sub: Signal<Option<String>>,
}

#[component]
pub fn UisMenu(
    anchor_id: String,
    open: bool,
    on_close: EventHandler<()>,
    #[props(default)] title: String,
    #[props(default)] align: UisPopAlign,
    children: Element,
) -> Element {
    let close = use_callback(move |_: ()| on_close.call(()));
    let mut open_sub = use_signal(|| Option::<String>::None);
    use_context_provider(|| MenuCtx { close, open_sub });

    // A reopened menu must never restore the submenu that was open when it closed.
    use_effect(use_reactive!(|(open,)| {
        if !open {
            open_sub.set(None);
        }
    }));

    rsx! {
        UisPopover {
            anchor_id,
            open,
            on_close: move |_| close.call(()),
            align,
            ignore_selector: ".menu-flyout",
            if !title.is_empty() {
                div { class: "uis-men-title", "{title}" }
            }
            {children}
        }
    }
}

#[component]
pub fn UisMenuLabel(children: Element) -> Element {
    rsx! {
        div { class: "uis-men-label", {children} }
    }
}

#[component]
pub fn UisMenuDivider() -> Element {
    rsx! {
        div { class: "uis-men-divider" }
    }
}

/// The rows of a flyout share the menu's close action but get a slot of their own that nothing else
/// reads: a leaf's `mouseenter` must not close the very flyout it lives in, and this also caps nesting
/// at one level.
#[component]
fn FlyoutScope(children: Element) -> Element {
    let parent = use_context::<MenuCtx>();
    let own = use_signal(|| Option::<String>::None);
    use_context_provider(|| MenuCtx { close: parent.close, open_sub: own });
    rsx! {
        {children}
    }
}

#[component]
pub fn UisMenuItem(
    label: String,
    #[props(default)] on_click: Option<EventHandler<()>>,
    /// A parent row: no action of its own, opens this as a flyout while hovered.
    #[props(default)] submenu: Option<Element>,
    #[props(default = false)] disabled: bool,
    #[props(default)] icon: Option<Element>,
) -> Element {
    let ctx = use_context::<MenuCtx>();
    let mut open_sub = ctx.open_sub;
    let id = use_hook(|| format!("menu-row-{}", NEXT_ID.fetch_add(1, Ordering::Relaxed)));
    // Bumped whenever the pointer comes back, so a close scheduled earlier knows it has been cancelled.
    let mut epoch = use_signal(|| 0u32);

    let hover = open_sub.read().as_deref() == Some(id.as_str());

    // Entering a row claims the submenu slot AT ONCE, so any sibling flyout disappears the moment you
    // reach the next row - no overlap, whatever the delay below is.
    let open_now = use_callback({
        let id = id.clone();
        move |_: ()| {
            epoch += 1;
            open_sub.set(Some(id.clone()));
        }
    });
    // Hover intent: leaving the row is not leaving the menu, so closing waits just long enough for the
    // pointer to cross into the flyout, which cancels it. It closes only if this row STILL owns the slot
    // - by then the pointer may already have opened a sibling, which a plain `set(None)` would close.
    let close_soon = use_callback({
        let id = id.clone();
        move |_: ()| {
            epoch += 1;
            let mine = epoch();
            let id = id.clone();
            spawn(async move {
                let _ = document::eval(&format!(
                    "await new Promise((r) => setTimeout(r, {CLOSE_DELAY_MS})); dioxus.send(1);"
                ))
                .recv::<i32>()
                .await;
                if epoch() == mine && open_sub.peek().as_deref() == Some(id.as_str()) {
                    open_sub.set(None);
                }
            });
        }
    });

    // A leaf row: close the menu first, then run the action (which may be async). Hovering it also
    // dismisses any open flyout - moving onto a plain row is a clear "not going to the submenu".
    let Some(submenu) = submenu else {
        return rsx! {
            button {
                r#type: "button",
                class: "uis-pop-item",
                role: "menuitem",
                disabled,
                onmouseenter: move |_| open_sub.set(None),
                onclick: move |_| {
                    ctx.close.call(());
                    if let Some(h) = on_click {
                        h.call(());
                    }
                },
                if let Some(icon) = icon {
                    {icon}
                }
                span { "{label}" }
            }
        };
    };

    rsx! {
        div { class: "uis-men-sub", onmouseenter: move |_| open_now(()), onmouseleave: move |_| close_soon(()),
            div {
                id: "{id}",
                class: "uis-pop-item uis-men-item-parent",
                role: "menuitem",
                "aria-haspopup": "true",
                span { "{label}" }
                span { class: "uis-men-caret", "aria-hidden": "true", "▸" }
            }
            UisPopover {
                anchor_id: id.clone(),
                open: hover,
                on_close: move |_| open_sub.set(None),
                placement: UisPopPlacement::Side,
                class: "menu-flyout",
                on_mouse_enter: move |_| open_now(()),
                on_mouse_leave: move |_| close_soon(()),
                FlyoutScope {
                    div { role: "menu", {submenu} }
                }
            }
        }
    }
}
