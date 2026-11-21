// data-icon="yaiol:ai-music-prompt-lab"
// ⚠ NO HARDCODED UI STRINGS IN THIS FILE.
//   Every string a user can read - text, title, aria-label - MUST go through the t() lookup, and
//   the key MUST be a string literal at the call site. Add the key to locales/en.json, then
//   translate, sort and audit every language with the i18n scripts. Never paste translations by hand.

mod ai;
mod card_view;
mod card_dlg;
mod dnd;
mod scrape;
mod translate;
mod catalog;
mod dialogs;
mod exchange;
mod i18n;
mod instance;
mod lucide_extra;
mod player;
mod settings;
mod settings_view;
mod site;
mod ui;
mod views;
mod wizard;
mod window_accent;
mod window_menu;
mod work;

use catalog::{UisAppTab, UisBarWindow, UisIcoGithub, UisBtnNews, UisBtnUpdate, UisNewsLoader, UisStgView, news, toast::use_toast};
use dioxus::{html::HasFileData, prelude::*};
use dioxus_free_icons::{
    Icon,
    icons::ld_icons::{LdCircleHelp, LdFileMusic, LdPipette, LdPlug, LdRotateCcw, LdSettings, LdSparkles, LdTag},
};
use engine::store::Store;
use settings::Settings;
use std::{collections::HashSet, rc::Rc, sync::OnceLock};
use ui::{Dialog, Nav, Tone, Ui, View};

const CSS: [&str; 6] = [
    include_str!("../assets/uis-colors.css"),
    include_str!("../assets/uis-app.css"),
    include_str!("../assets/uis-bar-window.css"),
    include_str!("../assets/uis-sld.css"),
    include_str!("../assets/uis-listview.css"),
    include_str!("../assets/app.css"),
];

static SERVER: OnceLock<engine::server::Server> = OnceLock::new();

/// The local file server the player reads songs from.
pub fn server() -> &'static engine::server::Server {
    SERVER.get_or_init(|| engine::server::Server::start().expect("the local server starts"))
}

/// The database and its settings.
///
/// ⚠ the database is `ampl.db` and the settings `settings.json`, the files the Electron app (now archived) wrote:
/// this app opens them as they are.
fn open_store() -> Result<Rc<Store>, String> {
    let dir = settings::data_dir().ok_or("no data folder")?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let (db, cfg) = (dir.join("ampl.db"), dir.join("settings.json"));
    // The first builds of this app named its files `ampl-rs.db` and `settings-rs.json`: take them over under the
    // real names, once.
    if !db.exists() && dir.join("ampl-rs.db").exists() {
        let _ = std::fs::rename(dir.join("ampl-rs.db"), &db);
    }
    if !cfg.exists() && dir.join("settings-rs.json").exists() {
        let _ = std::fs::rename(dir.join("settings-rs.json"), &cfg);
    }
    let store = Store::open(&db, &cfg)?;
    store.reconcile_ai_seed()?;
    Ok(Rc::new(store))
}

// ─── App identity - single source of truth: Cargo.toml (see build.rs) ───────────────────────────
const APP_NAME: &str = env!("YAIOL_PRODUCT_NAME");
const APP_ID: &str = env!("CARGO_PKG_NAME");
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// This build's identity, `<version>.<counter>`, as ONE contiguous string that survives into the
/// executable. The apps-cockpit finds it by its `yaiol-build:` prefix to tell a buffered build from
/// the release it follows - the counter cannot ride in Cargo's three-part version, and `dx bundle`
/// writes its own Windows version resource, so this is the one channel that is entirely ours.
/// ⚠ keep it a single `concat!` of literals, and read it through `black_box` (as
/// app_version_build does). A plain `BUILD_ID["yaiol-build:".len()..]` is constant-folded and the
/// optimiser keeps only the suffix - the PREFIX the cockpit searches for disappears from the exe and
/// every build reads as the bare release (found 2026-10-12 by grepping the staged exe).
const BUILD_ID: &str = concat!("yaiol-build:", env!("YAIOL_BUILD_ID"));

/// The full build identity shown to the user: the release version plus the build counter (the 4th
/// segment a buffered commit bumps), or just the release version on a clean build.
/// ⚠ DISPLAY ONLY. The release version (`CARGO_PKG_VERSION`) stays plain three-part semver
/// and must not absorb the counter: an update check compares it against the published version, and
/// four parts is not semver.
fn app_version_build() -> String {
    let full = std::hint::black_box(BUILD_ID);
    let id = &full["yaiol-build:".len()..];
    id.strip_suffix(".0").unwrap_or(id).to_owned()
}

/// A window that opens at a known size and place. Only a debug build needs it written out: see main().
#[cfg(debug_assertions)]
fn explicit_placement(window: dioxus::desktop::WindowBuilder) -> dioxus::desktop::WindowBuilder {
    window
        .with_inner_size(dioxus::desktop::LogicalSize::new(1000.0, 700.0))
        .with_position(dioxus::desktop::LogicalPosition::new(120.0, 80.0))
}

#[cfg(not(debug_assertions))]
fn explicit_placement(window: dioxus::desktop::WindowBuilder) -> dioxus::desktop::WindowBuilder {
    window
}

/// The window has no system frame: the app draws its own bar (`UisBarWindow`), which moves it and holds the three
/// window buttons. The system still resizes it from its edges, and on Windows it keeps its shadow.
fn frameless(window: dioxus::desktop::WindowBuilder) -> dioxus::desktop::WindowBuilder {
    let window = window.with_decorations(false);
    #[cfg(windows)]
    let window = {
        use dioxus::desktop::tao::platform::windows::WindowBuilderExtWindows;
        window.with_undecorated_shadow(true)
    };
    window
}

fn main() {
    // one window: a second launch hands over what it was asked to open and ends here
    if !instance::claim() {
        return;
    }
    // ⚠ in a DEBUG build Dioxus writes the window's position and size to `window-state.json` in
    // the temp folder when the app ends and puts the window back there on the next launch - for EVERY
    // Dioxus app on the machine, from the one shared file. A session that ended while the window was
    // MINIMIZED saved (-32000, -32000) and a 200 px square, so the next `cargo run` opened off screen
    // at that size and could not be brought back. Dioxus restores only what the app did NOT set
    // explicitly, so a debug build sets both, here. A release build never restores anything.
    let window = frameless(explicit_placement(dioxus::desktop::WindowBuilder::new().with_title(APP_NAME)));

    dioxus::LaunchBuilder::desktop()
        .with_cfg(
            dioxus::desktop::Config::new()
                // ⚠ the browser's own right-click menu (Back, Reload, Save as, Print, Inspect) is
                // never offered. Dioxus keeps it in a debug build by default - which is how it reached the
                // list on 2026-10-13 - and an app that wants a context menu draws its own.
                .with_disable_context_menu(true)
                .with_window(window)
                .with_menu(None::<dioxus::desktop::muda::Menu>),
        )
        .launch(App);
}

#[derive(Clone)]
struct StoreHandle(Rc<Store>);

impl PartialEq for StoreHandle {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

#[component]
fn App() -> Element {
    let mut opened = use_signal(open_store);
    let result = opened.read().clone();
    match result {
        Ok(store) => rsx! { Shell { store: StoreHandle(store) } },
        // Electron's "can't reach" screen (`apiError`): the data could not be opened; the button tries again
        Err(e) => {
            let lang = use_hook(|| settings::load().lang);
            let (title, retry) = (i18n::t(&lang, "msgAppCannotReach"), i18n::t(&lang, "btnAppRetry"));
            rsx! {
                for css in CSS {
                    style { "{css}" }
                }
                div { class: "uis-app-root",
                    div { class: "uis-emp", style: "flex: 1; display: flex; align-items: center; justify-content: center; flex-direction: column; gap: 20px; padding: 40px;",
                        Icon { icon: LdPlug }
                        h2 { style: "margin: 0; color: var(--uis-text);", "{title}" }
                        p { class: "uis-hnt", "{e}" }
                        button { class: "uis-btn primary", onclick: move |_| opened.set(open_store()),
                            Icon { width: 14, height: 14, icon: LdRotateCcw, style: "margin-right: 6px" }
                            "{retry}"
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn Shell(store: StoreHandle) -> Element {
    let saved = use_hook(settings::load);
    let mut lang = use_signal(|| saved.lang.clone());
    let mut theme = use_signal(|| saved.theme.clone());
    let mut settings_open = use_signal(|| false);
    let st = store.0.clone();
    let (cards0, projects0, envs0, trans0) = use_hook(|| (st.cards().unwrap_or_default(), st.projects().unwrap_or_default(), st.environments().unwrap_or_default(), st.translation_index().unwrap_or_default()));
    let env0 = {
        let remembered = st.setting("active-env");
        if envs0.iter().any(|e| e.id == remembered) { remembered } else { st.default_env.clone() }
    };
    let (colours0, tag_colours0) = ui::load_colours(&st);
    let ui = Ui {
        store: use_signal(|| st.clone()),
        lang,
        toast: use_toast(None),
        cards: use_signal(|| cards0.clone()),
        projects: use_signal(|| projects0.clone()),
        envs: use_signal(|| envs0.clone()),
        trans: use_signal(|| trans0.clone()),
        env: use_signal(|| env0.clone()),
        nav: use_signal(|| Nav::All),
        view: use_signal(|| View::Cards),
        search: use_signal(String::new),
        kind: use_signal(String::new),
        tags: use_signal(Vec::new),
        fav: use_signal(|| false),
        warn: use_signal(String::new),
        // the way the cards and the projects were last sorted and shown (the keys of the Electron app, same values)
        sort_name: use_signal(|| st.setting("sort") == "name"),
        sort_asc: use_signal(|| st.setting("sort-dir") == "asc"),
        proj_sort_name: use_signal(|| st.setting("proj-sort") == "name"),
        proj_sort_asc: use_signal(|| st.setting("proj-sort-dir") == "asc"),
        list_view: use_signal(|| st.setting("view") == "list"),
        selected: use_signal(HashSet::new),
        expanded: use_signal(HashSet::new),
        dialog: use_signal(|| Dialog::None),
        draft_busy: use_signal(|| false),
        drag: use_signal(Default::default),
        drag_at: use_signal(|| (0.0, 0.0)),
        sort_rename: use_signal(|| st.setting("sort-rename-media-default") == "true"),
        copied: use_signal(String::new),
        status: use_signal(|| (Tone::Plain, String::new())),
        playing: use_signal(String::new),
        playlist: use_signal(Vec::new),
        sounding: use_signal(|| false),
        colours: use_signal(|| colours0.clone()),
        tag_colours: use_signal(|| tag_colours0.clone()),
    };
    use_context_provider(|| ui);
    // a `.amlp` opened from the desktop: the file this launch was given, then those later launches hand over
    use_hook(move || {
        spawn(async move {
            let open = |path: String| match std::fs::read_to_string(&path) {
                Ok(text) => exchange::import_amlp_text(ui, &text),
                Err(e) => ui.fail(e.to_string()),
            };
            if let Some(first) = instance::first() {
                open(first.to_string_lossy().into_owned());
            }
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                while let Some(path) = instance::next() {
                    let window = dioxus::desktop::window();
                    window.set_minimized(false);
                    window.set_focus();
                    if !path.is_empty() {
                        open(path);
                    }
                }
            }
        });
    });
    // Ctrl+N is the "new card" button; it stands down while any dialog is open
    use_hook(move || {
        let mut key = document::eval("document.addEventListener('keydown', e => { if (e.ctrlKey && !e.shiftKey && !e.altKey && e.key.toLowerCase() === 'n') { e.preventDefault(); dioxus.send(1); } });");
        spawn(async move {
            while key.recv::<i32>().await.is_ok() {
                if matches!(*ui.dialog.peek(), Dialog::None) {
                    ui.open_new_card();
                }
            }
        });
    });
    // the way of sorting and showing comes back at the next start
    use_effect(move || ui.st().set_setting("sort", if (ui.sort_name)() { "name" } else { "date" }));
    use_effect(move || ui.st().set_setting("sort-dir", if (ui.sort_asc)() { "asc" } else { "desc" }));
    use_effect(move || ui.st().set_setting("proj-sort", if (ui.proj_sort_name)() { "name" } else { "date" }));
    use_effect(move || ui.st().set_setting("proj-sort-dir", if (ui.proj_sort_asc)() { "asc" } else { "desc" }));
    use_effect(move || ui.st().set_setting("view", if (ui.list_view)() { "list" } else { "grid" }));
    // the environment the user was in comes back at the next start
    use_effect(move || ui.st().set_setting("active-env", &(ui.env)()));
    // a view or a selection from another environment makes no sense in this one
    use_effect(move || {
        let _ = (ui.env)();
        let (mut nav, mut selected) = (ui.nav, ui.selected);
        nav.set(Nav::All);
        selected.set(HashSet::new());
    });
    // The window bar shows maximize or restore: the window says which, and says so again whenever its size
    // changes (the bar's own button is one way among several: a snap, Win+Up, a double click).
    let mut maximized = use_signal(|| dioxus::desktop::window().is_maximized());
    // The bar is painted like the system's own title bars: the accent colour when the system colours them, and only
    // while this window is the active one (an inactive title bar is neutral). Read again each time the window comes
    // to the front, so a colour changed in the system's settings shows on return.
    let mut bar_colour = use_signal(window_accent::title_bar);
    // the hidden window that reads a producer.ai page is opened from the event loop
    dioxus::desktop::use_wry_event_handler(move |_, target| scrape::pump(target));
    dioxus::desktop::use_wry_event_handler(move |event, _| {
        if let dioxus::desktop::tao::event::Event::WindowEvent { event, .. } = event {
            match event {
                dioxus::desktop::WindowEvent::Resized(_) => {
                    let now = dioxus::desktop::window().is_maximized();
                    if *maximized.peek() != now {
                        maximized.set(now);
                    }
                }
                dioxus::desktop::WindowEvent::Focused(active) => {
                    let now = if *active { window_accent::title_bar() } else { None };
                    if *bar_colour.peek() != now {
                        bar_colour.set(now);
                    }
                }
                _ => {}
            }
        }
    });
    use_effect(move || {
        let script = match bar_colour() {
            Some(rgb) => {
                let (bgd, text, hov) = window_accent::bar_colours(rgb);
                format!("const s=document.documentElement.style; s.setProperty('--uis-win-bar-bgd','{bgd}'); s.setProperty('--uis-win-bar-text','{text}'); s.setProperty('--uis-win-bar-hov','{hov}');")
            }
            None => "const s=document.documentElement.style; s.removeProperty('--uis-win-bar-bgd'); s.removeProperty('--uis-win-bar-text'); s.removeProperty('--uis-win-bar-hov');".to_owned(),
        };
        let _ = document::eval(&script);
    });
    // "Show news from yaiol": ONE switch per MACHINE, shared by every yaiol app (a small file in the yaiol
    // app-data folder), so it is read from there rather than from this app's own settings.
    let mut news_on = use_signal(|| news::read_state().enabled);


    // <html> carries the language, its text direction and the theme: the catalog CSS keys off
    // `data-theme`, and a right-to-left language needs `dir` for the webview's own text engine.
    use_effect(move || {
        let code = lang();
        let dir = if engine::is_rtl(&code) { "rtl" } else { "ltr" };
        let _ = document::eval(&format!(
            "const r=document.documentElement; r.setAttribute('lang','{}'); r.setAttribute('dir','{dir}'); r.setAttribute('data-theme','{}');",
            code.replace('_', "-"),
            theme()
        ));
    });

    // Every change persists immediately, so leaving the settings page is plain navigation.
    use_effect(move || settings::save(&Settings { lang: lang(), theme: theme() }));

    let code = lang();
    let t = |key: &str| i18n::t(&code, key);
    let tip_help = t("tipHdrHelp");
    let tip_settings = t("tipHdrSettings");
    let (tip_minimize, tip_maximize, tip_restore, tip_close) = (t("tipWinMinimize"), t("tipWinMaximize"), t("tipWinRestore"), t("tipWinClose"));
    let lbl_news = t("lblStgGeneralNews");
    let about_local_db = t("msgStgAboutLocalDb");
    let (tab_cards, tab_tags, tab_media, tab_ai) = (t("tabStgCards"), t("tabStgTags"), t("tabStgMedia"), t("tabStgAi"));

    let version = app_version_build();
    // The app's own icon, shown at the start of the window bar: the file the executable carries, as a page picture.
    let app_icon = use_hook(|| {
        use base64::{Engine, engine::general_purpose::STANDARD};
        format!("data:image/png;base64,{}", STANDARD.encode(include_bytes!("../assets/app.png")))
    });
    let help_url = site::site_url(APP_ID, &code, "help");
    let github_url = format!("https://github.com/yaiol/{APP_ID}");

    rsx! {
        for css in CSS {
            style { "{css}" }
        }
        div {
            class: if ui.dragging() { "uis-app-root ap-dragging" } else { "uis-app-root" },
            onmousemove: move |e| ui.drag_move(&e),
            onmouseup: move |_| ui.drag_end(),
            // files dropped by the system (the page's own drags are pointer events, see dnd.rs)
            ondrop: move |e| {
                let files: Vec<std::path::PathBuf> = e.files().iter().map(|f| f.path()).collect();
                if !files.is_empty() {
                    e.prevent_default();
                    ui.os_drop(files);
                }
            },
            // A new version (first) and the yaiol news, in one dialog; what is shown is decided inside.
            UisNewsLoader { app_id: APP_ID, app_version: APP_VERSION, lang: lang() }
            UisBarWindow {
                app_name: APP_NAME,
                app_version: version.clone(),
                icon_src: app_icon,
                maximized: maximized(),
                tip_minimize,
                tip_maximize,
                tip_restore,
                tip_close,
                on_minimize: move |_| dioxus::desktop::window().set_minimized(true),
                on_maximize: move |_| dioxus::desktop::window().toggle_maximized(),
                on_close: move |_| dioxus::desktop::window().close(),
                on_drag: move |_| dioxus::desktop::window().drag(),
                on_menu: move |_| window_menu::show(),
                button {
                    class: if settings_open() { "uis-barh-window-btn uis-stg-toggle active" } else { "uis-barh-window-btn uis-stg-toggle" },
                    title: "{tip_settings}",
                    "aria-label": "{tip_settings}",
                    "aria-pressed": "{settings_open()}",
                    onclick: move |_| settings_open.toggle(),
                    Icon { width: 14, height: 14, icon: LdSettings }
                }
                button {
                    class: "uis-barh-window-btn",
                    title: "{tip_help}",
                    "aria-label": "{tip_help}",
                    onclick: move |_| {
                        let _ = open::that(&help_url);
                    },
                    Icon { width: 14, height: 14, icon: LdCircleHelp }
                }
                button {
                    class: "uis-barh-window-btn",
                    title: "GitHub",
                    "aria-label": "GitHub",
                    onclick: move |_| {
                        let _ = open::that(&github_url);
                    },
                    UisIcoGithub {}
                }
                // A newer version is out: the reminder, drawn only then; a click reopens the update card.
                UisBtnUpdate { lang: lang() }
                // yaiol has news for this app: the second reminder, drawn only then; a click reopens the announcements.
                UisBtnNews { lang: lang() }
            }
            // The name and the version are in the window bar, so the header holds the work buttons only.
            views::Header { app_name: APP_NAME.to_owned(), app_version: version.clone() }
            // .uis-app-main = the growable content region. It is also the anchor for any
            // content-scoped overlay (settings, file drop).
            div { class: "uis-app-main",
                if settings_open() {
                    UisStgView {
                        app_name: APP_NAME,
                        app_version: version.clone(),
                        lang: lang(),
                        theme: theme(),
                        on_set_lang: move |l| lang.set(l),
                        on_set_theme: move |th| theme.set(th),
                        on_close: move |_| settings_open.set(false),
                        app_tabs: vec![
                            UisAppTab { key: "cards".to_owned(), label: tab_cards, icon: rsx! { Icon { width: 14, height: 14, icon: LdPipette } } },
                            UisAppTab { key: "tags".to_owned(), label: tab_tags, icon: rsx! { Icon { width: 14, height: 14, icon: LdTag } } },
                            UisAppTab { key: "media".to_owned(), label: tab_media, icon: rsx! { Icon { width: 14, height: 14, icon: LdFileMusic } } },
                            UisAppTab { key: "ai".to_owned(), label: tab_ai, icon: rsx! { Icon { width: 14, height: 14, icon: LdSparkles } } },
                        ],
                        app_pages: vec![
                            ("cards".to_owned(), rsx! { settings_view::SettingsCards {} }),
                            ("tags".to_owned(), rsx! { settings_view::SettingsTags {} }),
                            ("media".to_owned(), rsx! { settings_view::SettingsMedia {} }),
                            ("ai".to_owned(), rsx! { settings_view::SettingsAi {} }),
                        ],
                        general_extra: Some(rsx! {
                            div { class: "uis-fld-labelled divider",
                                label { class: "uis-chk-lbl",
                                    input {
                                        r#type: "checkbox",
                                        class: "uis-chk-box",
                                        checked: news_on(),
                                        onchange: move |e| {
                                            news::set_enabled(e.checked());
                                            news_on.set(e.checked());
                                        },
                                    }
                                    "{lbl_news}"
                                }
                            }
                        }),
                        about_extra: Some(rsx! {
                            div { class: "uis-dlg-about-version", "{about_local_db}" }
                        }),
                    }
                }
                views::Workspace {}
            }
            {ui.toast.view()}
            if let Some((_, label)) = ui.drag.read().live.clone() {
                {
                    let (x, y) = (ui.drag_at)();
                    rsx! {
                        div { class: "ap-ghost", style: "left:{x + 14.0}px;top:{y + 10.0}px", "{label}" }
                    }
                }
            }
        }
    }
}
