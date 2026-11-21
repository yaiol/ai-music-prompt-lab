//! Reading a song's public page: its lyrics, style, title and date from Suno; its lyrics from Tunee; and
//! turning a Suno share link into the song's own address.
//!
//! These pages are built by a framework that sends the data as chunks of escaped text inside script tags
//! (`self.__next_f.push([1,"..."])`); the readers below collect those chunks and look inside them.
//!
//! The producer.ai reader is not here: that site's bot check needs a real browser window, which only the app
//! can open (`scrape.rs` in the app).
//!
//! Blocking calls: run them off the window's thread.

use regex::Regex;
use std::{sync::LazyLock, time::Duration};

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SunoSong {
    pub lyrics: String,
    pub style: String,
    pub title: String,
    /// As the page gives it (an ISO date and time), or empty.
    pub date: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PageError {
    /// The song is deleted or private: the site answered with this status.
    Gone(u16),
    Other(String),
}

static PUSH: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"self\.__next_f\.push\(\[1,"((?:[^"\\]|\\.)*)"\]\)"#).unwrap());
static UUID: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)/song/([0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})").unwrap());

/// The text of an escaped JSON string (what sits between its quotes).
fn json_text(raw: &str) -> Option<String> {
    serde_json::from_str::<String>(&format!("\"{raw}\"")).ok()
}

/// Every chunk of the page, decoded, in order (a chunk that does not decode counts as empty).
fn pushes(html: &str) -> Vec<String> {
    PUSH.captures_iter(html).map(|c| json_text(&c[1]).unwrap_or_default()).collect()
}

fn decode_entities(s: &str) -> String {
    s.replace("&quot;", "\"").replace("&#039;", "'").replace("&#39;", "'").replace("&#x27;", "'").replace("&#X27;", "'").replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&")
}

/// A text chunk referenced by id: `<id>:T<length in hex>,<content>`, whose content may run over the next
/// chunks until that many BYTES are gathered.
fn text_chunk(pushes: &[String], id: &str, at_line_start_only: bool) -> Option<String> {
    let pattern = if at_line_start_only { format!(r"(?s)\n{}:T([0-9a-f]+),(.*)$", regex::escape(id)) } else { format!(r"(?s)(?:^|\n){}:T([0-9a-f]+),(.*)$", regex::escape(id)) };
    let re = Regex::new(&pattern).ok()?;
    for (i, push) in pushes.iter().enumerate() {
        let Some(m) = re.captures(push) else { continue };
        let expected = usize::from_str_radix(&m[1], 16).ok()?;
        let mut collected = m[2].to_owned();
        for next in &pushes[i + 1..] {
            if collected.len() >= expected {
                break;
            }
            collected.push_str(next);
        }
        let bytes = &collected.as_bytes()[..expected.min(collected.len())];
        let text = String::from_utf8_lossy(bytes).trim().to_owned();
        if !text.is_empty() {
            return Some(text);
        }
    }
    None
}

fn get(url: &str, follow: bool) -> Result<ureq::http::Response<ureq::Body>, PageError> {
    ureq::get(url)
        .header("User-Agent", USER_AGENT)
        .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
        .header("Accept-Language", "en-US,en;q=0.5")
        .config()
        .http_status_as_error(false)
        .max_redirects(if follow { 10 } else { 0 })
        .max_redirects_will_error(false)
        .timeout_global(Some(Duration::from_secs(15)))
        .build()
        .call()
        .map_err(|e| PageError::Other(e.to_string()))
}

/// A Suno share link (`suno.com/s/<code>`) as the song's own address (`https://suno.com/song/<uuid>`).
/// The share link is only a redirect: the address it points at is all that is read. Empty when it points
/// nowhere.
pub fn resolve_suno_share(url: &str) -> Result<String, PageError> {
    if !is_suno_share(url) {
        return Err(PageError::Other("Not a Suno share URL".to_owned()));
    }
    let resp = get(url, false)?;
    let location = if resp.status().is_redirection() { resp.headers().get("location").and_then(|v| v.to_str().ok()).unwrap_or("") } else { "" };
    Ok(UUID.captures(location).map(|c| format!("https://suno.com/song/{}", c[1].to_lowercase())).unwrap_or_default())
}

pub fn is_suno_share(url: &str) -> bool {
    static SHARE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^https?://(www\.)?suno\.com/s/[A-Za-z0-9_-]+").unwrap());
    SHARE.is_match(url)
}

/// Read a Suno song page out of its HTML.
pub fn read_suno_page(html: &str) -> SunoSong {
    static TAGS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#""tags":"((?:[^"\\]|\\.)*)""#).unwrap());
    static DISPLAY_TAGS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#""display_tags":"((?:[^"\\]|\\.)*)""#).unwrap());
    static OG_A: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?i)property="og:title"[^>]*content="([^"]*)""#).unwrap());
    static OG_B: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?i)content="([^"]*)"[^>]*property="og:title""#).unwrap());
    static TITLE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)<title>([^<]*)</title>").unwrap());
    static TITLE_SITE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)\s*\|\s*Suno\s*$").unwrap());
    static TITLE_BY: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)\s+by\s+[^|]*$").unwrap());
    static CREATED: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#""created_at":"([^"]+)""#).unwrap());
    static PROMPT_INLINE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#""prompt":"(\[(?:[^"\\]|\\.)*)""#).unwrap());
    static PROMPT_REF: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#""prompt":"\$([0-9a-f]+)""#).unwrap());

    let pushes = pushes(html);
    let mut song = SunoSong::default();

    // The style: metadata's own "tags" (the first after the object opens), else the display tags.
    // ⚠ these are JSON string values, so the capture is escape-aware and goes through the JSON
    // decoder. A plain "up to the next quote" cuts the style at the first escaped quote.
    for p in &pushes {
        if let Some(at) = p.find("\"metadata\":{") {
            if let Some(c) = TAGS.captures(&p[at..]) {
                song.style = json_text(&c[1]).unwrap_or_else(|| c[1].to_owned()).trim().to_owned();
                break;
            }
        }
        if let Some(c) = DISPLAY_TAGS.captures(p) {
            song.style = json_text(&c[1]).unwrap_or_else(|| c[1].to_owned()).trim().to_owned();
            break;
        }
    }

    // The title: the page's own (bare) title; the window title adds " by <artist> | Suno", so it is the fallback.
    if let Some(c) = OG_A.captures(html).or_else(|| OG_B.captures(html)) {
        song.title = decode_entities(&c[1]).trim().to_owned();
    }
    if song.title.is_empty() {
        if let Some(c) = TITLE.captures(html) {
            let t = decode_entities(&c[1]);
            let t = TITLE_SITE.replace(&t, "");
            song.title = TITLE_BY.replace(&t, "").trim().to_owned();
        }
    }
    if let Some(c) = pushes.iter().find_map(|p| CREATED.captures(p)) {
        song.date = c[1].to_owned();
    }

    // The lyrics, written in place ("prompt":"[Verse]...")...
    for p in &pushes {
        for c in PROMPT_INLINE.captures_iter(p) {
            if let Some(lyrics) = json_text(&c[1]).filter(|l| !l.trim().is_empty()) {
                song.lyrics = lyrics.trim().to_owned();
                return song;
            }
        }
    }
    // ...or as a reference to a text chunk ("prompt":"$3f").
    if let Some(id) = pushes.iter().find_map(|p| PROMPT_REF.captures(p)).map(|c| c[1].to_owned()) {
        if let Some(lyrics) = text_chunk(&pushes, &id, false) {
            song.lyrics = lyrics;
        }
    }
    song
}

/// Fetch a Suno song page.
// ⚠ the STATUS is the answer to "does this song still exist". A deleted or private song answers
// 404 with a page that reads perfectly well (a generic one), so reading the page whatever the status gives
// a plausible EMPTY song instead of "gone", and a card gets made for a song that is not there.
pub fn fetch_suno(url: &str) -> Result<SunoSong, PageError> {
    if !url.contains("suno.com/song/") {
        return Err(PageError::Other("Not a Suno song URL".to_owned()));
    }
    let mut resp = get(url, true)?;
    let status = resp.status().as_u16();
    if !(200..300).contains(&status) {
        return Err(PageError::Gone(status));
    }
    let html = resp.body_mut().with_config().limit(64 * 1024 * 1024).read_to_string().map_err(|e| PageError::Other(e.to_string()))?;
    Ok(read_suno_page(&html))
}

/// Read a Tunee song page's lyrics out of its HTML (empty when it has none).
pub fn read_tunee_page(html: &str) -> String {
    static LYRICS_REF: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#""lyrics":"\$(\d+)""#).unwrap());
    let pushes = pushes(html);
    let Some(id) = pushes.iter().find_map(|p| LYRICS_REF.captures(p)).map(|c| c[1].to_owned()) else { return String::new() };
    text_chunk(&pushes, &id, true).unwrap_or_default()
}

pub fn fetch_tunee_lyrics(url: &str) -> Result<String, PageError> {
    if !url.contains("tunee.ai/music/") {
        return Err(PageError::Other("Not a tunee.ai music URL".to_owned()));
    }
    let mut resp = get(url, true)?;
    let html = resp.body_mut().with_config().limit(64 * 1024 * 1024).read_to_string().map_err(|e| PageError::Other(e.to_string()))?;
    Ok(read_tunee_page(&html))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(chunks: &[&str]) -> String {
        let scripts: String = chunks.iter().map(|c| format!("<script>self.__next_f.push([1,{}])</script>", serde_json::to_string(c).unwrap())).collect();
        format!("<html><head><meta property=\"og:title\" content=\"10-Sanctus &amp; Co\"><title>10-Sanctus by someone | Suno</title></head><body>{scripts}</body></html>")
    }

    #[test]
    fn a_suno_page_with_lyrics_in_place() {
        let html = page(&[r#"x "metadata":{"other":1,"tags":"a driving \"walking\" bass line, strings","prompt":"[Verse]\nfirst line\nsecond"} "created_at":"2026-08-04T09:52:08Z""#]);
        let s = read_suno_page(&html);
        assert_eq!(s.style, "a driving \"walking\" bass line, strings");
        assert_eq!(s.lyrics, "[Verse]\nfirst line\nsecond");
        assert_eq!(s.title, "10-Sanctus & Co");
        assert_eq!(s.date, "2026-08-04T09:52:08Z");
    }

    #[test]
    fn a_suno_page_whose_lyrics_are_a_chunk_spread_over_two_pushes() {
        let lyrics = "[Intro]\nvoilà é\nline";
        let (a, b) = lyrics.split_at(12);
        let html = page(&[r#""display_tags":"slow choir" "prompt":"$3f""#, &format!("1:x\n3f:T{:x},{a}", lyrics.len()), &format!("{b}\nTRAILING")]);
        let s = read_suno_page(&html);
        assert_eq!(s.style, "slow choir");
        assert_eq!(s.lyrics, lyrics);
    }

    #[test]
    fn the_window_title_is_the_fallback_and_a_tunee_page_gives_its_lyrics() {
        let html = "<title>My Song by An Artist | Suno</title>";
        assert_eq!(read_suno_page(html).title, "My Song");
        let tunee = page(&[r#""lyrics":"$12""#, "0:a\n12:T5,hello world"]);
        assert_eq!(read_tunee_page(&tunee), "hello");
        assert_eq!(read_tunee_page("<html></html>"), "");
        assert!(is_suno_share("https://suno.com/s/AbC_12-x") && !is_suno_share("https://suno.com/song/x"));
    }
}
