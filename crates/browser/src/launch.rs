use std::ffi::OsString;
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
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

    let args = browser_args(profile);
    let mut child = match open_command(browser, &args) {
        Some(mut open) => {
            // `open` returns as soon as LaunchServices has started the browser. The
            // browser is then not our child, so there is no process to hold or kill.
            let status = open
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map_err(|e| Error::Launch(e.kind().to_string()))?;
            if !status.success() {
                return Err(Error::Launch("open failed".to_owned()));
            }
            None
        }
        None => Some(spawn_direct(browser, &args)?),
    };

    let deadline = Instant::now() + wait;
    while Instant::now() < deadline {
        if let Some(endpoint) = std::fs::read_to_string(&marker)
            .ok()
            .and_then(|text| parse_active_port(&text))
            && endpoint.reachable()
        {
            // Collect the process when it ends, so it does not linger as a zombie.
            if let Some(mut child) = child {
                std::thread::spawn(move || {
                    let _ = child.wait();
                });
            }
            return Ok(endpoint);
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    // Do not leave a browser behind that nobody holds a connection to. If the
    // start was handed over to a running browser, the child already exited and
    // there is nothing to stop.
    match child.as_mut() {
        Some(child) => {
            let _ = child.kill();
            let _ = child.wait();
            Err(Error::NotReady)
        }
        // Started through LaunchServices: the app has no handle on it.
        None => Err(Error::NotReadyOpened),
    }
}

fn spawn_direct(browser: &Path, args: &[OsString]) -> Result<Child, Error> {
    Command::new(browser)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| Error::Launch(e.kind().to_string()))
}

/// The arguments that make the browser use the dedicated profile and open a
/// debugging port on a free port.
fn browser_args(profile: &Path) -> Vec<OsString> {
    let mut folder = OsString::from("--user-data-dir=");
    folder.push(profile);
    vec![
        folder,
        "--remote-debugging-port=0".into(),
        "--no-first-run".into(),
        "--no-default-browser-check".into(),
    ]
}

/// The `.app` bundle that holds a macOS executable (`X.app/Contents/MacOS/Y`).
/// Anything of another shape is not a bundle the app may hand to `open`.
pub fn mac_bundle(executable: &Path) -> Option<PathBuf> {
    let macos = executable.parent()?;
    let contents = macos.parent()?;
    let bundle = contents.parent()?;
    if macos.file_name()? != "MacOS" || contents.file_name()? != "Contents" {
        return None;
    }
    let name = bundle.file_name()?.to_str()?;
    (name.len() > ".app".len() && name.ends_with(".app")).then(|| bundle.to_path_buf())
}

/// `open -n -a <bundle> --args ...` for a macOS browser, so that LaunchServices
/// starts the browser as its own process. Started as a child of the app, macOS
/// counts what the browser does (it updates itself inside its bundle) as done by
/// the app and blocks it with the App Management protection.
fn open_arguments(bundle: &Path, args: &[OsString]) -> Vec<OsString> {
    let mut out: Vec<OsString> = vec!["-n".into(), "-a".into(), bundle.into(), "--args".into()];
    out.extend(args.iter().cloned());
    out
}

fn open_command(browser: &Path, args: &[OsString]) -> Option<Command> {
    if !cfg!(target_os = "macos") {
        return None;
    }
    let bundle = mac_bundle(browser)?;
    let mut command = Command::new("/usr/bin/open");
    command.args(open_arguments(&bundle, args));
    Some(command)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundle_is_found_from_the_executable() {
        let exe = Path::new("/made/up/Some Browser.app/Contents/MacOS/Some Browser");
        assert_eq!(
            mac_bundle(exe),
            Some(PathBuf::from("/made/up/Some Browser.app"))
        );
    }

    #[test]
    fn other_shapes_are_not_bundles() {
        for bad in [
            "/made/up/browser",
            "/made/up/Some Browser.app/Contents/Other/Some Browser",
            "/made/up/Some Browser.app/Resources/MacOS/Some Browser",
            "/made/up/NotAnApp/Contents/MacOS/Some Browser",
            "/made/up/.app/Contents/MacOS/x",
            "/MacOS/x",
        ] {
            assert_eq!(mac_bundle(Path::new(bad)), None, "{bad}");
        }
    }

    #[test]
    fn open_arguments_start_a_new_instance_of_the_bundle() {
        let args = browser_args(Path::new("/made/up/profile dir"));
        let got = open_arguments(Path::new("/made/up/B.app"), &args);
        let got: Vec<String> = got
            .iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            got,
            [
                "-n",
                "-a",
                "/made/up/B.app",
                "--args",
                "--user-data-dir=/made/up/profile dir",
                "--remote-debugging-port=0",
                "--no-first-run",
                "--no-default-browser-check",
            ]
        );
    }

    #[test]
    fn direct_start_is_used_for_a_path_that_is_not_a_bundle() {
        assert!(open_command(Path::new("/made/up/browser"), &[]).is_none());
    }

    // Starts a real browser with a throwaway profile; run by hand with
    // `BROWSER_TEST_EXE=<executable> cargo test -p browser real_browser -- --ignored`.
    #[test]
    #[ignore = "starts a real browser"]
    fn real_browser_starts_is_reused_and_closes() {
        let exe = PathBuf::from(std::env::var("BROWSER_TEST_EXE").expect("BROWSER_TEST_EXE"));
        let profile = std::env::temp_dir().join(format!("robo-launch-test-{}", std::process::id()));
        let first = start(&exe, &profile, Duration::from_secs(30)).unwrap();
        let second = start(&exe, &profile, Duration::from_secs(30)).unwrap();
        assert_eq!(first, second, "second start must reuse the running browser");
        crate::Cdp::connect(&first).unwrap().close().unwrap();
        let deadline = Instant::now() + Duration::from_secs(15);
        while first.reachable() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(200));
        }
        assert!(!first.reachable(), "browser still reachable after close");
        let _ = std::fs::remove_dir_all(&profile);
    }
}
