//! A small server on this machine only: it gives a song file to the page's player, with byte ranges, so
//! seeking in a large file works. Every address carries a token made at start, so another program on the
//! machine cannot read files through it by guessing the port.

use std::{
    fs,
    hash::{BuildHasher, Hasher},
    io::{self, Read, Seek, SeekFrom, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    thread,
    time::Duration,
};

pub struct Server {
    port: u16,
    token: String,
}

fn random_token() -> String {
    let part = || std::collections::hash_map::RandomState::new().build_hasher().finish();
    format!("{:016x}{:016x}", part(), part())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex(text: &str) -> Option<Vec<u8>> {
    if text.len() % 2 != 0 {
        return None;
    }
    (0..text.len()).step_by(2).map(|i| u8::from_str_radix(text.get(i..i + 2)?, 16).ok()).collect()
}

fn mime(path: &Path) -> &'static str {
    match path.extension().map(|e| e.to_string_lossy().to_lowercase()).as_deref() {
        Some("wav") => "audio/wav",
        Some("ogg" | "opus") => "audio/ogg",
        Some("flac") => "audio/flac",
        Some("m4a") => "audio/mp4",
        Some("aac") => "audio/aac",
        Some("mp4") => "video/mp4",
        Some("webm") => "video/webm",
        _ => "audio/mpeg",
    }
}

impl Server {
    pub fn start() -> io::Result<Server> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let port = listener.local_addr()?.port();
        let token = random_token();
        let expected = token.clone();
        thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let token = expected.clone();
                thread::spawn(move || {
                    let _ = handle(stream, &token);
                });
            }
        });
        Ok(Server { port, token })
    }

    /// The address the page's player loads a song from.
    pub fn audio_url(&self, song: &Path) -> String {
        format!("http://127.0.0.1:{}/audio/{}/{}", self.port, self.token, hex(song.to_string_lossy().as_bytes()))
    }
}

fn respond(stream: &mut TcpStream, status: &str) -> io::Result<()> {
    write!(stream, "HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
}

fn handle(mut stream: TcpStream, token: &str) -> io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    let mut buf = Vec::new();
    let mut chunk = [0u8; 2048];
    loop {
        if buf.windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
        let n = stream.read(&mut chunk)?;
        if n == 0 || buf.len() > 64 * 1024 {
            return Ok(());
        }
        buf.extend_from_slice(&chunk[..n]);
    }
    let head = String::from_utf8_lossy(&buf).into_owned();
    let mut first = head.split("\r\n").next().unwrap_or("").split(' ');
    let (method, target) = (first.next().unwrap_or(""), first.next().unwrap_or(""));
    let range = head.split("\r\n").skip(1).find_map(|l| {
        let (k, v) = l.split_once(':')?;
        k.eq_ignore_ascii_case("range").then(|| v.trim().to_owned())
    });
    let parts: Vec<&str> = target.trim_start_matches('/').split('/').collect();
    match (method, parts.as_slice()) {
        ("GET", ["audio", t, path]) if *t == token => {
            let Some(path) = unhex(path).and_then(|b| String::from_utf8(b).ok()).map(PathBuf::from) else { return respond(&mut stream, "400 Bad Request") };
            serve_file(&mut stream, &path, range.as_deref())
        }
        _ => respond(&mut stream, "404 Not Found"),
    }
}

/// `bytes=<start>-<end?>` against a file of `total` bytes, as the first and last byte to send.
fn byte_range(range: &str, total: u64) -> Option<(u64, u64)> {
    let (start, end) = range.trim().strip_prefix("bytes=")?.split_once('-')?;
    let last = total.checked_sub(1)?;
    if start.is_empty() {
        let n: u64 = end.parse().ok()?;
        return Some((total.saturating_sub(n), last));
    }
    let start: u64 = start.parse().ok()?;
    let end = if end.is_empty() { last } else { end.parse::<u64>().ok()?.min(last) };
    (start <= end).then_some((start, end))
}

fn serve_file(stream: &mut TcpStream, path: &Path, range: Option<&str>) -> io::Result<()> {
    let Ok(mut file) = fs::File::open(path) else { return respond(stream, "404 Not Found") };
    let total = file.metadata()?.len();
    let kind = mime(path);
    let (start, end) = match range {
        Some(r) => match byte_range(r, total) {
            Some(se) => se,
            None => return write!(stream, "HTTP/1.1 416 Range Not Satisfiable\r\nContent-Range: bytes */{total}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"),
        },
        None => (0, total.saturating_sub(1)),
    };
    let length = if total == 0 { 0 } else { end - start + 1 };
    if range.is_some() {
        write!(stream, "HTTP/1.1 206 Partial Content\r\nContent-Type: {kind}\r\nContent-Range: bytes {start}-{end}/{total}\r\nAccept-Ranges: bytes\r\nContent-Length: {length}\r\nConnection: close\r\n\r\n")?;
    } else {
        write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: {kind}\r\nAccept-Ranges: bytes\r\nContent-Length: {length}\r\nConnection: close\r\n\r\n")?;
    }
    file.seek(SeekFrom::Start(start))?;
    io::copy(&mut file.take(length), stream).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges() {
        assert_eq!(byte_range("bytes=0-", 10), Some((0, 9)));
        assert_eq!(byte_range("bytes=2-50", 10), Some((2, 9)));
        assert_eq!(byte_range("bytes=-3", 10), Some((7, 9)));
        assert_eq!(byte_range("bytes=20-", 10), None);
    }

    #[test]
    fn serves_a_song_in_ranges_and_refuses_a_wrong_token() {
        let server = Server::start().unwrap();
        let song = std::env::temp_dir().join(format!("yaiol-ampl-server-{} é.mp3", std::process::id()));
        fs::write(&song, b"0123456789").unwrap();
        let url = server.audio_url(&song);
        let target = url.split_once(&server.port.to_string()).unwrap().1.to_owned();
        let get = |target: &str, range: Option<&str>| {
            let mut s = TcpStream::connect(("127.0.0.1", server.port)).unwrap();
            let range = range.map(|r| format!("Range: {r}\r\n")).unwrap_or_default();
            write!(s, "GET {target} HTTP/1.1\r\nHost: x\r\n{range}\r\n").unwrap();
            let mut out = String::new();
            s.read_to_string(&mut out).unwrap();
            out
        };
        assert!(get(&target, None).ends_with("0123456789"));
        let part = get(&target, Some("bytes=2-5"));
        assert!(part.starts_with("HTTP/1.1 206") && part.ends_with("2345"), "{part}");
        assert!(get(&target.replace(&server.token, "0000"), None).starts_with("HTTP/1.1 404"));
        let _ = fs::remove_file(song);
    }
}
