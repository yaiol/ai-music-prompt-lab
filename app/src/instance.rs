//! One window at a time, and a `.amlp` that is opened from the desktop.
//!
//! ⚠ ONE window. A second launch hands the file it was asked to open to the window already open, over a local
//! socket, and ends. Without this a double-click on a `.amlp` would start a second app on the same database. The
//! debug build has its own port, so a `cargo run` does not take the installed app's place.

use std::{
    io::{BufRead, BufReader, Write},
    net::{Ipv4Addr, TcpListener, TcpStream},
    path::PathBuf,
    sync::{Mutex, OnceLock, mpsc},
    thread,
};

#[cfg(debug_assertions)]
const PORT: u16 = 47832;
#[cfg(not(debug_assertions))]
const PORT: u16 = 47831;

/// What the window has to open: the file this launch was given, then those later launches hand over (an empty line is
/// only "come to the front").
static LATER: OnceLock<Mutex<mpsc::Receiver<String>>> = OnceLock::new();
static FIRST: OnceLock<Option<PathBuf>> = OnceLock::new();

/// The `.amlp` in the command line, if there is one.
fn asked() -> Option<PathBuf> {
    std::env::args().skip(1).map(PathBuf::from).find(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("amlp")))
}

/// Claim the window. `false` = another one is open and has been told what to open: this process ends.
pub fn claim() -> bool {
    let file = asked();
    match TcpListener::bind((Ipv4Addr::LOCALHOST, PORT)) {
        Ok(listener) => {
            let (tx, rx) = mpsc::channel();
            thread::spawn(move || {
                for stream in listener.incoming().flatten() {
                    let mut line = String::new();
                    if BufReader::new(stream).read_line(&mut line).is_ok() {
                        let _ = tx.send(line.trim().to_owned());
                    }
                }
            });
            let _ = LATER.set(Mutex::new(rx));
            let _ = FIRST.set(file);
            true
        }
        Err(_) => match TcpStream::connect((Ipv4Addr::LOCALHOST, PORT)) {
            Ok(mut stream) => {
                let line = file.map(|f| f.to_string_lossy().into_owned()).unwrap_or_default();
                writeln!(stream, "{line}").is_err()
            }
            // the port is held by something that is not this app: run anyway
            Err(_) => {
                let _ = FIRST.set(file);
                true
            }
        },
    }
}

/// The file this launch was given, once.
pub fn first() -> Option<PathBuf> {
    FIRST.get().cloned().flatten()
}

/// The next thing a later launch handed over (`Some("")` = just come to the front).
pub fn next() -> Option<String> {
    LATER.get()?.lock().ok()?.try_recv().ok()
}
