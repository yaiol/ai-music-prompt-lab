//! Two Lucide glyphs the `dioxus-free-icons` lucide set does not ship yet (`calendar-arrow-up` / `calendar-arrow-down`), written
//! the same way the crate writes its own: same view box, stroke width 2, round caps. The paths are Lucide's, unchanged, so the
//! sort-by-date button draws the same picture as in the Electron app. Delete this file when the crate has them.

use dioxus::prelude::*;
use dioxus_free_icons::IconShape;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct LdCalendarArrowUp;
impl IconShape for LdCalendarArrowUp {
    fn view_box(&self) -> &str {
        "0 0 24 24"
    }
    fn xmlns(&self) -> &str {
        "http://www.w3.org/2000/svg"
    }
    fn fill_and_stroke<'a>(&self, user_color: &'a str) -> (&'a str, &'a str, &'a str) {
        ("none", user_color, "2")
    }
    fn stroke_linecap(&self) -> &str {
        "round"
    }
    fn stroke_linejoin(&self) -> &str {
        "round"
    }
    fn child_elements(&self) -> Element {
        rsx! {
            path { d: "m14 17 4-4 4 4" }
            path { d: "M16 2v3" }
            path { d: "M18 21v-8" }
            path { d: "M21 10.343V5a2 2 0 00-2-2H5a2 2 0 00-2 2v14a2 2 0 002 2h9" }
            path { d: "M3 9h18" }
            path { d: "M8 2v3" }
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct LdCalendarArrowDown;
impl IconShape for LdCalendarArrowDown {
    fn view_box(&self) -> &str {
        "0 0 24 24"
    }
    fn xmlns(&self) -> &str {
        "http://www.w3.org/2000/svg"
    }
    fn fill_and_stroke<'a>(&self, user_color: &'a str) -> (&'a str, &'a str, &'a str) {
        ("none", user_color, "2")
    }
    fn stroke_linecap(&self) -> &str {
        "round"
    }
    fn stroke_linejoin(&self) -> &str {
        "round"
    }
    fn child_elements(&self) -> Element {
        rsx! {
            path { d: "m14 17 4 4 4-4" }
            path { d: "M16 2v3" }
            path { d: "M18 13v8" }
            path { d: "M21 10.354V5a2 2 0 00-2-2H5a2 2 0 00-2 2v14a2 2 0 002 2h7.343" }
            path { d: "M3 9h18" }
            path { d: "M8 2v3" }
        }
    }
}
