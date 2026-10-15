use std::path::{Path, PathBuf};

use serde::Serialize;

/// The Chromium-based browsers the app can use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Chrome,
    Edge,
    Brave,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    pub kind: Kind,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    Mac,
    Windows,
    Linux,
}

impl Os {
    pub fn current() -> Self {
        if cfg!(target_os = "macos") {
            Self::Mac
        } else if cfg!(windows) {
            Self::Windows
        } else {
            Self::Linux
        }
    }
}

/// The places each browser is normally installed. `env` reads an environment
/// variable; nothing here is tied to one user's home folder.
pub fn candidate_paths(os: Os, env: &dyn Fn(&str) -> Option<String>) -> Vec<(Kind, PathBuf)> {
    let mut out = Vec::new();
    match os {
        Os::Mac => {
            let mut roots = vec![PathBuf::from("/Applications")];
            if let Some(home) = env("HOME") {
                roots.push(Path::new(&home).join("Applications"));
            }
            for root in roots {
                out.push((
                    Kind::Chrome,
                    root.join("Google Chrome.app/Contents/MacOS/Google Chrome"),
                ));
                out.push((
                    Kind::Edge,
                    root.join("Microsoft Edge.app/Contents/MacOS/Microsoft Edge"),
                ));
                out.push((
                    Kind::Brave,
                    root.join("Brave Browser.app/Contents/MacOS/Brave Browser"),
                ));
            }
        }
        Os::Windows => {
            for variable in ["PROGRAMFILES", "PROGRAMFILES(X86)", "LOCALAPPDATA"] {
                let Some(base) = env(variable) else { continue };
                let base = PathBuf::from(base);
                out.push((
                    Kind::Chrome,
                    base.join(r"Google\Chrome\Application\chrome.exe"),
                ));
                out.push((
                    Kind::Edge,
                    base.join(r"Microsoft\Edge\Application\msedge.exe"),
                ));
                out.push((
                    Kind::Brave,
                    base.join(r"BraveSoftware\Brave-Browser\Application\brave.exe"),
                ));
            }
        }
        Os::Linux => {
            let path = env("PATH").unwrap_or_default();
            for dir in std::env::split_paths(&path) {
                for (kind, name) in [
                    (Kind::Chrome, "google-chrome"),
                    (Kind::Chrome, "google-chrome-stable"),
                    (Kind::Edge, "microsoft-edge"),
                    (Kind::Edge, "microsoft-edge-stable"),
                    (Kind::Brave, "brave-browser"),
                    (Kind::Brave, "brave"),
                ] {
                    out.push((kind, dir.join(name)));
                }
            }
        }
    }
    out
}

/// The installed browsers, each kind once (the first place it was found), in a
/// fixed order: Chrome, Edge, Brave.
pub fn find_with(
    os: Os,
    env: &dyn Fn(&str) -> Option<String>,
    exists: &dyn Fn(&Path) -> bool,
) -> Vec<Found> {
    let mut found: Vec<Found> = Vec::new();
    for (kind, path) in candidate_paths(os, env) {
        if !found.iter().any(|f| f.kind == kind) && exists(&path) {
            found.push(Found { kind, path });
        }
    }
    found.sort_by_key(|f| f.kind);
    found
}

/// The browsers installed on this computer.
pub fn find_browsers() -> Vec<Found> {
    find_with(Os::current(), &|name| std::env::var(name).ok(), &|path| {
        path.is_file()
    })
}
