//! Reads and writes the portfolio as one JSON file.
//!
//! The file holds the user's holdings, so error messages never repeat its
//! contents or its path, and it is created readable by the owner only.

use std::fs::{self, File, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::Path;

use portfolio::Portfolio;

/// A portfolio file larger than this is refused instead of read into memory.
const MAX_FILE_BYTES: u64 = 10 * 1024 * 1024;

/// Loads the portfolio, or an empty one when the file does not exist yet.
///
/// A file that exists but cannot be understood is an error. The caller must not
/// overwrite it, so a bad file is never silently replaced by an empty one.
pub fn load(path: &Path) -> Result<Portfolio, String> {
    let length = match fs::metadata(path) {
        Ok(metadata) => metadata.len(),
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Portfolio::default()),
        Err(e) => return Err(format!("Could not read the data file ({}).", e.kind())),
    };
    if length > MAX_FILE_BYTES {
        return Err("The data file is too large to open.".to_owned());
    }
    let bytes =
        fs::read(path).map_err(|e| format!("Could not read the data file ({}).", e.kind()))?;
    let mut portfolio: Portfolio = serde_json::from_slice(&bytes)
        .map_err(|_| "The data file is not valid; it was left untouched.".to_owned())?;
    portfolio
        .validate_integrity()
        .map_err(|e| format!("{e}; the data file was left untouched."))?;
    // An older file is read as it is and saved as the current version.
    portfolio.schema_version = portfolio.schema_version.max(portfolio::SCHEMA_VERSION);
    Ok(portfolio)
}

/// Writes the portfolio atomically: the data goes to a temporary file in the
/// same directory, is flushed to disk, and then replaces the real file, so a
/// crash never leaves a half-written file behind.
pub fn save(path: &Path, portfolio: &Portfolio) -> Result<(), String> {
    portfolio
        .validate_integrity()
        .map_err(|e| format!("Refusing to save inconsistent data: {e}"))?;
    let json = serde_json::to_vec_pretty(portfolio)
        .map_err(|_| "Could not encode the data.".to_owned())?;

    let temporary = path.with_extension("json.tmp");
    let write = || -> std::io::Result<()> {
        let mut file = create_private(&temporary)?;
        file.write_all(&json)?;
        file.sync_all()?;
        fs::rename(&temporary, path)?;
        sync_directory(path);
        Ok(())
    };
    write().map_err(|e| {
        let _ = fs::remove_file(&temporary);
        format!("Could not save the data file ({}).", e.kind())
    })
}

#[cfg(unix)]
fn create_private(path: &Path) -> std::io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt;
    OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)
}

#[cfg(not(unix))]
fn create_private(path: &Path) -> std::io::Result<File> {
    // On Windows the file inherits the per-user permissions of the app data
    // directory, which other standard users cannot read.
    OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
}

/// Persists the rename itself. Best effort: some platforms cannot open a
/// directory for syncing, and the data file is already flushed by then.
fn sync_directory(path: &Path) {
    #[cfg(unix)]
    if let Some(parent) = path.parent()
        && let Ok(directory) = File::open(parent)
    {
        let _ = directory.sync_all();
    }
    #[cfg(not(unix))]
    let _ = path;
}

#[cfg(test)]
mod tests {
    use super::*;
    use portfolio::{AccountKind, Edit};

    /// A fresh empty directory under the system temp directory.
    fn temp_dir(label: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "tauri-robo-advisor-test-{label}-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn sample() -> Portfolio {
        let mut p = Portfolio::default();
        p.apply(Edit::AddAccount {
            name: "Account 1".to_owned(),
            kind: AccountKind::Taxable,
        })
        .unwrap();
        p
    }

    #[test]
    fn load_returns_an_empty_portfolio_when_the_file_does_not_exist() {
        let dir = temp_dir("missing");

        let loaded = load(&dir.join("portfolio.json")).unwrap();

        assert_eq!(loaded, Portfolio::default());
    }

    #[test]
    fn save_then_load_round_trips_and_leaves_no_temporary_file() {
        let dir = temp_dir("roundtrip");
        let path = dir.join("portfolio.json");
        let original = sample();

        save(&path, &original).unwrap();

        assert_eq!(load(&path).unwrap(), original);
        assert!(!path.with_extension("json.tmp").exists());
    }

    #[cfg(unix)]
    #[test]
    fn the_saved_file_is_readable_by_the_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let dir = temp_dir("mode");
        let path = dir.join("portfolio.json");

        save(&path, &sample()).unwrap();

        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn an_invalid_file_is_an_error_and_is_not_overwritten_by_load() {
        let dir = temp_dir("invalid");
        let path = dir.join("portfolio.json");
        fs::write(&path, b"{ this is not json").unwrap();

        let error = load(&path).unwrap_err();

        assert!(error.contains("not valid"));
        assert_eq!(fs::read(&path).unwrap(), b"{ this is not json");
    }

    #[test]
    fn an_inconsistent_file_is_rejected() {
        let dir = temp_dir("inconsistent");
        let path = dir.join("portfolio.json");
        fs::write(&path, br#"{"schema_version":99,"next_id":1}"#).unwrap();

        let error = load(&path).unwrap_err();

        assert!(error.contains("schema version 99"));
    }

    #[test]
    fn a_file_over_the_size_limit_is_refused() {
        let dir = temp_dir("large");
        let path = dir.join("portfolio.json");
        let file = File::create(&path).unwrap();
        file.set_len(MAX_FILE_BYTES + 1).unwrap();

        assert!(load(&path).unwrap_err().contains("too large"));
    }

    #[test]
    fn save_refuses_inconsistent_data() {
        let dir = temp_dir("refuse");
        let path = dir.join("portfolio.json");
        let mut bad = sample();
        bad.next_id = 1;

        assert!(save(&path, &bad).is_err());
        assert!(!path.exists());
    }

    #[test]
    fn a_failed_save_leaves_no_temporary_file() {
        let dir = temp_dir("failed");
        // The target path is a directory, so the final rename cannot succeed.
        let path = dir.join("portfolio.json");
        fs::create_dir(&path).unwrap();

        assert!(save(&path, &sample()).is_err());
        assert!(!path.with_extension("json.tmp").exists());
    }

    #[test]
    fn errors_do_not_contain_the_path_or_the_contents() {
        let dir = temp_dir("secrets");
        let path = dir.join("portfolio.json");
        fs::write(&path, br#"{"secret-holding-name": 1"#).unwrap();

        let error = load(&path).unwrap_err();

        assert!(!error.contains("secret-holding-name"));
        assert!(!error.contains(dir.to_string_lossy().as_ref()));
    }

    #[test]
    fn a_version_1_file_loads_and_is_saved_as_the_current_version() {
        let dir = temp_dir("v1");
        let path = dir.join("portfolio.json");
        fs::write(&path, br#"{"schema_version":1,"next_id":1}"#).unwrap();

        let loaded = load(&path).unwrap();
        assert_eq!(loaded.schema_version, portfolio::SCHEMA_VERSION);
        save(&path, &loaded).unwrap();

        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains(&format!(
            "\"schema_version\": {}",
            portfolio::SCHEMA_VERSION
        )));
    }

    #[test]
    fn a_file_from_a_newer_version_is_refused_and_left_untouched() {
        let dir = temp_dir("v-newer");
        let path = dir.join("portfolio.json");
        let newer = format!(
            r#"{{"schema_version":{},"next_id":1}}"#,
            portfolio::SCHEMA_VERSION + 1
        );
        fs::write(&path, &newer).unwrap();

        assert!(load(&path).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), newer);
    }
}
