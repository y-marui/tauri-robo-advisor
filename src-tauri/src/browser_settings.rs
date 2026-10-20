//! The browser and profile folder the person last used.
//!
//! Both are specific to this computer, so they live in a small file in the
//! app's local data folder, never in the portfolio data file (which may sit in
//! a folder the OS syncs). Only the browser kind and the folder path are kept:
//! no credentials, and nothing the sites leave in the profile.

use std::fs;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

const SETTINGS_FILE: &str = "browser-settings.json";

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserSettings {
    /// The last browser started; `None` until one was.
    pub kind: Option<browser::Kind>,
    /// The last profile folder; empty means the default one in the app data folder.
    pub profile: String,
}

/// A saved folder must be a full path without `..`, like the one `open` accepts.
fn usable(profile: &str) -> bool {
    let path = Path::new(profile);
    path.is_absolute() && !path.components().any(|c| c == Component::ParentDir)
}

/// What was saved. A missing, unreadable, or unusable file gives the empty settings.
pub fn load(app_data: &Path) -> BrowserSettings {
    let mut settings = fs::read(app_data.join(SETTINGS_FILE))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<BrowserSettings>(&bytes).ok())
        .unwrap_or_default();
    settings.profile = settings.profile.trim().to_owned();
    if !settings.profile.is_empty() && !usable(&settings.profile) {
        settings.profile.clear();
    }
    settings
}

pub fn save(app_data: &Path, settings: &BrowserSettings) -> Result<(), String> {
    let path: PathBuf = app_data.join(SETTINGS_FILE);
    let json =
        serde_json::to_vec(settings).map_err(|_| "Could not save the setting.".to_owned())?;
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, json)
        .and_then(|()| fs::rename(&temporary, &path))
        .map_err(|e| format!("Could not save the setting ({}).", e.kind()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(label: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("browser-settings-{label}-{nanos}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn nothing_saved_gives_empty_settings() {
        let dir = temp_dir("none");
        assert_eq!(load(&dir), BrowserSettings::default());
    }

    #[test]
    fn saved_values_come_back() {
        let dir = temp_dir("roundtrip");
        let settings = BrowserSettings {
            kind: Some(browser::Kind::Brave),
            profile: "/some/profile".to_owned(),
        };
        save(&dir, &settings).unwrap();
        assert_eq!(load(&dir), settings);
    }

    #[test]
    fn a_broken_file_gives_empty_settings() {
        let dir = temp_dir("broken");
        fs::write(dir.join(SETTINGS_FILE), b"not json").unwrap();
        assert_eq!(load(&dir), BrowserSettings::default());
    }

    #[test]
    fn a_relative_or_parent_folder_is_dropped() {
        for bad in ["relative/dir", "/a/../b"] {
            let dir = temp_dir("bad");
            let settings = BrowserSettings {
                kind: Some(browser::Kind::Chrome),
                profile: bad.to_owned(),
            };
            save(&dir, &settings).unwrap();
            let loaded = load(&dir);
            assert_eq!(loaded.profile, "");
            assert_eq!(loaded.kind, Some(browser::Kind::Chrome));
        }
    }
}
