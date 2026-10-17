//! Where the portfolio file lives.
//!
//! By default it is in the app's local data folder. The user may point it at
//! another folder, for example one that the OS already syncs between computers
//! (iCloud Drive). The choice itself is a tiny pointer file that always stays in
//! the local folder, so each computer decides where it reads its data. The app
//! sends nothing anywhere: the sync, if any, is the OS's job.

use std::fs;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The portfolio file inside the data folder.
pub const DATA_FILE: &str = "portfolio.json";
const POINTER_FILE: &str = "data-location.json";
const MAX_LISTED: usize = 20;
const MAX_NAME_CHARS: usize = 100;

#[derive(Serialize, Deserialize)]
struct Pointer {
    folder: PathBuf,
}

/// The folder must be a full path without `..`, so it cannot mean anything else.
pub fn check_folder(folder: &Path) -> Result<(), String> {
    if !folder.is_absolute() || folder.components().any(|c| c == Component::ParentDir) {
        return Err("The data folder must be a full path.".to_owned());
    }
    Ok(())
}

/// The folder the data lives in: the chosen one, or the local folder when none
/// was chosen or the choice cannot be used.
pub fn resolve(app_data: &Path) -> PathBuf {
    let chosen = fs::read(app_data.join(POINTER_FILE))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Pointer>(&bytes).ok())
        .map(|pointer| pointer.folder)
        .filter(|folder| check_folder(folder).is_ok() && folder.is_dir());
    chosen.unwrap_or_else(|| app_data.to_path_buf())
}

/// Remembers the chosen folder, or forgets it (`None`) to go back to the local one.
pub fn write_pointer(app_data: &Path, folder: Option<&Path>) -> Result<(), String> {
    let path = app_data.join(POINTER_FILE);
    match folder {
        None => match fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(format!("Could not save the setting ({}).", e.kind())),
        },
        Some(folder) => {
            let json = serde_json::to_vec(&Pointer {
                folder: folder.to_path_buf(),
            })
            .map_err(|_| "Could not save the setting.".to_owned())?;
            let temporary = path.with_extension("json.tmp");
            fs::write(&temporary, json)
                .and_then(|()| fs::rename(&temporary, &path))
                .map_err(|e| format!("Could not save the setting ({}).", e.kind()))
        }
    }
}

/// The folder that iCloud Drive offers for this app, when iCloud Drive is set
/// up on this computer (macOS).
pub fn icloud_folder() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    let root = Path::new(&home).join("Library/Mobile Documents/com~apple~CloudDocs");
    root.is_dir().then(|| root.join("RoboAdvisor"))
}

/// What a synced folder can show that the user should know about.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct SyncNotes {
    /// Other copies of the data file that the sync created because two
    /// computers changed it at the same time (for example `portfolio 2.json`).
    pub conflict_copies: Vec<String>,
    /// The data file has not been downloaded to this computer yet.
    pub waiting_for_download: bool,
}

pub fn scan(folder: &Path) -> SyncNotes {
    let mut notes = SyncNotes::default();
    let Ok(entries) = fs::read_dir(folder) else {
        return notes;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == format!(".{DATA_FILE}.icloud") {
            notes.waiting_for_download = true;
        } else if name.starts_with("portfolio")
            && name.ends_with(".json")
            && name != DATA_FILE
            && notes.conflict_copies.len() < MAX_LISTED
        {
            notes
                .conflict_copies
                .push(name.chars().take(MAX_NAME_CHARS).collect());
        }
    }
    notes.conflict_copies.sort();
    notes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(label: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "tauri-robo-advisor-folder-{label}-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn the_chosen_folder_is_used_and_forgotten() {
        let local = temp_dir("local");
        let shared = temp_dir("shared");
        assert_eq!(resolve(&local), local);

        write_pointer(&local, Some(&shared)).unwrap();
        assert_eq!(resolve(&local), shared);

        write_pointer(&local, None).unwrap();
        assert_eq!(resolve(&local), local);
        write_pointer(&local, None).unwrap();
    }

    #[test]
    fn an_unusable_choice_falls_back_to_the_local_folder() {
        let local = temp_dir("fallback");
        let missing = local.join("gone");
        write_pointer(&local, Some(&missing)).unwrap();
        assert_eq!(resolve(&local), local, "a missing folder");

        fs::write(local.join(POINTER_FILE), b"not json").unwrap();
        assert_eq!(resolve(&local), local, "a damaged pointer");

        fs::write(local.join(POINTER_FILE), br#"{"folder":"relative/dir"}"#).unwrap();
        assert_eq!(resolve(&local), local, "a relative path");
    }

    #[test]
    fn only_a_full_path_without_parent_steps_is_accepted() {
        let full = std::env::temp_dir().join("x");
        assert!(check_folder(&full).is_ok());
        assert!(check_folder(Path::new("data")).is_err());
        assert!(check_folder(&full.join("..").join("y")).is_err());
    }

    #[test]
    fn conflict_copies_and_missing_downloads_are_reported() {
        let dir = temp_dir("scan");
        assert_eq!(scan(&dir), SyncNotes::default());

        fs::write(dir.join(DATA_FILE), b"{}").unwrap();
        fs::write(dir.join("portfolio 2.json"), b"{}").unwrap();
        fs::write(dir.join("portfolio (conflicted copy).json"), b"{}").unwrap();
        fs::write(dir.join("notes.json"), b"{}").unwrap();
        fs::write(dir.join(".portfolio.json.icloud"), b"").unwrap();

        let notes = scan(&dir);
        assert_eq!(
            notes.conflict_copies,
            vec!["portfolio (conflicted copy).json", "portfolio 2.json"]
        );
        assert!(notes.waiting_for_download);
    }
}
