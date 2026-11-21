// ⚠ NO HARDCODED UI STRINGS IN THIS FILE.
//   Every string a user can read MUST go through the t() lookup, and the key MUST be a string literal at the call site.

//! The card dialog, laid out as the Electron app lays it out: the type icon, the title and the version in the header; the name; the
//! tabs of the card's type; the tags under the tabs; and a foot with the date, the counts, the wizard, Cancel and Save.

use crate::{
    ai,
    card_view::kind_icon,
    catalog::{UisBtnCopy, UisCboStandard, UisComboItem, UisFldValue, UisNum, UisPkrDate},
    dialogs::Field,
    scrape,
    ui::{Nav, Ui},
    wizard,
};
use dioxus::prelude::*;
use dioxus_free_icons::{
    Icon,
    icons::ld_icons::{LdAlignLeft, LdCheck, LdChevronDown, LdCopy, LdExternalLink, LdFileText, LdFolderOpen, LdGlobe, LdLanguages, LdLink2, LdMusic, LdMusic2, LdPlay, LdScrollText, LdSparkles, LdStar, LdTrash2, LdWand, LdX},
};
use engine::{
    cards::{CardPatch, NewCard},
    langs, songpage,
    store::{Card, new_id},
};

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Desc,
    Style,
    Linked,
    Lyrics,
    Translation,
    Links,
    Note,
}

/// One translation as the card dialog holds it: new (`pending`, not in the database yet) or old, possibly `deleted`.
#[derive(Clone, PartialEq)]
struct TrRow {
    id: String,
    lang: String,
    name: String,
    content: String,
    pending: bool,
    deleted: bool,
}

/// The rows of each text area, and the least height of a tab's pane (a new card is shorter: it has no date line).
const ROWS_DESC: u32 = 13;
const ROWS_STYLE: u32 = 7;
const ROWS_LYRICS: u32 = 11;
const ROWS_TRANSLATION: u32 = 9;
const ROWS_EDIT_BONUS: u32 = 2;

#[component]
pub fn CardDlg(id: Option<String>, kind: String, draft: Option<Card>) -> Element {
    let ui = use_context::<Ui>();
    let existing: Option<Card> = id.as_ref().and_then(|i| ui.card(i));
    let start = existing.clone().or(draft.clone()).unwrap_or_else(|| Card { kind: kind.clone(), ..Default::default() });
    let is_new = existing.is_none();
    let bonus = if is_new { 0 } else { ROWS_EDIT_BONUS };
    let mut kind_s = use_signal({
        let k = start.kind.clone();
        move || k.clone()
    });
    let (mut name, mut sort, mut style, mut ref_artist, mut ref_comp, mut lyrics, mut desc, mut note, mut tags_text, mut media, mut lang) = (
        use_signal({ let v = start.name.clone(); move || v.clone() }),
        use_signal(move || start.sort_number),
        use_signal({ let v = start.style.clone(); move || v.clone() }),
        use_signal({ let v = start.reference_artist.clone(); move || v.clone() }),
        use_signal({ let v = start.reference_complement.clone(); move || v.clone() }),
        use_signal({ let v = start.lyrics.clone(); move || v.clone() }),
        use_signal({ let v = start.desc.clone(); move || v.clone() }),
        use_signal({ let v = start.note.clone(); move || v.clone() }),
        use_signal({ let v = start.tags.join(", "); move || v.clone() }),
        use_signal({ let v = start.media_path.clone(); move || v.clone() }),
        use_signal({ let v = start.lang.clone(); move || v.clone() }),
    );
    // what was typed in the AI dialog last time, kept on the card
    let mut ai_tweaks = use_signal({ let v = start.ai_tweaks.clone(); move || v.clone() });
    let mut linked = use_signal({ let v = start.linked_cards.clone(); move || v.clone() });
    let mut link_query = use_signal(String::new);
    let mut urls = use_signal({ let v: Vec<(String, String)> = start.urls.iter().map(|u| (u.label.clone(), u.href.clone())).collect(); move || v.clone() });
    let (mut url_label, mut url_href) = (use_signal(String::new), use_signal(String::new));
    let mut tab = use_signal(|| Tab::Desc);
    let mut fetching = use_signal(|| false);
    // The translations as the dialog holds them: nothing reaches the database until the card is saved, and Cancel drops it all.
    let mut trs = use_signal({
        let rows: Vec<TrRow> = id.as_ref().map(|i| ui.st().translations(i).unwrap_or_default()).unwrap_or_default().into_iter().map(|t| TrRow { id: t.id, lang: t.lang, name: t.name, content: t.content, pending: false, deleted: false }).collect();
        move || rows.clone()
    });
    let mut tr_open = use_signal(std::collections::HashSet::<String>::new);
    let mut tr_query = use_signal(String::new);
    let mut tr_busy = use_signal(|| None::<String>);
    let mut src_query = use_signal(String::new);
    let mut src_open = use_signal(|| false);
    let mut name_err = use_signal(|| false);
    // the card's day, `YYYY-MM-DD`; only an existing card shows it
    let date_start = engine::dates::ymd(start.date);
    let mut date_val = use_signal({
        let v = date_start.clone();
        move || v.clone()
    });
    let mut wizard_open = use_signal(|| false);
    // the names the AI proposed with its text: one click puts one in the name
    let mut suggestions = use_signal(Vec::<String>::new);
    // which AI dialog is open: "" none, else the kind it is for ("songstyle" = the style of a song, "song" = its lyrics)
    let mut ai_for = use_signal(String::new);

    let k = kind_s();
    let is_song = k == "song";
    let is_note = k == "note";
    let accent = ui.kind_colour(&k);
    let style_limit: usize = ui.st().setting("ai-warn-limit").parse().unwrap_or(1000);
    let lyrics_limit: usize = ui.st().setting("ai-lyric-threshold").parse().unwrap_or(5000);
    let t = |key: &str| ui.t(key);
    let title = if is_new { t("ttlDlgCardNew") } else { t("ttlDlgCardEdit") };
    let kinds = [("note", t("lblStgColorNote")), ("music", t("lblAppTypeMusic")), ("vocal", t("lblAppTypeVocal")), ("song", t("lblAppTypeSong"))];
    let (plh_name, plh_style, plh_lyrics, plh_tags, plh_ref_a, plh_ref_c) = (
        match k.as_str() { "song" => t("plhDlgCardNameSong"), "vocal" => t("plhDlgCardNameVocal"), _ => t("plhDlgCardNameMusic") },
        t("plhDlgCardStyle"),
        t("plhDlgCardLyrics"),
        match k.as_str() { "song" => t("plhDlgCardTagsSong"), "vocal" => t("plhDlgCardTagsVocal"), _ => t("plhDlgCardTagsMusic") },
        t("plhDlgCardReferenceArtist"),
        t("plhDlgCardReferenceComplement"),
    );
    let save_label = if is_new { t("btnDlgCardCreate") } else { t("btnDlgCardSaveChanges") };
    let tab_defs: Vec<(Tab, String)> = if is_note {
        vec![(Tab::Desc, t("tabDlgCardDesc")), (Tab::Note, t("tabDlgCardNote"))]
    } else if is_song {
        vec![
            (Tab::Desc, t("tabDlgCardDesc")),
            (Tab::Style, t("lblDlgCardStyle")),
            (Tab::Linked, t("lblDlgCardLinkedStyles")),
            (Tab::Lyrics, t("tabDlgCardLyrics")),
            (Tab::Translation, t("tabDlgCardTranslation")),
            (Tab::Links, t("tabDlgCardLinks")),
            (Tab::Note, t("tabDlgCardNote")),
        ]
    } else {
        vec![(Tab::Desc, t("tabDlgCardDesc")), (Tab::Style, t("lblDlgCardStyle")), (Tab::Links, t("lblDlgCardExternalLinks")), (Tab::Note, t("tabDlgCardNote"))]
    };
    let active_tab = if tab_defs.iter().any(|(x, _)| *x == tab()) { tab() } else { tab_defs[0].0 };

    let translations: Vec<TrRow> = trs().into_iter().filter(|r| !r.deleted).collect();
    let used_langs: Vec<String> = translations.iter().map(|x| x.lang.clone()).collect();
    let ui_lang = ui.lang.peek().clone();
    let matches_lang = |l: &langs::SongLanguage, q: &str| q.is_empty() || l.name.to_lowercase().contains(q) || langs::lang_name(&l.code, &ui_lang).to_lowercase().contains(q) || l.code.to_lowercase().contains(q);
    let add_q = tr_query().trim().to_lowercase();
    let free_langs: Vec<&langs::SongLanguage> = langs::song_languages().iter().filter(|l| !used_langs.contains(&l.code) && matches_lang(l, &add_q)).collect();
    let free_items: Vec<UisComboItem> = free_langs.iter().map(|l| UisComboItem { key: l.code.clone(), label: format!("{} ({})", langs::lang_name(&l.code, &ui_lang), l.code), active: false, header: false }).collect();
    let free_codes: Vec<String> = free_langs.iter().map(|l| l.code.clone()).collect();
    // the source language of the lyrics: the field shows the language when closed, what is being typed when open
    let src_q = src_query().trim().to_lowercase();
    let src_langs: Vec<&langs::SongLanguage> = langs::song_languages().iter().filter(|l| matches_lang(l, &src_q)).collect();
    let src_items: Vec<UisComboItem> = src_langs.iter().map(|l| UisComboItem { key: l.code.clone(), label: format!("{} ({})", langs::lang_name(&l.code, &ui_lang), l.code), active: l.code == lang(), header: false }).collect();
    let src_codes: Vec<String> = src_langs.iter().map(|l| l.code.clone()).collect();
    let src_shown = if src_open() {
        src_query()
    } else if lang().is_empty() {
        String::new()
    } else {
        format!("{} ({})", langs::lang_name(&lang(), &ui_lang), lang())
    };
    let (l_today, l_clear, l_ai_tr, tip_copy_req) = (ui.t("btnGlbToday"), ui.t("btnGlbClear"), ui.t("btnDlgCardTranslationAi"), ui.t("tipDlgAiCopyRequest"));
    let l_chars = t("lblDlgCardChars");
    let style_total = style().chars().count() + linked().iter().filter_map(|i| ui.card(i)).map(|c| c.style.chars().count()).sum::<usize>();
    let lyrics_total = lyrics().chars().count();
    let version = existing.as_ref().map(|c| c.version.max(1)).unwrap_or(1);
    let (global_env, tip_global, btn_global) = (ui.global_env(), ui.t("tipPnlProjectGlobal"), ui.t("btnPnlProjectMakeGlobal"));

    // the tags: what is being typed after the last comma, matched against the tags the cards already have
    let typed = tags_text();
    let word = typed.rsplit(',').next().unwrap_or("").trim().to_lowercase();
    let confirmed = if typed.contains(',') { format!("{} ", &typed[..=typed.rfind(',').unwrap_or(0)]) } else { String::new() };
    let entered: Vec<String> = typed.split(',').map(|x| x.trim().to_lowercase()).filter(|x| !x.is_empty()).collect();
    let tag_hits: Vec<String> = if word.is_empty() { Vec::new() } else { ui.all_tags().into_iter().filter(|tg| tg.to_lowercase().contains(&word) && !entered.contains(&tg.to_lowercase())).collect() };
    let tag_items: Vec<UisComboItem> = tag_hits.iter().map(|tg| UisComboItem { key: tg.clone(), label: tg.clone(), active: false, header: false }).collect();
    let render_tag = use_callback({
        let (hits, word) = (tag_hits.clone(), word.clone());
        move |i: usize| {
            let Some(tg) = hits.get(i) else { return rsx! {} };
            let colour = ui.tag_colour(tg);
            let lower = tg.to_lowercase();
            let (before, hit, after) = match (lower.len() == tg.len()).then(|| lower.find(&word)).flatten() {
                Some(at) => (tg.get(..at).unwrap_or(""), tg.get(at..at + word.len()).unwrap_or(""), tg.get(at + word.len()..).unwrap_or("")),
                None => (tg.as_str(), "", ""),
            };
            rsx! {
                span { class: "uis-chp", style: "--uis-chp:{colour}",
                    "{before}"
                    strong { "{hit}" }
                    "{after}"
                }
            }
        }
    });

    // the music and vocal cards a song can link to: the open project's (or all of them) and what is linked already, then,
    // for a search, the Global ones
    let project_ids: Option<Vec<String>> = match &*ui.nav.read() {
        Nav::Project(p) => ui.projects.read().iter().find(|x| &x.id == p).map(|x| x.card_ids.clone()),
        _ => None,
    };
    let linkable: Vec<Card> = ui
        .cards
        .read()
        .iter()
        .filter(|c| (c.kind == "music" || c.kind == "vocal") && Some(&c.id) != id.as_ref() && project_ids.as_ref().is_none_or(|ids| ids.contains(&c.id) || linked.read().contains(&c.id)))
        .cloned()
        .collect();
    let link_q = link_query().trim().to_lowercase();
    let link_groups: Vec<(&'static str, Vec<(Card, bool)>)> = ["music", "vocal"]
        .into_iter()
        .map(|key| {
            let mut group: Vec<(Card, bool)> = linkable.iter().filter(|c| c.kind == key && (linked.read().contains(&c.id) || (!link_q.is_empty() && c.name.to_lowercase().contains(&link_q)))).map(|c| (c.clone(), false)).collect();
            if !link_q.is_empty() {
                group.extend(ui.cards.read().iter().filter(|c| c.kind == key && c.env == global_env && !linkable.iter().any(|l| l.id == c.id) && c.name.to_lowercase().contains(&link_q)).map(|c| (c.clone(), true)));
            }
            group.sort_by(|(a, _), (b, _)| b.favorite.cmp(&a.favorite).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())).then(a.version.cmp(&b.version)));
            (key, group)
        })
        .filter(|(_, g)| !g.is_empty())
        .collect();
    let search_placeholder = if linked().is_empty() { ui.t("plhHdrSearchCards") } else { ui.t("plhHdrSearchSeeAll") };

    // the links, the ones of the AI sites first, then the music sites, then the rest
    let (ai_sites, music_sites) = (ui.ai_sites(), ui.music_sites());
    let mut sorted_urls: Vec<(usize, String, String)> = urls().into_iter().enumerate().map(|(i, (l, h))| (i, l, h)).collect();
    sorted_urls.sort_by(|a, b| {
        let rank = |l: &str| {
            let l = l.trim().to_lowercase();
            if ai_sites.contains(&l) { 0 } else if music_sites.contains(&l) { 1 } else { 2 }
        };
        rank(&a.1).cmp(&rank(&b.1)).then_with(|| a.1.to_lowercase().cmp(&b.1.to_lowercase()))
    });

    let src_header: Option<Callback<Callback<()>, Element>> = if lang().is_empty() {
        None
    } else {
        Some(Callback::new(move |close: Callback<()>| {
            rsx! {
                button {
                    r#type: "button",
                    class: "uis-pop-item uis-pop-dim",
                    onmousedown: move |e| {
                        e.prevent_default();
                        lang.set(String::new());
                        close.call(());
                    },
                    "{ui.t(\"btnGlbClear\")}"
                }
            }
        }))
    };

    let id_save = id.clone();
    let save = move |_| {
        let tags: Vec<String> = tags_text().split(',').map(|t| t.trim().to_lowercase()).filter(|t| !t.is_empty()).collect();
        let nm = name().trim().to_owned();
        if nm.is_empty() {
            name_err.set(true);
            return;
        }
        let want_urls = urls();
        // a day the user did not touch is left as it was
        let date_patch = if date_val() != date_start { engine::dates::from_ymd(&date_val()) } else { None };
        let card_id = match &id_save {
            Some(i) => {
                let ok = ui.save_card(
                    i,
                    &CardPatch {
                        kind: Some(kind_s()),
                        name: Some(nm),
                        desc: Some(desc().trim().to_owned()),
                        note: Some(note().trim().to_owned()),
                        style: Some(style().trim().to_owned()),
                        reference_artist: Some(ref_artist().trim().to_owned()),
                        reference_complement: Some(ref_comp().trim().to_owned()),
                        lyrics: Some(lyrics().trim().to_owned()),
                        tags: Some(tags),
                        linked_cards: Some(linked()),
                        sort_number: sort(),
                        media_path: Some(media()),
                        lang: Some(lang()),
                        date: date_patch,
                        ..Default::default()
                    },
                );
                if !ok {
                    return;
                }
                i.clone()
            }
            None => {
                let new = NewCard {
                    id: new_id(),
                    kind: kind_s(),
                    name: nm,
                    desc: desc().trim().to_owned(),
                    note: note().trim().to_owned(),
                    style: style().trim().to_owned(),
                    reference_artist: ref_artist().trim().to_owned(),
                    reference_complement: ref_comp().trim().to_owned(),
                    lyrics: lyrics().trim().to_owned(),
                    tags,
                    linked_cards: linked(),
                    sort_number: sort(),
                    media_path: media(),
                    // a draft keeps the date its song has
                    date: draft.as_ref().map(|d| d.date).filter(|d| *d != 0),
                    ..Default::default()
                };
                match ui.create_card(new) {
                    Some(c) => {
                        let tweaks = ai_tweaks();
                        let has_tweaks = tweaks.as_object().is_some_and(|o| !o.is_empty());
                        if !lang().is_empty() || has_tweaks {
                            let _ = ui.st().update_card(&c.id, &CardPatch { lang: Some(lang()).filter(|l| !l.is_empty()), ai_tweaks: Some(tweaks).filter(|_| has_tweaks), ..Default::default() });
                        }
                        c.id
                    }
                    None => return,
                }
            }
        };
        let st = ui.st();
        // translations: what was deleted goes, what is new comes, what was edited is written
        for r in trs() {
            if r.pending {
                if !r.deleted {
                    let _ = st.create_translation(&card_id, &r.lang, &r.name, &r.content);
                }
            } else if r.deleted {
                let _ = st.delete_translation(&r.id);
            } else {
                let _ = st.update_translation(&r.id, Some(&r.name), Some(&r.content));
            }
        }
        // links: what was removed goes, what is new comes
        if let Ok(Some(card)) = st.card(&card_id) {
            for u in card.urls.iter().filter(|u| !want_urls.iter().any(|(l, h)| *l == u.label && *h == u.href)) {
                let _ = st.delete_card_url(&card_id, &u.id);
            }
            for (l, h) in want_urls.iter().filter(|(l, h)| !card.urls.iter().any(|u| u.label == *l && u.href == *h)) {
                let _ = st.add_card_url(&card_id, l, h);
            }
        }
        ui.reload();
        ui.close();
    };

    // Read the lyrics off a song page, and for a song with no linked card its style too
    let mut fetch_lyrics = move |target: String| {
        fetching.set(true);
        spawn(async move {
            let got = scrape::lyrics_of(&target).await;
            fetching.set(false);
            match got {
                Ok((l, s)) => {
                    let mut any = false;
                    if !l.is_empty() {
                        lyrics.set(l);
                        any = true;
                    }
                    if kind_s.peek().as_str() == "song" && linked.peek().is_empty() && !s.is_empty() {
                        style.set(s);
                        any = true;
                    }
                    if any {
                        ui.ok(ui.t("tstDlgCardLyricsFetched"));
                    } else {
                        ui.ok(ui.t("tstDlgCardLyricsNone"));
                    }
                }
                Err(e) => ui.fail(ui.t("tstDlgCardLyricsFailed").replace("{error}", &e)),
            }
        });
    };
    let fetchable = urls().into_iter().find(|(_, h)| scrape::supported(h)).map(|(_, h)| h);

    // A typed link: an address without a scheme gets one, a Suno share link becomes the song's own address,
    // a missing label is made from the site; a page that can give lyrics is read when the card has none
    let add_link = move || {
        let typed = url_href.peek().trim().to_owned();
        if typed.is_empty() {
            return;
        }
        let given = url_label.peek().trim().to_owned();
        spawn(async move {
            let mut href = if typed.starts_with("http") { typed } else { format!("https://{typed}") };
            if songpage::is_suno_share(&href) {
                let share = href.clone();
                if let Ok(Ok(song)) = tokio::task::spawn_blocking(move || songpage::resolve_suno_share(&share)).await {
                    if !song.is_empty() {
                        href = song;
                    }
                }
            }
            let label = if given.is_empty() { engine::transfer::song_url_label(&href) } else { given };
            urls.write().push((label, href.clone()));
            url_label.set(String::new());
            url_href.set(String::new());
            let is_song = kind_s.peek().as_str() == "song";
            if scrape::supported(&href) && (lyrics.peek().trim().is_empty() || (is_song && linked.peek().is_empty() && style.peek().trim().is_empty())) {
                fetch_lyrics(href);
            }
        });
    };

    // the AI button of a field: it names the kind of text the dialog writes
    let ai_style_kind = if is_song { "songstyle".to_owned() } else { k.clone() };
    let ai_lyrics_kind = "song".to_owned();

    let opt_label = format!("({})", ui.t("msgDlgCardOptional"));
    let min_h = if is_new { 290 } else { 330 };
    let note_area = if is_note { "min-height: 200px;" } else { "" };
    let over_style = |n: usize| if n > style_limit { "font-variant-numeric: tabular-nums; color: var(--uis-danger); font-weight: 700;" } else { "font-variant-numeric: tabular-nums;" };
    let over_lyrics = |n: usize| if n > lyrics_limit { "font-variant-numeric: tabular-nums; color: var(--uis-danger); font-weight: 700;" } else { "font-variant-numeric: tabular-nums;" };
    let opt_span = "color: var(--uis-text-mute); text-transform: none; letter-spacing: 0; font-weight: 400;";
    let src_input_style = if lang().is_empty() { "width: 110px; max-width: 150px;" } else { "width: auto; max-width: 150px;" };

    rsx! {
        div { class: "uis-dl-backdrop", onmousedown: move |_| ui.close(),
            div { class: "uis-dlg", style: "max-width: 900px;", onmousedown: move |e| e.stop_propagation(),
                div { style: "padding: 28px 28px 0; flex-shrink: 0;",
                    div { style: "display: flex; justify-content: space-between; align-items: center;",
                        h2 { class: "uis-dlg-title", style: "margin: 0;",
                            span { style: "color: {accent}; display: flex;", {kind_icon(&k, 18)} }
                            "{title}"
                            span { style: "font-size: 12px; font-weight: 700; color: {accent}; border: 1px solid {accent}55; border-radius: 6px; padding: 2px 8px;", "v{version}" }
                        }
                        button {
                            class: "uis-dl-close",
                            onmounted: move |e| async move {
                                let _ = e.set_focus(true).await;
                            },
                            onclick: move |_| ui.close(),
                            Icon { width: 18, height: 18, icon: LdX }
                        }
                    }
                }
                div { style: "flex: 1; min-height: 0; overflow-y: auto; padding: 20px 28px; display: flex; flex-direction: column; gap: 20px;",
                    if is_new {
                        Field { label: ui.t("lblDlgCardType"), accent: accent.clone(),
                            div { class: "uis-opt-set",
                                for (key , label) in kinds {
                                    button {
                                        key: "{key}",
                                        class: if k == key { "uis-opt active" } else { "uis-opt" },
                                        style: "--uis-accent: {ui.kind_colour(key)};",
                                        onclick: move |_| {
                                            kind_s.set(key.to_owned());
                                            suggestions.set(Vec::new());
                                            tab.set(Tab::Desc);
                                        },
                                        {kind_icon(key, 16)}
                                        span { "{label}" }
                                    }
                                }
                            }
                        }
                    }
                    Field { label: ui.t("lblDlgCardName"), accent: accent.clone(),
                        if !suggestions().is_empty() {
                            div { style: "display: flex; flex-wrap: wrap; gap: 6px; margin-bottom: 8px;",
                                for (i , sug) in suggestions().into_iter().enumerate() {
                                    button {
                                        key: "{i}",
                                        class: "uis-btn primary",
                                        onclick: {
                                            let sug = sug.clone();
                                            move |_| {
                                                name.set(sug.clone());
                                                suggestions.set(Vec::new());
                                            }
                                        },
                                        Icon { width: 14, height: 14, icon: LdSparkles }
                                        "{sug}"
                                    }
                                }
                                button { class: "uis-btn ico small subtle", onclick: move |_| suggestions.set(Vec::new()), Icon { width: 14, height: 14, icon: LdX } }
                            }
                        }
                        div { style: "display: flex; gap: 8px; align-items: flex-start;",
                            UisNum {
                                value: sort().map(|n| n as f64),
                                width: "64px",
                                allow_empty: true,
                                min: Some(1.0),
                                max: Some(999.0),
                                decimals: Some(0),
                                placeholder: "#",
                                style: "text-align: center; font-weight: 700;",
                                on_change: move |v: Option<f64>| sort.set(v.map(|n| n.round() as i64)),
                            }
                            input {
                                class: "uis-txt",
                                style: if name_err() { "border-color: var(--uis-danger); flex: 1; margin: 0;" } else { "flex: 1; margin: 0;" },
                                placeholder: "{plh_name}",
                                value: "{name}",
                                oninput: move |e| {
                                    name.set(e.value());
                                    name_err.set(false);
                                },
                            }
                        }
                        if name_err() {
                            span { style: "font-size: 12px; color: var(--uis-danger);", "{ui.t(\"msgDlgCardRequired\")}" }
                        }
                    }
                    div { class: "uis-tab-set", style: "margin: 0 -28px; padding: 0 28px;",
                        for (x , label) in tab_defs {
                            button {
                                key: "{label}-{active_tab == x}",
                                class: if active_tab == x { "uis-tab active" } else { "uis-tab" },
                                style: if active_tab == x { "--uis-accent: {accent};" } else { "" },
                                onclick: move |_| tab.set(x),
                                match x {
                                    Tab::Desc => rsx! { Icon { width: 14, height: 14, icon: LdAlignLeft } },
                                    Tab::Style => rsx! { Icon { width: 14, height: 14, icon: LdMusic2 } },
                                    Tab::Linked => rsx! { Icon { width: 14, height: 14, icon: LdLink2 } },
                                    Tab::Lyrics => rsx! { Icon { width: 14, height: 14, icon: LdScrollText } },
                                    Tab::Translation => rsx! { Icon { width: 14, height: 14, icon: LdLanguages } },
                                    Tab::Links => rsx! { Icon { width: 14, height: 14, icon: LdGlobe } },
                                    Tab::Note => rsx! { Icon { width: 14, height: 14, icon: LdFileText } },
                                }
                                "{label}"
                            }
                        }
                    }
                    div { style: "min-height: {min_h}px; display: flex; flex-direction: column; gap: 20px;",
                        match active_tab {
                            Tab::Desc => rsx! {
                                Field { label: ui.t("tabDlgCardDesc"), accent: accent.clone(),
                                    textarea { class: "uis-txt-area", style: "{note_area}", rows: "{ROWS_DESC + bonus}", placeholder: "{ui.t(\"tabDlgCardDesc\")}…", value: "{desc}", oninput: move |e| desc.set(e.value()) }
                                }
                            },
                            Tab::Note => rsx! {
                                Field { label: ui.t("tabDlgCardNote"), accent: accent.clone(),
                                    textarea { class: "uis-txt-area", style: "{note_area}", rows: "{ROWS_DESC + bonus}", placeholder: "{ui.t(\"tabDlgCardNote\")}…", value: "{note}", oninput: move |e| note.set(e.value()) }
                                }
                            },
                            Tab::Style => rsx! {
                                div { style: "display: flex; gap: 6px;",
                                    div { class: "uis-fld-labelled", style: "flex: 1;",
                                        label { class: "uis-fld-labelled-label", style: "color: {accent};", "{ui.t(\"lblDlgCardReferenceArtist\")}" }
                                        input { class: "uis-txt", placeholder: "{plh_ref_a}", value: "{ref_artist}", oninput: move |e| ref_artist.set(e.value()) }
                                    }
                                    div { class: "uis-fld-labelled", style: "flex: 2;",
                                        label { class: "uis-fld-labelled-label", style: "color: {accent};", "{ui.t(\"lblDlgCardReferenceComplement\")}" }
                                        input { class: "uis-txt", placeholder: "{plh_ref_c}", value: "{ref_comp}", oninput: move |e| ref_comp.set(e.value()) }
                                    }
                                }
                                Field { label: ui.t("lblDlgCardStyle"), accent: accent.clone(),
                                    textarea { class: "uis-txt-area", rows: "{ROWS_STYLE + bonus}", placeholder: "{plh_style}", value: "{style}", oninput: move |e| style.set(e.value()) }
                                    div { style: "display: flex; align-items: center; justify-content: space-between; margin-top: 4px;",
                                        button {
                                            class: "uis-btn",
                                            onclick: {
                                                let kind_for_ai = ai_style_kind.clone();
                                                move |_| ai_for.set(kind_for_ai.clone())
                                            },
                                            Icon { width: 14, height: 14, icon: LdSparkles }
                                            "{ui.t(\"tipDlgAiGenerate\")}"
                                        }
                                        if is_song {
                                            span { class: "uis-hnt", style: "font-variant-numeric: tabular-nums;", "{style().chars().count()} {l_chars}" }
                                        } else {
                                            span { class: "uis-hnt", style: "{over_style(style().chars().count())}", "{style().chars().count()} {l_chars}" }
                                        }
                                    }
                                    // a music or vocal card of a saved card can be made global: the one set of cards every environment sees
                                    if let (false, Some(card)) = (is_song, existing.as_ref()) {
                                        div { style: "margin-top: 12px; display: flex; align-items: center; gap: 8px;",
                                            if card.env == global_env {
                                                span { style: "font-size: 12px; color: var(--uis-accent); display: flex; align-items: center; gap: 5px;",
                                                    Icon { width: 14, height: 14, class: "uis-ico-inline", icon: LdGlobe }
                                                    "{tip_global}"
                                                }
                                            } else {
                                                button {
                                                    class: "uis-btn",
                                                    onclick: {
                                                        let card_id = card.id.clone();
                                                        let global_env = global_env.clone();
                                                        move |_| {
                                                            if ui.st().update_card(&card_id, &CardPatch { env: Some(global_env.clone()), ..Default::default() }).is_ok() {
                                                                ui.reload();
                                                                ui.ok(ui.t("tstPnlProjectMadeGlobal"));
                                                            }
                                                        }
                                                    },
                                                    Icon { width: 14, height: 14, class: "uis-ico-inline", icon: LdGlobe }
                                                    "{btn_global}"
                                                }
                                            }
                                        }
                                    }
                                }
                            },
                            Tab::Linked => rsx! {
                                div { class: "uis-fld-labelled",
                                    div { style: "display: flex; align-items: center; margin-bottom: 0;",
                                        label { class: "uis-fld-labelled-label", style: "color: {accent}; margin-bottom: 0; display: flex; align-items: center; gap: 5px;",
                                            Icon { width: 14, height: 14, class: "uis-ico-inline", icon: LdLink2 }
                                            "{ui.t(\"lblDlgCardLinkedStyles\")}"
                                            span { style: "{opt_span}", "{opt_label}" }
                                        }
                                    }
                                    if linkable.is_empty() {
                                        p { class: "uis-hnt", "{ui.t(\"empDlgCardNoMusicVocal\")}" }
                                    } else {
                                        div { style: "display: flex; flex-direction: column; gap: 8px;",
                                            input { class: "uis-txt", placeholder: "{search_placeholder}", value: "{link_query}", oninput: move |e| link_query.set(e.value()) }
                                            div { style: "display: flex; flex-direction: column; gap: 8px; max-height: 260px; overflow-y: auto;",
                                                for (key , group) in link_groups {
                                                    {
                                                        let colour = ui.kind_colour(key);
                                                        let heading = if key == "music" { ui.t("lblAppTypeMusic") } else { ui.t("lblAppTypeVocal") };
                                                        rsx! {
                                                            div { key: "{key}",
                                                                div { style: "font-size: 10px; color: {colour}; letter-spacing: 1px; text-transform: uppercase; margin-bottom: 4px; display: flex; align-items: center; gap: 4px;",
                                                                    span { style: "display: flex;", {kind_icon(key, 12)} }
                                                                    "{heading}"
                                                                }
                                                                div { style: "display: flex; flex-wrap: wrap; gap: 6px;",
                                                                    for (c , is_global) in group {
                                                                        {
                                                                            let on = linked.read().contains(&c.id);
                                                                            let cid = c.id.clone();
                                                                            rsx! {
                                                                                button {
                                                                                    key: "{c.id}",
                                                                                    class: if on { "uis-btn active" } else { "uis-btn" },
                                                                                    style: "--uis-accent: {colour};",
                                                                                    title: if is_global { ui.t("lblCardGlobalResource") } else { String::new() },
                                                                                    onclick: move |_| {
                                                                                        let mut set = linked.write();
                                                                                        if let Some(i) = set.iter().position(|x| *x == cid) {
                                                                                            set.remove(i);
                                                                                        } else {
                                                                                            set.push(cid.clone());
                                                                                        }
                                                                                    },
                                                                                    if on {
                                                                                        Icon { width: 14, height: 14, class: "uis-ico-inline", icon: LdCheck }
                                                                                    }
                                                                                    if c.favorite && !on {
                                                                                        Icon { width: 14, height: 14, class: "uis-ico-inline", style: "opacity: 0.5;", icon: LdStar }
                                                                                    }
                                                                                    if is_global {
                                                                                        span { style: "font-size: 9px; opacity: 0.6;", "🌍" }
                                                                                    }
                                                                                    "{c.name}"
                                                                                    span { style: "font-size: 10px; opacity: 0.7; font-weight: 700;", "v{c.version.max(1)}" }
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            },
                            Tab::Lyrics => rsx! {
                                Field { label: ui.t("tabDlgCardLyrics"), accent: accent.clone(),
                                    textarea { class: "uis-txt-area", rows: "{ROWS_LYRICS + bonus}", placeholder: "{plh_lyrics}", value: "{lyrics}", oninput: move |e| lyrics.set(e.value()) }
                                    div { style: "display: flex; align-items: center; margin-top: 4px; gap: 8px;",
                                        button {
                                            class: "uis-btn",
                                            onclick: {
                                                let kind_for_ai = ai_lyrics_kind.clone();
                                                move |_| ai_for.set(kind_for_ai.clone())
                                            },
                                            Icon { width: 14, height: 14, icon: LdSparkles }
                                            "{ui.t(\"tipDlgAiGenerate\")}"
                                        }
                                        if let Some(target) = fetchable.clone() {
                                            button { class: "uis-btn", title: "{ui.t(\"tipDlgCardFetchLyrics\")}", disabled: fetching(), onclick: move |_| fetch_lyrics(target.clone()),
                                                Icon { width: 14, height: 14, icon: LdGlobe }
                                                if fetching() { "{ui.t(\"lblDlgCardFetching\")}" } else { "{ui.t(\"lblDlgCardFetchLyrics\")}" }
                                            }
                                        }
                                        if !lyrics().trim().is_empty() {
                                            button {
                                                class: "uis-btn",
                                                title: "{ui.t(\"tipDlgCardCopyLyricsClean\")}",
                                                onclick: move |_| {
                                                    let _ = arboard::Clipboard::new().and_then(|mut b| b.set_text(engine::exports::clean_lyrics(&lyrics())));
                                                    ui.ok(ui.t("btnDlgCardCopyLyricsClean"));
                                                },
                                                Icon { width: 14, height: 14, icon: LdMusic }
                                                "{ui.t(\"btnDlgCardCopyLyricsClean\")}"
                                            }
                                        }
                                        div { style: "flex: 1;" }
                                        span { class: "uis-hnt", style: "flex-shrink: 0;", "{ui.t(\"lblDlgCardTranslationSourceLanguage\")}" }
                                        div { style: "flex-shrink: 0;",
                                            UisCboStandard {
                                                input_style: "{src_input_style}",
                                                value: src_shown,
                                                placeholder: "…",
                                                items: src_items,
                                                on_change: move |v: String| src_query.set(v),
                                                on_focus: move |_| {
                                                    let l = lang();
                                                    src_query.set(if l.is_empty() { String::new() } else { langs::lang_name(&l, &ui.lang.peek()) });
                                                },
                                                on_open_change: move |o: bool| src_open.set(o),
                                                on_pick: move |i: usize| {
                                                    if let Some(code) = src_codes.get(i) {
                                                        lang.set(code.clone());
                                                    }
                                                    src_query.set(String::new());
                                                },
                                                render_header: src_header,
                                            }
                                        }
                                        span { class: "uis-hnt", style: "font-variant-numeric: tabular-nums; flex-shrink: 0;", "{lyrics_total} {l_chars}" }
                                    }
                                }
                            },
                            Tab::Translation => rsx! {
                                div { class: "uis-fld-labelled",
                                    if translations.is_empty() {
                                        p { class: "uis-hnt", style: "text-align: center; padding: 16px 0;", "{ui.t(\"empDlgCardTranslationNone\")}" }
                                    }
                                    for tr in translations {
                                        {
                                            let open = tr_open.read().contains(&tr.id);
                                            let label = langs::lang_name(&tr.lang, &ui_lang);
                                            let busy = tr_busy() == Some(tr.id.clone());
                                            let (id_fold, id_name, id_text, id_ai, id_del) = (tr.id.clone(), tr.id.clone(), tr.id.clone(), tr.id.clone(), tr.id.clone());
                                            let (lang_ai, lang_req) = (tr.lang.clone(), tr.lang.clone());
                                            let content_copy = tr.content.clone();
                                            let chevron = if open { format!("transform: rotate(180deg); transition: transform 0.15s; color: {accent}; flex-shrink: 0;") } else { "transform: none; transition: transform 0.15s; color: var(--uis-text-mute); flex-shrink: 0;".to_owned() };
                                            rsx! {
                                                div { key: "{tr.id}", style: "border: 1px solid var(--uis-border-strong); border-radius: 8px; margin-bottom: 8px; overflow: hidden;",
                                                    div {
                                                        style: "display: flex; align-items: center; gap: 6px; padding: 7px 10px; background: var(--uis-bg-input); cursor: pointer;",
                                                        onclick: move |_| {
                                                            let mut set = tr_open.write();
                                                            if !set.remove(&id_fold) {
                                                                set.insert(id_fold.clone());
                                                            }
                                                        },
                                                        Icon { width: 14, height: 14, class: "uis-ico-inline", style: "{chevron}", icon: LdChevronDown }
                                                        span { style: "font-size: 13px; font-weight: 600; color: var(--uis-text); flex-shrink: 0;",
                                                            "{label} "
                                                            span { style: "color: var(--uis-text-mute); font-weight: 400;", "({tr.lang})" }
                                                        }
                                                        input {
                                                            class: "uis-txt",
                                                            style: "flex: 1;",
                                                            value: "{tr.name}",
                                                            onclick: move |e| e.stop_propagation(),
                                                            oninput: move |e| {
                                                                let v = e.value();
                                                                trs.write().iter_mut().filter(|r| r.id == id_name).for_each(|r| r.name = v.clone());
                                                            },
                                                        }
                                                        button {
                                                            class: "uis-btn small",
                                                            title: "{l_ai_tr}",
                                                            disabled: busy,
                                                            onclick: move |e| {
                                                                e.stop_propagation();
                                                                if ui.st().setting("gemini-key").is_empty() {
                                                                    return ui.fail(ui.t("msgDlgCardTranslationNoKey"));
                                                                }
                                                                if lyrics().trim().is_empty() {
                                                                    return ui.fail(ui.t("msgDlgCardTranslationNoLyrics"));
                                                                }
                                                                let (row, lang_code) = (id_ai.clone(), lang_ai.clone());
                                                                tr_busy.set(Some(row.clone()));
                                                                spawn(async move {
                                                                    let got = crate::translate::translate_song(ui, name().trim(), &lyrics(), &lang(), &lang_code).await;
                                                                    tr_busy.set(None);
                                                                    match got {
                                                                        Ok(t) => {
                                                                            trs.write().iter_mut().filter(|r| r.id == row).for_each(|r| {
                                                                                r.content = t.content.clone();
                                                                                if !t.name.is_empty() {
                                                                                    r.name = t.name.clone();
                                                                                }
                                                                            });
                                                                            if lang().is_empty() && crate::translate::known_code(&t.source) {
                                                                                lang.set(t.source);
                                                                            }
                                                                            tr_open.write().insert(row);
                                                                        }
                                                                        Err(e) => ui.fail(e),
                                                                    }
                                                                });
                                                            },
                                                            Icon { width: 14, height: 14, icon: LdSparkles }
                                                            if busy {
                                                                "…"
                                                            } else {
                                                                "{l_ai_tr}"
                                                            }
                                                        }
                                                        button {
                                                            class: "uis-btn ico small subtle",
                                                            title: "{tip_copy_req}",
                                                            onclick: move |e| {
                                                                e.stop_propagation();
                                                                let lang_code = lang_req.clone();
                                                                let source = if lang().is_empty() { String::new() } else { langs::english_name(&lang()).unwrap_or_default().to_owned() };
                                                                let target = langs::english_name(&lang_code).unwrap_or(&lang_code).to_owned();
                                                                match engine::translate::request(&ui.st(), name().trim(), &lyrics(), &source, &target) {
                                                                    Ok(text) => {
                                                                        let _ = arboard::Clipboard::new().and_then(|mut b| b.set_text(text));
                                                                        ui.ok(ui.t("tipDlgAiCopyRequest"));
                                                                    }
                                                                    Err(e) => ui.fail(e),
                                                                }
                                                            },
                                                            Icon { width: 14, height: 14, icon: LdSparkles }
                                                        }
                                                        button {
                                                            class: "uis-btn ico small subtle",
                                                            title: "{ui.t(\"tipDlgCardTranslationCopy\")}",
                                                            onclick: move |e| {
                                                                e.stop_propagation();
                                                                let _ = arboard::Clipboard::new().and_then(|mut b| b.set_text(engine::exports::clean_lyrics(&content_copy)));
                                                                ui.ok(ui.t("tipDlgCardTranslationCopy"));
                                                            },
                                                            Icon { width: 14, height: 14, icon: LdCopy }
                                                        }
                                                        button {
                                                            class: "uis-btn ico small subtle",
                                                            title: "{ui.t(\"tipDlgCardTranslationDelete\")}",
                                                            onclick: move |e| {
                                                                e.stop_propagation();
                                                                {
                                                                    let mut list = trs.write();
                                                                    match list.iter().position(|r| r.id == id_del && r.pending) {
                                                                        Some(i) => {
                                                                            list.remove(i);
                                                                        }
                                                                        None => list.iter_mut().filter(|r| r.id == id_del).for_each(|r| r.deleted = true),
                                                                    }
                                                                }
                                                                tr_open.write().remove(&id_del);
                                                                ui.ok(ui.t("tstDlgCardTranslationDeleted"));
                                                            },
                                                            Icon { width: 14, height: 14, icon: LdTrash2 }
                                                        }
                                                    }
                                                    if open {
                                                        textarea {
                                                            class: "uis-txt-area",
                                                            style: "width: 100%;",
                                                            rows: "{ROWS_TRANSLATION + bonus}",
                                                            value: "{tr.content}",
                                                            oninput: move |e| {
                                                                let v = e.value();
                                                                trs.write().iter_mut().filter(|r| r.id == id_text).for_each(|r| r.content = v.clone());
                                                            },
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    div { style: "margin-top: 4px;",
                                        UisCboStandard {
                                            value: tr_query(),
                                            placeholder: format!("{}…", ui.t("btnDlgCardTranslationAdd")),
                                            items: free_items,
                                            on_change: move |v: String| tr_query.set(v),
                                            on_pick: move |i: usize| {
                                                let Some(code) = free_codes.get(i).cloned() else { return };
                                                let mut list = trs.write();
                                                if list.iter().any(|r| r.lang == code && !r.deleted) {
                                                    return ui.fail(ui.t("tstDlgCardTranslationExists"));
                                                }
                                                let row_id = format!("pending-{code}");
                                                list.retain(|r| r.id != row_id);
                                                list.push(TrRow { id: row_id.clone(), lang: code, name: String::new(), content: String::new(), pending: true, deleted: false });
                                                tr_open.write().insert(row_id);
                                                tr_query.set(String::new());
                                            },
                                        }
                                    }
                                }
                            },
                            Tab::Links => rsx! {
                                div { class: "uis-fld-labelled",
                                    label { class: "uis-fld-labelled-label", style: "color: {accent}; display: flex; align-items: center; gap: 5px;",
                                        Icon { width: 14, height: 14, class: "uis-ico-inline", icon: LdGlobe }
                                        "{ui.t(\"lblDlgCardExternalLinks\")} "
                                        span { style: "{opt_span}", "{opt_label}" }
                                    }
                                    div { style: "display: flex; flex-direction: column; gap: 8px;",
                                        for (i , label , href) in sorted_urls {
                                            UisFldValue {
                                                key: "{i}-{href}",
                                                label: label.clone(),
                                                accent: accent.clone(),
                                                actions: rsx! {
                                                    button {
                                                        class: "uis-btn ico small subtle",
                                                        onclick: {
                                                            let href = href.clone();
                                                            move |_| {
                                                                let _ = open::that(&href);
                                                            }
                                                        },
                                                        Icon { width: 14, height: 14, icon: LdExternalLink }
                                                    }
                                                    UisBtnCopy { text: href.clone(), subtle: true }
                                                    button {
                                                        class: "uis-btn ico small subtle",
                                                        onclick: move |_| {
                                                            urls.write().remove(i);
                                                        },
                                                        Icon { width: 14, height: 14, icon: LdX }
                                                    }
                                                },
                                                "{href}"
                                            }
                                        }
                                        div { style: "display: flex; gap: 6px;",
                                            input { class: "uis-txt", style: "width: 80px; flex-shrink: 0;", placeholder: "{ui.t(\"plhDlgCardUrlLabel\")}", value: "{url_label}", oninput: move |e| url_label.set(e.value()) }
                                            input {
                                                class: "uis-txt",
                                                style: "flex: 1;",
                                                placeholder: if is_song { "https://suno.com/song/…" } else { "https://…" },
                                                value: "{url_href}",
                                                oninput: move |e| url_href.set(e.value()),
                                                onkeydown: move |e| {
                                                    if e.key() == Key::Enter {
                                                        add_link();
                                                    }
                                                },
                                            }
                                            button {
                                                class: "uis-btn primary",
                                                style: "--uis-accent: {accent}; --uis-accent-hov: {accent};",
                                                disabled: url_href().trim().is_empty(),
                                                onclick: move |_| add_link(),
                                                "{ui.t(\"btnDlgCardAdd\")}"
                                            }
                                        }
                                    }
                                }
                                div { class: "uis-fld-labelled",
                                    label { class: "uis-fld-labelled-label", style: "color: {accent}; display: flex; align-items: center; gap: 5px;",
                                        Icon { width: 14, height: 14, class: "uis-ico-inline", icon: LdPlay }
                                        "{ui.t(\"lblDlgCardMediaFile\")} "
                                        span { style: "{opt_span}", "{opt_label}" }
                                    }
                                    div { style: "display: flex; align-items: center; gap: 8px;",
                                        button {
                                            class: "uis-btn",
                                            onclick: move |_| {
                                                spawn(async move {
                                                    if let Some(f) = rfd::AsyncFileDialog::new().set_parent(&dioxus::desktop::window().window).add_filter("media", &engine::media::MEDIA_EXTS).pick_file().await {
                                                        media.set(f.path().to_string_lossy().into_owned());
                                                    }
                                                });
                                            },
                                            Icon { width: 14, height: 14, icon: LdFolderOpen }
                                            "{ui.t(\"btnDlgCardBrowse\")}"
                                        }
                                        if media().is_empty() {
                                            span { class: "uis-hnt", "{ui.t(\"msgDlgCardNoFileSelected\")}" }
                                        } else {
                                            span { style: "font-size: 12px; color: {accent}; flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;", title: "{media}", "{file_name(&media())}" }
                                            button { class: "uis-btn ico small subtle", onclick: move |_| media.set(String::new()), Icon { width: 14, height: 14, icon: LdX } }
                                        }
                                    }
                                }
                            },
                        }
                    }
                }
                div { class: "uis-fld-labelled", style: "padding: 12px 28px 12px; flex-shrink: 0; position: relative; z-index: 10;",
                    label { class: "uis-fld-labelled-label", style: "color: {accent};",
                        "{ui.t(\"lblDlgCardTags\")} "
                        span { style: "{opt_span}", "({ui.t(\"msgDlgCardTagsHint\")})" }
                    }
                    UisCboStandard {
                        value: tags_text(),
                        placeholder: "{plh_tags}",
                        items: tag_items,
                        on_change: move |v: String| tags_text.set(v),
                        on_pick: {
                            let hits = tag_hits.clone();
                            move |i: usize| {
                                if let Some(tg) = hits.get(i) {
                                    tags_text.set(format!("{confirmed}{tg}, "));
                                }
                            }
                        },
                        render_item: render_tag,
                    }
                }
                div { style: "display: flex; justify-content: flex-end; flex-direction: column; gap: 10px; padding: 0 28px 28px; flex-shrink: 0;",
                    if existing.is_some() || is_song {
                        div { class: "uis-hnt", style: "align-self: flex-start; display: flex; gap: 12px; align-items: center;",
                            if existing.is_some() {
                                UisPkrDate { value: date_val(), on_change: move |v: String| date_val.set(v), locale: ui.lang.peek().clone(), today_label: l_today.clone(), clear_label: l_clear.clone() }
                                span { style: "font-weight: 700; color: {accent};", "v{version}" }
                            }
                            if is_song {
                                span { class: "uis-hnt", style: "{over_style(style_total)}", "{ui.t(\"lblDlgCardStyleChars\")} {style_total} {l_chars}" }
                                span { class: "uis-hnt", style: "{over_lyrics(lyrics_total)}", "{ui.t(\"lblDlgCardLyricsChars\")} {lyrics_total} {l_chars}" }
                            }
                        }
                    }
                    div { style: "display: flex; gap: 10px; width: 100%; align-items: center;",
                        if is_song && !ui.st().setting("gemini-key").is_empty() {
                            button { class: "uis-btn", onclick: move |_| wizard_open.set(true),
                                Icon { width: 14, height: 14, icon: LdWand }
                                "{ui.t(\"ttlDlgWizard\")}"
                            }
                        }
                        div { style: "flex: 1;" }
                        button { class: "uis-btn subtle", onclick: move |_| ui.close(), "{ui.t(\"btnGlbCancel\")}" }
                        button { class: "uis-btn primary", style: "--uis-accent: {accent}; --uis-accent-hov: {accent};", onclick: save, "{save_label}" }
                    }
                }
                if wizard_open() {
                    wizard::WizardDlg {
                        on_close: move |_| wizard_open.set(false),
                        on_dna: move |(text, artist): (String, String)| {
                            note.set(text);
                            name.set(artist);
                        },
                        on_done: move |song: wizard::WizardSong| {
                            style.set(song.style);
                            lyrics.set(song.lyrics);
                            if !song.name.is_empty() {
                                name.set(song.name);
                            }
                            if !song.lang.is_empty() {
                                lang.set(song.lang);
                            }
                            if !song.desc.is_empty() {
                                desc.set(song.desc);
                            }
                            // the versions the model wrote go with the card; a language it already holds is replaced
                            let mut list = trs.write();
                            for (code, tr_name, content) in song.translations {
                                for r in list.iter_mut().filter(|r| r.lang == code) {
                                    r.deleted = true;
                                }
                                list.retain(|r| !(r.pending && r.lang == code));
                                tr_open.write().insert(format!("pending-{code}"));
                                list.push(TrRow { id: format!("pending-{code}"), lang: code, name: tr_name, content, pending: true, deleted: false });
                            }
                            kind_s.set("song".to_owned());
                            tab.set(Tab::Style);
                        },
                    }
                }
                if !ai_for().is_empty() {
                    ai::AiDlg {
                        kind: ai_for(),
                        tweaks: ai_tweaks(),
                        on_tweaks: {
                            let card_id = id.clone();
                            move |v: serde_json::Value| {
                                // a saved card keeps it at once; a new one keeps it with the card
                                if let Some(card_id) = &card_id {
                                    let _ = ui.st().update_card(card_id, &CardPatch { ai_tweaks: Some(v.clone()), ..Default::default() });
                                }
                                ai_tweaks.set(v);
                            }
                        },
                        on_close: move |_| ai_for.set(String::new()),
                        on_done: move |(text, names): (String, Vec<String>)| {
                            if ai_for.peek().as_str() == "song" {
                                lyrics.set(text);
                            } else {
                                style.set(text);
                            }
                            if !names.is_empty() {
                                suggestions.set(names);
                            }
                        },
                    }
                }
            }
        }
    }
}

/// The last part of a path: the file's own name.
fn file_name(path: &str) -> String {
    path.rsplit(['\\', '/']).next().unwrap_or(path).to_owned()
}
