//! Fill the music-site address of a project's songs from the project's playlist page.
//!
//! A project may carry the address of a playlist on a music site (SoundCloud). The page holds the tracks
//! in order; each song of the project is matched to one, and its address (with `?in=<playlist>` so the
//! site opens it inside the playlist) becomes the song's link for that site: added, or updated when the
//! song already has one.

use crate::store::{Card, Res, Store, err};
use regex::Regex;
use rusqlite::params;
use std::{collections::HashSet, sync::LazyLock, time::Duration};

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

static HYDRATION: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?s)window\.__sc_hydration\s*=\s*(\[.+?\]);\s*</script>").unwrap());

/// One track of the playlist, in the playlist's order.
#[derive(Clone, Debug, PartialEq)]
pub struct Track {
    pub index: usize,
    pub url: String,
    pub title: String,
}

/// What a sync did.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SyncReport {
    pub matched: usize,
    pub added: usize,
    pub updated: usize,
    pub skipped: usize,
    pub total_songs: usize,
}

/// Letters and digits only, lower case: what two titles are compared on.
fn normalize(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric()).flat_map(|c| c.to_lowercase()).collect()
}

/// The playlist address among a project's links: the first whose host contains one of the music sites.
/// Returns the address and the site that matched (the label the songs' links take).
pub fn playlist_of(links: &[String], sites: &[String]) -> Option<(String, String)> {
    links.iter().find_map(|href| {
        let host = host_of(href)?;
        sites.iter().find(|s| host.contains(s.as_str())).map(|s| (href.clone(), s.clone()))
    })
}

/// The lower-case host of an address, `None` when it is not one.
fn host_of(href: &str) -> Option<String> {
    let rest = href.split_once("://")?.1;
    let host = rest.split(['/', '?', '#']).next()?.rsplit('@').next()?.split(':').next()?;
    (!host.is_empty()).then(|| host.to_lowercase())
}

/// The path of an address, without its first slash.
fn path_of(href: &str) -> String {
    let rest = href.split_once("://").map(|(_, r)| r).unwrap_or(href);
    let after = rest.split_once('/').map(|(_, p)| p).unwrap_or("");
    after.split(['?', '#']).next().unwrap_or("").to_owned()
}

fn get(url: &str) -> Result<String, String> {
    let mut resp = ureq::get(url)
        .header("User-Agent", USER_AGENT)
        .config()
        .http_status_as_error(false)
        .timeout_global(Some(Duration::from_secs(20)))
        .build()
        .call()
        .map_err(err)?;
    if !resp.status().is_success() {
        return Err(format!("Fetch failed: {}", resp.status().as_u16()));
    }
    resp.body_mut().read_to_string().map_err(err)
}

/// The tracks of a playlist page's data (the `__sc_hydration` array). A track given only by id is resolved
/// through `resolve` (the ids in, `(id, url, title)` out); one that cannot be resolved is left out.
fn tracks_of(hydration: &[serde_json::Value], resolve: impl Fn(&[i64], &str) -> Vec<(i64, String, String)>) -> Result<Vec<Track>, String> {
    let find = |name: &str| hydration.iter().find(|h| h["hydratable"] == name).map(|h| &h["data"]);
    let raw = find("playlist").and_then(|d| d["tracks"].as_array()).ok_or("No playlist data found")?;
    let mut tracks = Vec::new();
    let mut stubs: Vec<(i64, usize)> = Vec::new();
    for (i, t) in raw.iter().enumerate() {
        if let Some(url) = t["permalink_url"].as_str().filter(|u| !u.is_empty()) {
            tracks.push(Track { index: i, url: url.to_owned(), title: t["title"].as_str().unwrap_or("").to_owned() });
        } else if let Some(id) = t["id"].as_i64() {
            stubs.push((id, i));
        }
    }
    if !stubs.is_empty() {
        let client = find("apiClient").and_then(|d| d["client_id"].as_str().or(d["id"].as_str())).unwrap_or("");
        if !client.is_empty() {
            let ids: Vec<i64> = stubs.iter().map(|(id, _)| *id).collect();
            for (id, url, title) in resolve(&ids, client) {
                if let Some((_, pos)) = stubs.iter().find(|(s, _)| *s == id) {
                    tracks.push(Track { index: *pos, url, title });
                }
            }
        }
    }
    tracks.sort_by_key(|t| t.index);
    Ok(tracks)
}

/// The stub tracks through the site's own API. A failure leaves them unresolved.
fn resolve_stubs(ids: &[i64], client: &str) -> Vec<(i64, String, String)> {
    let list = ids.iter().map(i64::to_string).collect::<Vec<_>>().join(",");
    let Ok(body) = get(&format!("https://api-v2.soundcloud.com/tracks?ids={list}&client_id={client}")) else { return Vec::new() };
    let Ok(serde_json::Value::Array(items)) = serde_json::from_str::<serde_json::Value>(&body) else { return Vec::new() };
    items.iter().filter_map(|t| Some((t["id"].as_i64()?, t["permalink_url"].as_str()?.to_owned(), t["title"].as_str().unwrap_or("").to_owned()))).collect()
}

/// Match songs to tracks, cascading: sort number + exact name, sort number + partial name, partial name
/// alone. A song is `(sort number, name)`; the result is `(song index, track index)`, one track per song.
pub fn match_songs(songs: &[(Option<i64>, String)], tracks: &[Track]) -> Vec<(usize, usize)> {
    let mut matched: Vec<(usize, usize)> = Vec::new();
    let mut used: HashSet<usize> = HashSet::new();
    let done = |matched: &Vec<(usize, usize)>, s: usize| matched.iter().any(|(m, _)| *m == s);
    // the sort number is 1-based, the track index 0-based
    let slot = |order: Option<i64>| order.and_then(|o| usize::try_from(o - 1).ok()).filter(|i| *i < tracks.len());
    let partial = |a: &str, b: &str| !a.is_empty() && !b.is_empty() && (a.contains(b) || b.contains(a));
    for pass in 0..2 {
        for (s, (order, name)) in songs.iter().enumerate() {
            if done(&matched, s) {
                continue;
            }
            let Some(i) = slot(*order).filter(|i| !used.contains(i)) else { continue };
            let (ns, nt) = (normalize(name), normalize(&tracks[i].title));
            if (pass == 0 && ns == nt) || (pass == 1 && partial(&ns, &nt)) {
                matched.push((s, i));
                used.insert(i);
            }
        }
    }
    for (s, (_, name)) in songs.iter().enumerate() {
        if done(&matched, s) {
            continue;
        }
        let ns = normalize(name);
        if ns.is_empty() {
            continue;
        }
        if let Some(i) = (0..tracks.len()).find(|i| !used.contains(i) && partial(&ns, &normalize(&tracks[*i].title))) {
            matched.push((s, i));
            used.insert(i);
        }
    }
    matched
}

impl Store {
    /// Does this project carry a playlist address on one of the music sites? The sync's button is drawn
    /// enabled by this.
    pub fn has_playlist(&self, project: &str) -> bool {
        self.project(project).ok().flatten().is_some_and(|p| playlist_of(&p.urls.iter().map(|u| u.href.clone()).collect::<Vec<_>>(), &self.music_sites()).is_some())
    }

    /// What a sync needs before the network: the playlist address and the site's name. The store is not
    /// `Send`, so the page is fetched by `fetch_playlist` (on a worker thread) and applied by `apply_sync`.
    pub fn plan_sync(&self, project: &str) -> Res<SyncPlan> {
        let p = self.project(project)?.ok_or("noPlaylistUrl")?;
        let links: Vec<String> = p.urls.iter().map(|u| u.href.clone()).collect();
        let (playlist, site) = playlist_of(&links, &self.music_sites()).ok_or("noPlaylistUrl")?;
        Ok(SyncPlan { playlist, site })
    }

    /// Match the project's songs against `tracks` and write their links.
    pub fn apply_sync(&self, project: &str, plan: &SyncPlan, tracks: &[Track]) -> Res<SyncReport> {
        let songs: Vec<Card> = self.cards()?.into_iter().filter(|c| c.kind == "song" && c.project.as_deref() == Some(project)).collect();
        if songs.is_empty() {
            return Err("noSongs".to_owned());
        }
        let key: Vec<(Option<i64>, String)> = songs.iter().map(|s| (s.sort_number, s.name.clone())).collect();
        let matched = match_songs(&key, tracks);
        let path = path_of(&plan.playlist);
        let mut report = SyncReport { matched: matched.len(), skipped: songs.len() - matched.len(), total_songs: songs.len(), ..Default::default() };
        for (s, t) in matched {
            let href = format!("{}?in={path}", tracks[t].url);
            let existing: Option<String> = self.db.query_row("SELECT url_id FROM url WHERE url_parent_id=? AND url_type='cad' AND LOWER(url_label)=?", params![songs[s].id, plan.site], |r| r.get(0)).ok();
            match existing {
                Some(id) => {
                    self.db.execute("UPDATE url SET url_href=? WHERE url_id=?", params![href, id]).map_err(err)?;
                    report.updated += 1;
                }
                None => {
                    let order: i64 = self.db.query_row("SELECT COUNT(*) FROM url WHERE url_parent_id=?", [&songs[s].id], |r| r.get(0)).map_err(err)?;
                    self.db.execute("INSERT INTO url (url_id,url_parent_id,url_type,url_label,url_href,url_order) VALUES (?,?,?,?,?,?)", params![crate::store::new_id(), songs[s].id, "cad", plan.site, href, order]).map_err(err)?;
                    report.added += 1;
                }
            }
        }
        Ok(report)
    }
}

/// What a sync needs before the network.
#[derive(Clone, Debug)]
pub struct SyncPlan {
    pub playlist: String,
    pub site: String,
}

/// The tracks of the playlist at `url`, in its order. Needs no store, so it can run on a worker thread.
pub fn fetch_playlist(url: &str) -> Res<Vec<Track>> {
    let html = get(url)?;
    let json = HYDRATION.captures(&html).ok_or("Could not parse playlist page")?[1].to_owned();
    let hydration: Vec<serde_json::Value> = serde_json::from_str(&json).map_err(|_| "Could not parse hydration data".to_owned())?;
    let tracks = tracks_of(&hydration, resolve_stubs)?;
    if tracks.is_empty() {
        return Err("No track URLs found".to_owned());
    }
    Ok(tracks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn track(i: usize, title: &str) -> Track {
        Track { index: i, url: format!("https://soundcloud.com/a/t{i}"), title: title.to_owned() }
    }

    #[test]
    fn the_playlist_is_the_first_link_on_a_music_site() {
        let links = vec!["https://example.com/x".to_owned(), "https://soundcloud.com/me/sets/lp".to_owned()];
        let sites = vec!["soundcloud".to_owned()];
        assert_eq!(playlist_of(&links, &sites), Some(("https://soundcloud.com/me/sets/lp".to_owned(), "soundcloud".to_owned())));
        assert_eq!(playlist_of(&links[..1], &sites), None);
        assert_eq!(path_of("https://soundcloud.com/me/sets/lp?x=1"), "me/sets/lp");
    }

    #[test]
    fn songs_match_by_number_then_by_name() {
        let tracks = vec![track(0, "Alpha"), track(1, "Beta (live)"), track(2, "Gamma"), track(3, "Delta")];
        let songs = vec![
            (Some(1), "alpha!".to_owned()),   // number + exact
            (Some(2), "Beta".to_owned()),     // number + partial
            (None, "delta".to_owned()),       // name only
            (Some(1), "Nothing".to_owned()),  // its slot is taken, no name fits
        ];
        assert_eq!(match_songs(&songs, &tracks), vec![(0, 0), (1, 1), (2, 3)]);
    }

    #[test]
    fn a_stub_track_is_resolved_and_keeps_its_place() {
        let hydration = vec![
            json!({"hydratable": "playlist", "data": {"tracks": [{"permalink_url": "https://soundcloud.com/a/one", "title": "One"}, {"id": 7}]}}),
            json!({"hydratable": "apiClient", "data": {"client_id": "abc"}}),
        ];
        let got = tracks_of(&hydration, |ids, client| {
            assert_eq!((ids, client), (&[7][..], "abc"));
            vec![(7, "https://soundcloud.com/a/two".to_owned(), "Two".to_owned())]
        })
        .unwrap();
        assert_eq!(got.iter().map(|t| (t.index, t.title.as_str())).collect::<Vec<_>>(), vec![(0, "One"), (1, "Two")]);
        assert!(tracks_of(&[], |_, _| Vec::new()).is_err());
    }
}
