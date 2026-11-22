// twin: uis-pkr-date.jsx jsx@e9e492047a5d rsx@967f49971f52
//! UisPkrDate — twin of `uis-pkr-date.jsx`: the themed calendar date field. A trigger showing the
//! formatted date + a calendar glyph; clicking opens a popup: a month header (chevron nav + a month/year
//! jump), a weekday row, the day grid, and a Today / Clear footer. Same classes and element order, so
//! the paired `uis-pkr-date.css` (carried by this control, like the JSX imports it) skins it.
//!
//! i18n WITHOUT app coupling: the month and weekday names, the display format and the first day of the
//! week come from the WebView's own `Intl` (the `locale` prop, e.g. "fr" / "pt-BR") - the same locale
//! data that localizes the native picker, so every language is covered with zero i18n keys. That is the
//! one question the page is asked; the calendar ARITHMETIC (which day is which cell) is Rust. The only
//! words passed in are `today_label` / `clear_label`.
//!
//! Controlled: `value` is a "YYYY-MM-DD" string ("" = empty) and `on_change(str)` ("" when cleared).
//! Differs: behaviour none. The month / weekday names and the display text come from the page's `Intl` (the same data
//! the JSX reads), asked once per change, so the field's text can arrive one beat after the first paint.

use super::popover::use_dismiss;
use dioxus::prelude::*;
use dioxus_free_icons::{
    Icon,
    icons::ld_icons::{LdCalendarDays, LdChevronDown, LdChevronUp},
};
use serde::Deserialize;
use std::sync::atomic::{AtomicU32, Ordering};

const CSS: &str = include_str!("../../assets/uis-pkr-date.css");
static NEXT_ID: AtomicU32 = AtomicU32::new(1);

const POP_H: f64 = 340.0;

/// What the WebView's `Intl` says about a locale, a displayed month and the selected date.
#[derive(Clone, PartialEq, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct Cal {
    month_label: String,
    display: String,
    weekdays: Vec<String>,
    months: Vec<String>,
    /// 0 = Sunday ... 6 = Saturday
    first_day: u32,
    /// (year, month 0-11, day) in the user's local time
    today: (i32, u32, u32),
}

// ── Calendar arithmetic (proleptic Gregorian; days counted from 1970-01-01) ────────────────────────────

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// 0 = Sunday ... 6 = Saturday (1970-01-01 was a Thursday).
fn weekday(days: i64) -> u32 {
    (days + 4).rem_euclid(7) as u32
}

/// "YYYY-MM-DD" -> (year, month 0-11, day).
fn parse_ymd(s: &str) -> Option<(i32, u32, u32)> {
    let mut it = s.split('-');
    let (y, m, d) = (it.next()?, it.next()?, it.next()?);
    if it.next().is_some() || y.len() != 4 || m.len() != 2 || d.len() != 2 {
        return None;
    }
    Some((y.parse().ok()?, m.parse::<u32>().ok()?.checked_sub(1)?, d.parse().ok()?))
}

fn to_ymd(y: i32, m: u32, d: u32) -> String {
    format!("{y:04}-{:02}-{d:02}", m + 1)
}

const CAL_JS: &str = r#"
const p = JSON.parse(PARAMS);
const L = p.loc;
const f = (o, d) => new Intl.DateTimeFormat(L, o).format(d);
let firstDay = 1;   // 1 = Mon ... 7 = Sun
try { const wi = new Intl.Locale(L).weekInfo; if (wi && wi.firstDay) firstDay = wi.firstDay; } catch (_) {}
const fd = firstDay % 7;   // 0 = Sun ... 6 = Sat
const weekdays = [];
for (let i = 0; i < 7; i++) weekdays.push(f({ weekday: 'short' }, new Date(2023, 0, 1 + ((fd + i) % 7))));   // 2023-01-01 is a Sunday
const months = [];
for (let m = 0; m < 12; m++) months.push(f({ month: 'short' }, new Date(2023, m, 1)));
const t = new Date();
dioxus.send({
  monthLabel: f({ month: 'long', year: 'numeric' }, new Date(p.y, p.m, 1)),
  display: p.sel ? new Intl.DateTimeFormat(L).format(new Date(p.sel[0], p.sel[1], p.sel[2])) : '',
  weekdays, months, firstDay: fd,
  today: [t.getFullYear(), t.getMonth(), t.getDate()],
});
"#;

#[component]
pub fn UisPkrDate(
    value: String,
    on_change: EventHandler<String>,
    #[props(default = "en".to_string())] locale: String,
    #[props(default)] today_label: String,
    #[props(default)] clear_label: String,
    #[props(default)] placeholder: String,
    #[props(default)] class: String,
    /// ⚠ It must set the SAME properties on every render: Dioxus keeps one the string no longer sets.
    #[props(default)] style: String,
) -> Element {
    // app langKeys use "_" (pt_BR); Intl wants "-"
    let loc = locale.replace('_', "-");
    let id = use_hook(|| NEXT_ID.fetch_add(1, Ordering::Relaxed));
    let btn_id = format!("dp-btn-{id}");
    let pop_id = format!("dp-pop-{id}");

    let mut open = use_signal(|| false);
    let mut jump = use_signal(|| false);
    let mut pos = use_signal(|| Option::<(f64, f64)>::None);
    let sel = parse_ymd(&value);
    let mut view = use_signal(|| sel.map(|(y, m, _)| (y, m)).unwrap_or((1970, 0)));

    let close = use_callback(move |_: ()| open.set(false));
    use_dismiss(open(), &btn_id, &pop_id, close);

    // Everything the page's Intl is asked, in ONE call, whenever the locale, the shown month or the
    // selection changes.
    let (vy, vm) = view();
    let cal_loc = loc.clone();
    let cal_value = value.clone();
    let cal = use_resource(use_reactive!(|(cal_loc, cal_value)| async move {
        let (vy, vm) = view();
        let sel = parse_ymd(&cal_value);
        let params = serde_json::json!({ "loc": cal_loc, "y": vy, "m": vm, "sel": sel.map(|(y, m, d)| [y, m as i32, d as i32]) });
        let js = format!("const PARAMS = {}; {CAL_JS}", serde_json::to_string(&params.to_string()).unwrap_or_default());
        document::eval(&js).recv::<Cal>().await.ok()
    }));
    let _ = (vy, vm);
    let cal_now: Option<Cal> = cal.read().clone().flatten();

    let toggle = {
        let btn_id = btn_id.clone();
        let cal_now = cal_now.clone();
        move |_| {
            if open() {
                open.set(false);
                return;
            }
            // Opening starts on the selected month, else on today's.
            let start = sel.map(|(y, m, _)| (y, m)).or(cal_now.as_ref().map(|c| (c.today.0, c.today.1)));
            if let Some(v) = start {
                view.set(v);
            }
            jump.set(false);
            let btn_id = btn_id.clone();
            spawn(async move {
                let mut ev = document::eval(&format!(
                    "const el = document.getElementById('{btn_id}'); \
                     if (el) {{ const r = el.getBoundingClientRect(); dioxus.send([r.top, r.bottom, r.left, window.innerHeight]); }} \
                     else {{ dioxus.send([0, 0, 0, 0]); }}"
                ));
                if let Ok([top, bottom, left, win_h]) = ev.recv::<[f64; 4]>().await {
                    let go_up = bottom + POP_H > win_h && top > POP_H;
                    pos.set(Some(if go_up { (top - POP_H - 6.0, left) } else { (bottom + 6.0, left) }));
                    open.set(true);
                }
            });
        }
    };

    let mut pick = move |y: i32, m: u32, d: u32| {
        on_change.call(to_ymd(y, m, d));
        open.set(false);
    };
    let mut shift_month = move |delta: i32| {
        let (y, m) = view();
        let total = y * 12 + m as i32 + delta;
        view.set((total.div_euclid(12), total.rem_euclid(12) as u32));
    };

    let field_class = {
        let mut c = String::from("uis-dat-field");
        if sel.is_none() {
            c.push_str(" dp-empty");
        }
        if open() {
            c.push_str(" open");
        }
        if !class.is_empty() {
            c.push(' ');
            c.push_str(&class);
        }
        c
    };
    let shown = match (&cal_now, sel) {
        (Some(c), Some(_)) => c.display.clone(),
        (_, Some((y, m, d))) => to_ymd(y, m, d),
        _ => placeholder.clone(),
    };

    rsx! {
        document::Style { "{CSS}" }
        button {
            id: "{btn_id}",
            r#type: "button",
            class: "{field_class}",
            style: "{style}",
            onclick: toggle,
            span { "{shown}" }
            span { class: "uis-btn ico small subtle",
                Icon { width: 16, height: 16, icon: LdCalendarDays }
            }
        }
        if let (true, Some((top, left)), Some(cal)) = (open(), pos(), cal_now.clone()) {
            div { id: "{pop_id}", class: "uis-dat-pop", style: "top: {top}px; left: {left}px",
                div { class: "uis-dat-head",
                    button { r#type: "button", class: "uis-dat-title", onclick: move |_| jump.set(!jump()),
                        "{cal.month_label}"
                        Icon { width: 14, height: 14, icon: LdChevronDown }
                    }
                    div { class: "uis-barh-grp",
                        button { r#type: "button", class: "uis-btn ico", onclick: move |_| shift_month(-1),
                            Icon { width: 16, height: 16, icon: LdChevronUp }
                        }
                        button { r#type: "button", class: "uis-btn ico", onclick: move |_| shift_month(1),
                            Icon { width: 16, height: 16, icon: LdChevronDown }
                        }
                    }
                }
                if jump() {
                    div { class: "uis-dat-yr",
                        button { r#type: "button", class: "uis-btn ico", onclick: move |_| view.set((view().0 - 1, view().1)),
                            Icon { width: 16, height: 16, icon: LdChevronUp }
                        }
                        span { "{view().0}" }
                        button { r#type: "button", class: "uis-btn ico", onclick: move |_| view.set((view().0 + 1, view().1)),
                            Icon { width: 16, height: 16, icon: LdChevronDown }
                        }
                    }
                    div { class: "uis-dat-months",
                        for (m, name) in cal.months.iter().enumerate() {
                            button {
                                key: "{m}",
                                r#type: "button",
                                class: if m as u32 == view().1 { "uis-dat-month on" } else { "uis-dat-month" },
                                onclick: move |_| {
                                    view.set((view().0, m as u32));
                                    jump.set(false);
                                },
                                "{name}"
                            }
                        }
                    }
                } else {
                    div { class: "uis-dat-grid",
                        for (i, w) in cal.weekdays.iter().enumerate() {
                            div { key: "w{i}", class: "uis-dat-wd", "{w}" }
                        }
                        // 6 weeks; the arithmetic spills into the adjacent months by itself.
                        {
                            let first = days_from_civil(view().0 as i64, view().1 as i64 + 1, 1);
                            let leading = (weekday(first) as i64 - cal.first_day as i64 + 7) % 7;
                            rsx! {
                                for i in 0..42i64 {
                                    {
                                        let (y, m, d) = civil_from_days(first + i - leading);
                                        let (y, m, d) = (y as i32, (m - 1) as u32, d as u32);
                                        let mut class = String::from("uis-dat-day");
                                        if m != view().1 {
                                            class.push_str(" other");
                                        }
                                        if (y, m, d) == cal.today {
                                            class.push_str(" today");
                                        }
                                        if sel == Some((y, m, d)) {
                                            class.push_str(" on");
                                        }
                                        rsx! {
                                            button {
                                                key: "{i}",
                                                r#type: "button",
                                                class: "{class}",
                                                onclick: move |_| pick(y, m, d),
                                                "{d}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                div { class: "uis-dat-foot",
                    button {
                        r#type: "button",
                        class: "uis-btn subtle",
                        onclick: move |_| {
                            on_change.call(String::new());
                            open.set(false);
                        },
                        "{clear_label}"
                    }
                    button {
                        r#type: "button",
                        class: "uis-btn",
                        onclick: move |_| pick(cal.today.0, cal.today.1, cal.today.2),
                        "{today_label}"
                    }
                }
            }
        }
    }
}
