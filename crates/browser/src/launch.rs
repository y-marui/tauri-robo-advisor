use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::Error;

/// Where the browser's debugging connection listens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endpoint {
    pub port: u16,
    /// The path after the port, `/devtools/browser/<id>`.
    pub path: String,
}

impl Endpoint {
    pub fn url(&self) -> String {
        format!("ws://127.0.0.1:{}{}", self.port, self.path)
    }

    fn reachable(&self) -> bool {
        let address = SocketAddr::from((Ipv4Addr::LOCALHOST, self.port));
        TcpStream::connect_timeout(&address, Duration::from_millis(500)).is_ok()
    }
}

/// The file a Chromium browser writes into its profile folder when it opens a
/// debugging port: the port on the first line, the connection path on the second.
const ACTIVE_PORT_FILE: &str = "DevToolsActivePort";

/// Reads that file. Only a browser endpoint is accepted, so a damaged or
/// swapped file cannot point the app at some other path.
pub fn parse_active_port(contents: &str) -> Option<Endpoint> {
    let mut lines = contents.lines();
    let port: u16 = lines.next()?.trim().parse().ok().filter(|p| *p != 0)?;
    let path = lines.next()?.trim();
    let id = path.strip_prefix("/devtools/browser/")?;
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return None;
    }
    Some(Endpoint {
        port,
        path: path.to_owned(),
    })
}

/// Opens (or reuses) the browser for `profile`, and returns its debugging
/// endpoint.
///
/// The profile folder is the browser's own, not the user's daily one: recent
/// Chromium refuses a debugging port on the default folder, and the port would
/// give control of everything in that profile. A browser already running with
/// this profile is reused. A second start would only be handed over to it and
/// exit, so success is judged by the port answering, never by the exit.
pub fn start(browser: &Path, profile: &Path, wait: Duration) -> Result<Endpoint, Error> {
    std::fs::create_dir_all(profile).map_err(|e| Error::Launch(e.to_string()))?;
    let marker = profile.join(ACTIVE_PORT_FILE);

    if let Some(endpoint) = std::fs::read_to_string(&marker)
        .ok()
        .and_then(|text| parse_active_port(&text))
    {
        if endpoint.reachable() {
            return Ok(endpoint);
        }
        // Left behind by a browser that is gone.
        let _ = std::fs::remove_file(&marker);
    }

    let mut child = Command::new(browser)
        .arg(format!("--user-data-dir={}", profile.display()))
        .arg("--remote-debugging-port=0")
        .arg("--no-first-run")
        .arg("--no-default-browser-check")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| Error::Launch(e.kind().to_string()))?;

    let deadline = Instant::now() + wait;
    while Instant::now() < deadline {
        if let Some(endpoint) = std::fs::read_to_string(&marker)
            .ok()
            .and_then(|text| parse_active_port(&text))
            && endpoint.reachable()
        {
            // Collect the process when it ends, so it does not linger as a zombie.
            std::thread::spawn(move || {
                let _ = child.wait();
            });
            return Ok(endpoint);
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    // Do not leave a browser behind that nobody holds a connection to. If the
    // start was handed over to a running browser, the child already exited and
    // there is nothing to stop.
    let _ = child.kill();
    let _ = child.wait();
    Err(Error::NotReady)
}
