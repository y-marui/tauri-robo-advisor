//! Writes a full-record export to a new folder.
//!
//! The export holds the user's holdings, so the folder and its files are
//! created readable by the owner only, nothing is ever overwritten, and the
//! names are fixed by the app: no path comes from the frontend.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use export::ExportFile;

const MAX_SAME_SECOND: u32 = 99;

/// The current time as a file-name stamp (`20261015-012345`) and as an ISO 8601
/// UTC time (`2026-10-15T01:23:45Z`), from seconds since the Unix epoch.
pub fn utc_stamp(seconds: u64) -> (String, String) {
    let days = (seconds / 86_400) as i64;
    let (hour, minute, second) = (seconds % 86_400 / 3600, seconds % 3600 / 60, seconds % 60);
    let (year, month, day) = civil_from_days(days);
    (
        format!("{year:04}{month:02}{day:02}-{hour:02}{minute:02}{second:02}"),
        format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z"),
    )
}

/// Days since 1970-01-01 to a calendar date (Howard Hinnant's algorithm).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * month_index + 2) / 5 + 1) as u32;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    } as u32;
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

/// Creates `<base>/tauri-robo-advisor-export-<stamp>` (with `-2`, `-3`, ... if
/// it exists) and writes the files into it. Returns the folder.
pub fn write_export(base: &Path, stamp: &str, files: &[ExportFile]) -> Result<PathBuf, String> {
    let directory = create_directory(base, stamp)?;
    let result = files.iter().try_for_each(|file| {
        let mut out = create_private_file(&directory.join(file.name))?;
        out.write_all(&file.content)?;
        out.sync_all()
    });
    match result {
        Ok(()) => Ok(directory),
        Err(error) => {
            // Do not leave a half-written export behind.
            let _ = fs::remove_dir_all(&directory);
            Err(format!("Could not write the export ({}).", error.kind()))
        }
    }
}

fn create_directory(base: &Path, stamp: &str) -> Result<PathBuf, String> {
    for attempt in 1..=MAX_SAME_SECOND {
        let name = if attempt == 1 {
            format!("tauri-robo-advisor-export-{stamp}")
        } else {
            format!("tauri-robo-advisor-export-{stamp}-{attempt}")
        };
        let candidate = base.join(name);
        match create_private_directory(&candidate) {
            Ok(()) => return Ok(candidate),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(format!(
                    "Could not create the export folder ({}).",
                    error.kind()
                ));
            }
        }
    }
    Err("Could not create a new export folder.".to_owned())
}

#[cfg(unix)]
fn create_private_directory(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::DirBuilderExt;
    fs::DirBuilder::new().mode(0o700).create(path)
}

#[cfg(not(unix))]
fn create_private_directory(path: &Path) -> std::io::Result<()> {
    // The folder inherits the per-user permissions of the download directory.
    fs::create_dir(path)
}

#[cfg(unix)]
fn create_private_file(path: &Path) -> std::io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt;
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
}

#[cfg(not(unix))]
fn create_private_file(path: &Path) -> std::io::Result<File> {
    OpenOptions::new().write(true).create_new(true).open(path)
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
            "tauri-robo-advisor-exporter-{label}-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn files() -> Vec<ExportFile> {
        vec![
            ExportFile {
                name: "holdings.csv",
                content: b"a,b\r\n".to_vec(),
            },
            ExportFile {
                name: "README.txt",
                content: b"hello".to_vec(),
            },
        ]
    }

    #[test]
    fn stamps_are_correct_for_known_instants() {
        assert_eq!(
            utc_stamp(0),
            (
                "19700101-000000".to_owned(),
                "1970-01-01T00:00:00Z".to_owned()
            )
        );
        assert_eq!(
            utc_stamp(1_700_000_000),
            (
                "20231114-221320".to_owned(),
                "2023-11-14T22:13:20Z".to_owned()
            )
        );
        // A leap day and the end of a year.
        assert_eq!(utc_stamp(1_709_164_800).1, "2024-02-29T00:00:00Z");
        assert_eq!(utc_stamp(1_798_761_599).1, "2026-12-31T23:59:59Z");
    }

    #[test]
    fn the_export_is_written_to_a_new_folder() {
        let base = temp_dir("write");

        let folder = write_export(&base, "20261015-012345", &files()).unwrap();

        assert_eq!(
            folder.file_name().unwrap(),
            "tauri-robo-advisor-export-20261015-012345"
        );
        assert_eq!(fs::read(folder.join("holdings.csv")).unwrap(), b"a,b\r\n");
        assert_eq!(fs::read(folder.join("README.txt")).unwrap(), b"hello");
    }

    #[test]
    fn an_existing_export_is_never_overwritten() {
        let base = temp_dir("again");

        let first = write_export(&base, "20261015-012345", &files()).unwrap();
        let second = write_export(&base, "20261015-012345", &files()).unwrap();

        assert_ne!(first, second);
        assert!(second.to_string_lossy().ends_with("-2"));
        assert!(first.join("README.txt").exists());
    }

    #[cfg(unix)]
    #[test]
    fn the_folder_and_files_are_readable_by_the_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let base = temp_dir("mode");

        let folder = write_export(&base, "20261015-012345", &files()).unwrap();

        let mode = |path: &Path| fs::metadata(path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(&folder), 0o700);
        assert_eq!(mode(&folder.join("holdings.csv")), 0o600);
    }

    #[test]
    fn a_failed_write_leaves_no_folder_behind() {
        let base = temp_dir("fail");
        let bad = vec![
            ExportFile {
                name: "one.csv",
                content: b"x".to_vec(),
            },
            // The same name twice: the second `create_new` fails.
            ExportFile {
                name: "one.csv",
                content: b"y".to_vec(),
            },
        ];

        assert!(write_export(&base, "20261015-012345", &bad).is_err());

        assert_eq!(fs::read_dir(&base).unwrap().count(), 0);
    }

    #[test]
    fn errors_do_not_contain_the_path() {
        let missing = temp_dir("missing").join("does-not-exist");

        let error = write_export(&missing, "20261015-012345", &files()).unwrap_err();

        assert!(!error.contains(missing.to_string_lossy().as_ref()));
    }
}
