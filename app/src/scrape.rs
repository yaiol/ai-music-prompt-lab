//! Lyrics and style from a song's public page, whichever site it is on.
//!
//! Suno and Tunee answer a plain request (`engine::songpage`). Producer.ai sits behind a bot check that
//! a plain request does not pass, so its page is opened in a hidden browser window of the app: the check
//! runs there as it would for a visitor, and the text of the page is read once it has landed.

use dioxus::desktop::{
    LogicalSize, WindowBuilder,
    tao::{event_loop::EventLoopWindowTarget, window::Window},
    wry::{WebView, WebViewBuilder},
};
use engine::songpage;
use std::{cell::RefCell, time::Duration};

/// Read off the page's visible text: the block after a keyword, up to the next all-capitals heading.
const EXTRACT: &str = r#"(function() {
  function section(text, keywords) {
    for (const kw of keywords) {
      const idx = text.search(new RegExp('\\b' + kw + '\\b'));
      if (idx === -1) continue;
      const after = text.substring(idx).replace(new RegExp('^' + kw + '[\\r\\n]+'), '').trim();
      const end = after.search(/\n[A-Z]{4,}(\s|\n|$)/);
      const candidate = (end !== -1 ? after.substring(0, end) : after.substring(0, 3000)).trim();
      if (candidate.length > 20) return candidate;
    }
    return '';
  }
  const text = document.body.innerText;
  return JSON.stringify({ lyrics: section(text, ['LYRICS']), style: section(text, ['SOUND', 'STYLE', 'PROMPT']) });
})()"#;

/// Can lyrics be read from this address? A song page of Suno, Producer.ai or Tunee.
pub fn supported(href: &str) -> bool {
    href.contains("suno.com/song/") || href.contains("producer.ai/song/") || href.contains("tunee.ai/music/")
}

/// `(lyrics, style)` of a song page. Errors are short messages for a toast.
pub async fn lyrics_of(url: &str) -> Result<(String, String), String> {
    if url.contains("producer.ai/song/") {
        return producer(url).await;
    }
    let url = url.to_owned();
    let got = tokio::task::spawn_blocking(move || {
        if url.contains("tunee.ai/music/") {
            songpage::fetch_tunee_lyrics(&url).map(|l| (l, String::new()))
        } else {
            songpage::fetch_suno(&url).map(|s| (s.lyrics, s.style))
        }
    })
    .await;
    match got {
        Ok(Ok(pair)) => Ok(pair),
        Ok(Err(songpage::PageError::Gone(_))) => Err("gone".to_owned()),
        Ok(Err(songpage::PageError::Other(m))) => Err(m),
        Err(e) => Err(e.to_string()),
    }
}

// ⚠ the page is read in a window of ITS OWN, not in a second Dioxus window. A Dioxus window refuses
// to navigate to an outside address: it hands the address to the default browser instead and stays on its own
// blank page. The window is created from the event loop (`pump`, called by a handler registered once in the
// root component), because only the loop can create one; everything after that runs where the caller is.
thread_local! {
    static WANTED: RefCell<Option<String>> = const { RefCell::new(None) };
    static HIDDEN: RefCell<Option<(Window, WebView)>> = const { RefCell::new(None) };
}

/// Called on every event of the loop: opens the hidden window when a page was asked for.
pub fn pump<T: 'static>(target: &EventLoopWindowTarget<T>) {
    let Some(url) = WANTED.with(|w| w.borrow_mut().take()) else { return };
    let built = WindowBuilder::new().with_visible(false).with_inner_size(LogicalSize::new(1280.0, 800.0)).build(target).ok().and_then(|w| {
        let v = WebViewBuilder::new().with_url(&url).build(&w).ok()?;
        Some((w, v))
    });
    HIDDEN.with(|h| *h.borrow_mut() = built);
}

/// A producer.ai song page, read in a hidden window: wait until the window has landed on the song page
/// (the bot check may send it through one or two pages first), give the page time to build itself, then
/// read its text. The window is closed whatever happens.
async fn producer(url: &str) -> Result<(String, String), String> {
    WANTED.with(|w| *w.borrow_mut() = Some(url.to_owned()));
    // wake the loop: the request is only seen when an event goes through it
    dioxus::desktop::window().window.request_redraw();
    let result = read_producer().await;
    HIDDEN.with(|h| *h.borrow_mut() = None);
    result
}

fn current_url() -> Option<String> {
    HIDDEN.with(|h| h.borrow().as_ref().and_then(|(_, v)| v.url().ok()))
}

async fn read_producer() -> Result<(String, String), String> {
    let mut waited = 0;
    loop {
        tokio::time::sleep(Duration::from_millis(500)).await;
        waited += 500;
        if current_url().is_some_and(|u| u.contains("producer.ai/song/")) {
            break;
        }
        if waited >= 30_000 {
            return Err(format!("Timeout: page never reached producer.ai (stopped at {})", current_url().unwrap_or_else(|| "no window".to_owned())));
        }
    }
    // the page builds itself after it has loaded
    tokio::time::sleep(Duration::from_secs(3)).await;
    let (tx, rx) = std::sync::mpsc::channel::<String>();
    HIDDEN.with(|h| match h.borrow().as_ref() {
        Some((_, v)) => v.evaluate_script_with_callback(EXTRACT, move |raw| drop(tx.send(raw))).map_err(|e| e.to_string()),
        None => Err("The page window is gone".to_owned()),
    })?;
    let mut raw = None;
    for _ in 0..40 {
        if let Ok(r) = rx.try_recv() {
            raw = Some(r);
            break;
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    // the callback gets the script's result as JSON: a string holding the JSON the script built
    let inner: String = serde_json::from_str(&raw.ok_or("The page did not answer")?).map_err(|e| e.to_string())?;
    let v: serde_json::Value = serde_json::from_str(&inner).map_err(|e| e.to_string())?;
    let text = |k: &str| v[k].as_str().unwrap_or("").trim().to_owned();
    Ok((text("lyrics"), text("style")))
}
