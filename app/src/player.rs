// ⚠ NO HARDCODED UI STRINGS IN THIS FILE.
//   Every string a user can read MUST go through the t() lookup, and the key MUST be a string literal at the call site.

//! The player bar at the foot of the window (the Electron app's `GlobalPlayer`): shuffle, previous, play / pause, stop, next,
//! repeat, the title, the position, the time, mute and volume, close. The sound comes from the page's own audio element, fed by
//! the local file server; the bar reads it back every quarter of a second and sends it its orders. Shuffle, repeat and volume
//! are remembered (`shuffle`, `repeat`, `volume`).

use crate::ui::{Ui, title_of};
use dioxus::prelude::*;
use dioxus_free_icons::{
    Icon,
    icons::ld_icons::{LdChevronLeft, LdChevronRight, LdPause, LdPlay, LdRepeat, LdRepeat2, LdShuffle, LdSquare, LdVolume1, LdVolume2, LdVolumeX, LdX},
};
use engine::store::Card;
use std::path::Path;

/// The audio element the bar drives.
const AUDIO: &str = "ap-audio";

/// Play, or pause, the file that is on the bar.
pub fn toggle() {
    let _ = document::eval(&format!("const a=document.getElementById('{AUDIO}'); if(a) {{ a.paused ? a.play().catch(()=>{{}}) : a.pause(); }}"));
}

impl Ui {
    /// The play button of a card: the card on the bar is paused or resumed; any other card takes the bar, and the cards
    /// shown that have a file become the queue (next, previous, shuffle and repeat walk it).
    pub fn play_card(self, card: &Card) {
        if card.media_path.is_empty() {
            return;
        }
        if *self.playing.peek() == card.id {
            toggle();
            return;
        }
        let queue: Vec<String> = self.shown_cards().into_iter().filter(|c| !c.media_path.is_empty()).map(|c| c.id).collect();
        let mut playlist = self.playlist;
        playlist.set(queue);
        let mut playing = self.playing;
        playing.set(card.id.clone());
    }
}

fn clock(seconds: f64) -> String {
    if !seconds.is_finite() || seconds <= 0.0 {
        return "0:00".to_owned();
    }
    format!("{}:{:02}", seconds as u64 / 60, seconds as u64 % 60)
}

/// A number below `n`, different from `not` when there is a choice.
fn random_below(n: usize, not: usize) -> usize {
    let mut x = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos() as usize).unwrap_or(0);
    loop {
        let i = x % n;
        if i != not || n < 2 {
            return i;
        }
        x = x / 7 + 3;
    }
}

#[component]
pub fn PlayerBar() -> Element {
    let ui = use_context::<Ui>();
    let id = (ui.playing)();
    match ui.card(&id).filter(|c| !c.media_path.is_empty()) {
        Some(card) => rsx! { Bar { card } },
        None => rsx! {},
    }
}

#[component]
fn Bar(card: Card) -> Element {
    let ui = use_context::<Ui>();
    let mut sounding = ui.sounding;
    let (mut playing, playlist) = (ui.playing, ui.playlist);
    let mut time = use_signal(|| 0.0f64);
    let mut length = use_signal(|| 0.0f64);
    let mut volume = use_signal(|| ui.st().setting("volume").parse::<f64>().unwrap_or(1.0).clamp(0.0, 1.0));
    let mut muted = use_signal(|| false);
    let mut shuffle = use_signal(|| ui.st().setting("shuffle") == "1");
    let mut repeat = use_signal(|| match ui.st().setting("repeat").as_str() {
        "all" => "all",
        "one" => "one",
        _ => "off",
    });

    // read the audio element back; it also gets the volume, since a new song is a new element
    use_future(move || async move {
        let mut feed = document::eval(&format!(
            "clearInterval(window.__apPoll); window.__apPoll = setInterval(() => {{ const a = document.getElementById('{AUDIO}'); if (!a) return; if (a.volume !== window.__apVol && window.__apVol !== undefined) a.volume = window.__apVol; dioxus.send([a.currentTime, isFinite(a.duration) ? a.duration : 0, a.paused ? 0 : 1]); }}, 250);"
        ));
        while let Ok(v) = feed.recv::<Vec<f64>>().await {
            if v.len() == 3 {
                time.set(v[0]);
                length.set(v[1]);
                sounding.set(v[2] > 0.5);
            }
        }
    });
    use_drop(move || {
        let _ = document::eval("clearInterval(window.__apPoll)");
        sounding.set(false);
    });
    use_effect(move || {
        let v = if muted() { 0.0 } else { volume() };
        let _ = document::eval(&format!("window.__apVol = {v}; const a = document.getElementById('{AUDIO}'); if (a) a.volume = {v};"));
    });

    let list = playlist.read().clone();
    let index = list.iter().position(|x| *x == card.id);
    let len = list.len();
    let has_prev = index.is_some_and(|i| i > 0);
    let has_next = index.is_some_and(|i| i + 1 < len);
    let jump = {
        let list = list.clone();
        move |i: usize| {
            if let Some(next) = list.get(i) {
                playing.set(next.clone());
            }
        }
    };
    let (on_prev, on_next, on_end) = {
        let (mut jump_a, mut jump_b, mut jump_c) = (jump.clone(), jump.clone(), jump.clone());
        let at = index.unwrap_or(0);
        (
            move |_| {
                if shuffle() && len > 1 {
                    jump_a(random_below(len, at));
                } else if has_prev {
                    jump_a(at - 1);
                }
            },
            move |_| {
                if shuffle() && len > 1 {
                    jump_b(random_below(len, at));
                } else if has_next {
                    jump_b(at + 1);
                }
            },
            move |_| {
                if repeat() == "one" {
                    let _ = document::eval(&format!("const a=document.getElementById('{AUDIO}'); if(a) {{ a.currentTime=0; a.play().catch(()=>{{}}); }}"));
                } else if shuffle() && len > 1 {
                    jump_c(random_below(len, at));
                } else if has_next {
                    jump_c(at + 1);
                } else if repeat() == "all" && len > 0 {
                    jump_c(0);
                }
            },
        )
    };

    let url = crate::server().audio_url(Path::new(&card.media_path));
    let missing = !Path::new(&card.media_path).exists();
    let title = title_of(&card);
    let vol = if muted() { 0.0 } else { volume() };
    let s_shuffle = if shuffle() { ui.t("tipFtrShuffleOn") } else { ui.t("tipFtrShuffleOff") };
    let s_repeat = match repeat() {
        "all" => ui.t("tipFtrRepeatAll"),
        "one" => ui.t("tipFtrRepeatOne"),
        _ => ui.t("tipFtrRepeatOff"),
    };
    let s_mute = if muted() { ui.t("tipFtrUnmute") } else { ui.t("tipFtrMute") };
    let s_vol = format!("{}: {}%", ui.t("tipFtrVolume"), (vol * 100.0).round());
    let s_prev = ui.t("tipFtrPrevious");
    let s_next = ui.t("tipFtrNext");
    let s_stop = ui.t("tipFtrStop");
    let s_play = ui.t("tipCardPlay");
    let s_pause = ui.t("tipCardPause");
    let s_missing = ui.t("tipCardMediaMissing");
    let is_on = sounding();
    rsx! {
        div { class: "uis-barh-footer",
            audio { key: "{url}", id: AUDIO, autoplay: true, src: "{url}", onended: on_end }
            button {
                class: if shuffle() { "uis-btn ico small subtle active ap-ply" } else { "uis-btn ico small subtle ap-ply" },
                title: "{s_shuffle}",
                onclick: move |_| {
                    let now = !shuffle();
                    shuffle.set(now);
                    ui.st().set_setting("shuffle", if now { "1" } else { "0" });
                },
                Icon { width: 14, height: 14, icon: LdShuffle }
                if shuffle() {
                    span { class: "ap-ply-dot" }
                }
            }
            button { class: "uis-btn ico small subtle", title: "{s_prev}", onclick: on_prev, Icon { width: 14, height: 14, icon: LdChevronLeft } }
            button {
                class: "uis-btn ico primary",
                title: if is_on { "{s_pause}" } else { "{s_play}" },
                onclick: move |_| toggle(),
                if is_on {
                    Icon { width: 14, height: 14, icon: LdPause }
                } else {
                    Icon { width: 14, height: 14, icon: LdPlay }
                }
            }
            button {
                class: "uis-btn ico small subtle ap-ply-stop",
                title: "{s_stop}",
                onclick: move |_| {
                    let _ = document::eval(&format!("const a=document.getElementById('{AUDIO}'); if(a) {{ a.pause(); a.currentTime=0; }}"));
                },
                Icon { width: 14, height: 14, icon: LdSquare }
            }
            button { class: "uis-btn ico small subtle", title: "{s_next}", onclick: on_next, Icon { width: 14, height: 14, icon: LdChevronRight } }
            button {
                class: if repeat() != "off" { "uis-btn ico small subtle active ap-ply" } else { "uis-btn ico small subtle ap-ply" },
                title: "{s_repeat}",
                onclick: move |_| {
                    let next = match repeat() {
                        "off" => "all",
                        "all" => "one",
                        _ => "off",
                    };
                    repeat.set(next);
                    ui.st().set_setting("repeat", next);
                },
                if repeat() == "one" {
                    Icon { width: 14, height: 14, icon: LdRepeat2 }
                    span { class: "ap-ply-one", "1" }
                } else {
                    Icon { width: 14, height: 14, icon: LdRepeat }
                    if repeat() == "all" {
                        span { class: "ap-ply-dot" }
                    }
                }
            }
            span { class: "ap-player-title", title: "{title}", "{title}" }
            if missing {
                span { class: "ap-hint", "{s_missing}" }
            }
            input {
                r#type: "range",
                class: "uis-sld ap-grow",
                min: "0",
                max: "{length().max(1.0)}",
                step: "0.1",
                value: "{time()}",
                oninput: move |e| {
                    if let Ok(v) = e.value().parse::<f64>() {
                        time.set(v);
                        let _ = document::eval(&format!("const a=document.getElementById('{AUDIO}'); if(a) a.currentTime={v};"));
                    }
                },
            }
            span { class: "uis-hnt ap-ply-time", "{clock(time())} / {clock(length())}" }
            div { class: "uis-barh-sep" }
            button { class: "uis-btn ico small subtle", title: "{s_mute}", onclick: move |_| muted.toggle(),
                if vol == 0.0 {
                    Icon { width: 14, height: 14, icon: LdVolumeX }
                } else if vol < 0.5 {
                    Icon { width: 14, height: 14, icon: LdVolume1 }
                } else {
                    Icon { width: 14, height: 14, icon: LdVolume2 }
                }
            }
            input {
                r#type: "range",
                class: "uis-sld ap-ply-vol",
                min: "0",
                max: "1",
                step: "0.02",
                value: "{vol}",
                title: "{s_vol}",
                oninput: move |e| {
                    if let Ok(v) = e.value().parse::<f64>() {
                        volume.set(v);
                        muted.set(v == 0.0);
                        ui.st().set_setting("volume", &v.to_string());
                    }
                },
            }
            button { class: "uis-btn ico small subtle", onclick: move |_| playing.set(String::new()), Icon { width: 14, height: 14, icon: LdX } }
        }
    }
}
