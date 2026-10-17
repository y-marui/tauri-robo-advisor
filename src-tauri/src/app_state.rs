//! The portfolio held in memory together with the file it is saved to.
//!
//! This is separate from the Tauri commands so it can be tested without Tauri.

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::SystemTime;

use portfolio::{Edit, ImportPlan, ImportRow, Issue, Portfolio};
use serde::Serialize;
use suggest::{AccountBalance, Overview, Request, Suggestion};
use tabular::{ColumnMapping, Skipped, Table};

use crate::data_folder::{self, DATA_FILE, SyncNotes};
use crate::store;

/// Where the data lives and what the folder shows, for the settings card.
#[derive(Debug, Serialize)]
pub struct DataInfo {
    /// The folder with the data file.
    pub folder: String,
    /// `true` when it is the app's own local folder (no folder was chosen).
    pub is_local: bool,
    #[serde(flatten)]
    pub notes: SyncNotes,
    /// Why a newer file on disk could not be used (never its contents).
    pub warning: Option<String>,
}

/// The portfolio plus the problems that block suggestions.
#[derive(Debug, Serialize)]
pub struct PortfolioView {
    pub portfolio: Portfolio,
    pub issues: Vec<Issue>,
    pub data: DataInfo,
}

impl PortfolioView {
    fn new(portfolio: &Portfolio, data: DataInfo) -> Self {
        Self {
            portfolio: portfolio.clone(),
            issues: portfolio.issues(),
            data,
        }
    }
}

type Fingerprint = (SystemTime, u64);

/// The data file on disk, and what was last seen of it.
struct Disk {
    path: PathBuf,
    seen: Option<Fingerprint>,
    warning: Option<String>,
}

/// The message when another computer saved first; the frontend shows it as is.
const CHANGED_ELSEWHERE: &str =
    "The data file was changed on another computer. It has been reloaded; please try again.";
/// The message that asks the user to confirm using data that is already in the chosen folder.
pub const FOLDER_HAS_DATA: &str = "FOLDER_HAS_DATA";

/// Adopts the file on disk when it is newer than what is in memory (another
/// computer saved it). A file that cannot be read is reported and left alone.
fn refresh(portfolio: &mut Result<Portfolio, String>, disk: &mut Disk) {
    let now = store::fingerprint(&disk.path);
    if now == disk.seen {
        return;
    }
    disk.seen = now;
    if now.is_none() {
        return;
    }
    match store::load(&disk.path) {
        Ok(on_disk) => {
            disk.warning = None;
            let newer = match portfolio {
                Ok(current) => on_disk.revision > current.revision,
                Err(_) => true,
            };
            if newer {
                *portfolio = Ok(on_disk);
            }
        }
        Err(error) => disk.warning = Some(error),
    }
}

pub struct AppState {
    /// The fixed local folder: the pointer to the chosen data folder lives here.
    app_data: PathBuf,
    disk: Mutex<Disk>,
    /// `Err` holds why the stored file could not be loaded. While it is an
    /// error nothing is written, so the unreadable file is never overwritten.
    portfolio: Mutex<Result<Portfolio, String>>,
    /// The table last read for an import, kept here so the frontend does not
    /// send a large table back and forth. It holds holdings, so it is dropped
    /// as soon as it is replaced or cleared.
    table: Mutex<Option<Table>>,
    /// The browser opened for a fetch, and the tables read from it.
    pub browser: crate::browser_session::BrowserSession,
}

/// How many rows and how many characters per cell the preview shows.
const PREVIEW_ROWS: usize = 30;
const PREVIEW_CELL_CHARS: usize = 60;

/// The top of a table read for an import, enough to choose the columns.
#[derive(Debug, Serialize)]
pub struct TablePreview {
    pub rows: Vec<Vec<String>>,
    pub total_rows: usize,
    pub columns: usize,
    pub encoding: Option<tabular::Encoding>,
    pub delimiter: Option<tabular::Delimiter>,
}

/// What importing the loaded table would do.
#[derive(Debug, Serialize)]
pub struct TableImportPlan {
    pub plan: ImportPlan,
    /// Rows that were left out, so nothing is dropped silently.
    pub skipped: Vec<Skipped>,
    pub rows_read: usize,
}

impl AppState {
    /// Opens the data in the folder the user chose, or in the local app folder.
    pub fn open(app_data: PathBuf) -> Self {
        let path = data_folder::resolve(&app_data).join(DATA_FILE);
        Self::with_paths(app_data, path)
    }

    /// Opens one data file; the pointer folder is the file's own folder.
    #[cfg(test)]
    pub fn load(path: PathBuf) -> Self {
        let app_data = path.parent().map(PathBuf::from).unwrap_or_default();
        Self::with_paths(app_data, path)
    }

    fn with_paths(app_data: PathBuf, path: PathBuf) -> Self {
        let portfolio = Mutex::new(store::load(&path));
        let seen = store::fingerprint(&path);
        Self {
            app_data,
            disk: Mutex::new(Disk {
                path,
                seen,
                warning: None,
            }),
            portfolio,
            table: Mutex::new(None),
            browser: crate::browser_session::BrowserSession::default(),
        }
    }

    fn data_info(&self, disk: &Disk) -> DataInfo {
        let folder = disk.path.parent().map(PathBuf::from).unwrap_or_default();
        DataInfo {
            is_local: folder == self.app_data,
            notes: data_folder::scan(&folder),
            folder: folder.display().to_string(),
            warning: disk.warning.clone(),
        }
    }

    pub fn view(&self) -> Result<PortfolioView, String> {
        let guard = self.lock()?;
        let disk = self.lock_disk()?;
        guard
            .as_ref()
            .map(|p| PortfolioView::new(p, self.data_info(&disk)))
            .map_err(Clone::clone)
    }

    /// Applies one edit and saves it. The change takes effect in memory only
    /// after the file was written, so memory and disk never disagree. A newer
    /// file from another computer is adopted first, and the edit is refused if
    /// the file turns out to have changed while it was being applied, so an
    /// accepted edit is never lost to a stale copy.
    pub fn apply(&self, edit: Edit) -> Result<PortfolioView, String> {
        let mut guard = self.lock()?;
        let mut disk = self.lock_disk()?;
        let base_revision = guard.as_ref().map_err(Clone::clone)?.revision;
        let mut next = guard.as_ref().map_err(Clone::clone)?.clone();
        next.apply(edit).map_err(|e| e.to_string())?;
        if store::peek_revision(&disk.path).is_some_and(|on_disk| on_disk > base_revision) {
            disk.seen = None; // read the newer file on the next look
            return Err(CHANGED_ELSEWHERE.to_owned());
        }
        store::save(&disk.path, &next)?;
        disk.seen = store::fingerprint(&disk.path);
        let view = PortfolioView::new(&next, self.data_info(&disk));
        *guard = Ok(next);
        Ok(view)
    }

    /// Moves the data to another folder (or back to the local one with `None`).
    /// If the folder already has a data file, the caller must confirm with
    /// `use_existing`; that file is then used as it is. Otherwise the current
    /// data is copied there. The old file is left where it was, as a backup.
    pub fn set_data_folder(
        &self,
        folder: Option<PathBuf>,
        use_existing: bool,
    ) -> Result<PortfolioView, String> {
        let mut guard = self.lock()?;
        let mut disk = self.lock_disk()?;
        let target_dir = match &folder {
            Some(folder) => {
                data_folder::check_folder(folder)?;
                std::fs::create_dir_all(folder)
                    .map_err(|e| format!("Could not use that folder ({}).", e.kind()))?;
                folder.clone()
            }
            None => self.app_data.clone(),
        };
        let target = target_dir.join(DATA_FILE);
        if target != disk.path {
            if store::fingerprint(&target).is_some() {
                if !use_existing {
                    return Err(FOLDER_HAS_DATA.to_owned());
                }
                *guard = Ok(store::load(&target)?);
            } else {
                store::save(&target, guard.as_ref().map_err(Clone::clone)?)?;
            }
            data_folder::write_pointer(&self.app_data, folder.as_deref())?;
            disk.path = target;
            disk.seen = store::fingerprint(&disk.path);
            disk.warning = None;
        }
        let portfolio = guard.as_ref().map_err(Clone::clone)?;
        Ok(PortfolioView::new(portfolio, self.data_info(&disk)))
    }

    /// Builds the full-record export from the current portfolio.
    pub fn export_files(&self, info: &export::Info) -> Result<Vec<export::ExportFile>, String> {
        let guard = self.lock()?;
        let portfolio = guard.as_ref().map_err(Clone::clone)?;
        export::build(portfolio, info).map_err(|e| e.to_string())
    }

    /// Computes a suggestion from the current portfolio. Nothing is changed or
    /// saved.
    pub fn suggest(&self, request: &Request) -> Result<Suggestion, String> {
        let guard = self.lock()?;
        let portfolio = guard.as_ref().map_err(Clone::clone)?;
        suggest::suggest(portfolio, request).map_err(|e| e.to_string())
    }

    /// Values every category for the balance charts. Nothing is changed or saved.
    pub fn overview(&self) -> Result<Overview, String> {
        let guard = self.lock()?;
        let portfolio = guard.as_ref().map_err(Clone::clone)?;
        suggest::overview(portfolio).map_err(|e| e.to_string())
    }

    /// Compares the products inside one account with their weights. Nothing is
    /// changed or saved.
    pub fn account_balance(&self, account: portfolio::AccountId) -> Result<AccountBalance, String> {
        let guard = self.lock()?;
        let portfolio = guard.as_ref().map_err(Clone::clone)?;
        suggest::account_balance(portfolio, account).map_err(|e| e.to_string())
    }

    /// Links imported rows to products by alias. Nothing is changed or saved.
    pub fn plan_import(
        &self,
        source: &str,
        currency: &str,
        rows: &[ImportRow],
    ) -> Result<ImportPlan, String> {
        let guard = self.lock()?;
        let portfolio = guard.as_ref().map_err(Clone::clone)?;
        Ok(portfolio.plan_import(source, currency, rows))
    }

    /// Keeps a table read for an import and returns its top for the preview.
    pub fn keep_table(&self, table: Table) -> Result<TablePreview, String> {
        let preview = TablePreview {
            rows: table
                .rows
                .iter()
                .take(PREVIEW_ROWS)
                .map(|row| {
                    row.iter()
                        .map(|cell| cell.chars().take(PREVIEW_CELL_CHARS).collect())
                        .collect()
                })
                .collect(),
            total_rows: table.rows.len(),
            columns: table.columns(),
            encoding: table.encoding,
            delimiter: table.delimiter,
        };
        *self.lock_table()? = Some(table);
        Ok(preview)
    }

    pub fn clear_table(&self) -> Result<(), String> {
        *self.lock_table()? = None;
        Ok(())
    }

    /// Picks the chosen columns of the kept table and links the rows to products
    /// by alias. Nothing is changed or saved.
    pub fn plan_table_import(
        &self,
        source: &str,
        currency: &str,
        mapping: &ColumnMapping,
    ) -> Result<TableImportPlan, String> {
        let extraction = {
            let table = self.lock_table()?;
            let table = table
                .as_ref()
                .ok_or_else(|| "No table is loaded.".to_owned())?;
            tabular::extract(&table.rows, mapping)
        };
        let rows: Vec<ImportRow> = extraction
            .rows
            .iter()
            .map(|r| ImportRow {
                name: r.name.clone(),
                value: r.value,
            })
            .collect();
        let plan = self.plan_import(source, currency, &rows)?;
        Ok(TableImportPlan {
            plan,
            skipped: extraction.skipped,
            rows_read: rows.len(),
        })
    }

    fn lock_table(&self) -> Result<std::sync::MutexGuard<'_, Option<Table>>, String> {
        self.table
            .lock()
            .map_err(|_| "The data is temporarily unavailable.".to_owned())
    }

    /// Locks the portfolio after taking in a newer file from disk, if there is one.
    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Result<Portfolio, String>>, String> {
        let mut guard = self
            .portfolio
            .lock()
            .map_err(|_| "The data is temporarily unavailable.".to_owned())?;
        {
            let mut disk = self.lock_disk()?;
            refresh(&mut guard, &mut disk);
        }
        Ok(guard)
    }

    fn lock_disk(&self) -> Result<std::sync::MutexGuard<'_, Disk>, String> {
        self.disk
            .lock()
            .map_err(|_| "The data is temporarily unavailable.".to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use portfolio::{AccountKind, Issue};

    fn temp_path(label: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "tauri-robo-advisor-state-{label}-{}-{nanos}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("portfolio.json")
    }

    fn add_account(name: &str) -> Edit {
        Edit::AddAccount {
            name: name.to_owned(),
            kind: AccountKind::Taxable,
        }
    }

    #[test]
    fn apply_updates_memory_and_persists_to_disk() {
        let path = temp_path("persist");
        let state = AppState::load(path.clone());

        let view = state.apply(add_account("Account 1")).unwrap();

        assert_eq!(view.portfolio.accounts.len(), 1);
        let reloaded = AppState::load(path);
        assert_eq!(reloaded.view().unwrap().portfolio.accounts.len(), 1);
    }

    #[test]
    fn view_reports_issues() {
        let state = AppState::load(temp_path("issues"));
        state
            .apply(Edit::AddCategory {
                name: "A".to_owned(),
                target_weight: rust_decimal::Decimal::new(5, 1),
            })
            .unwrap();

        let view = state.view().unwrap();

        assert!(matches!(
            view.issues.as_slice(),
            [Issue::CategoryWeightsDoNotSumToOne { .. }]
        ));
    }

    #[test]
    fn suggest_reports_why_there_is_nothing_to_suggest() {
        let state = AppState::load(temp_path("suggest"));
        state.apply(add_account("Account 1")).unwrap();
        let account = state.view().unwrap().portfolio.accounts[0].id;

        let error = state
            .suggest(&Request {
                account,
                mode: suggest::Mode::BuyOnlyToTarget,
            })
            .unwrap_err();

        assert!(error.contains("nothing in the portfolio"));
    }

    #[test]
    fn a_rejected_edit_changes_neither_memory_nor_disk() {
        let path = temp_path("rejected");
        let state = AppState::load(path.clone());
        state.apply(add_account("Account 1")).unwrap();
        let on_disk = std::fs::read(&path).unwrap();

        let error = state.apply(add_account("Account 1")).unwrap_err();

        assert!(error.contains("exists"));
        assert_eq!(state.view().unwrap().portfolio.accounts.len(), 1);
        assert_eq!(std::fs::read(&path).unwrap(), on_disk);
    }

    #[test]
    fn a_failed_save_leaves_memory_unchanged() {
        let path = temp_path("failedsave");
        let state = AppState::load(path.clone());
        state.apply(add_account("Account 1")).unwrap();
        // Make the final rename impossible: the target becomes a directory.
        std::fs::remove_file(&path).unwrap();
        std::fs::create_dir(&path).unwrap();

        assert!(state.apply(add_account("Account 2")).is_err());

        assert_eq!(state.view().unwrap().portfolio.accounts.len(), 1);
    }

    #[test]
    fn an_unreadable_file_blocks_reads_and_writes_and_stays_untouched() {
        let path = temp_path("broken");
        std::fs::write(&path, b"not json").unwrap();
        let state = AppState::load(path.clone());

        assert!(state.view().is_err());
        assert!(state.apply(add_account("Account 1")).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"not json");
    }

    #[test]
    fn a_loaded_table_is_planned_with_the_chosen_columns_and_dropped_when_cleared() {
        let state = AppState::load(temp_path("table"));
        state
            .apply(Edit::AddCategory {
                name: "A".to_owned(),
                target_weight: rust_decimal::Decimal::ONE,
            })
            .unwrap();
        let category = state.view().unwrap().portfolio.categories[0].id;
        state
            .apply(Edit::AddProduct {
                name: "Fund".to_owned(),
                category,
                currency: "JPY".to_owned(),
            })
            .unwrap();
        let product = state.view().unwrap().portfolio.products[0].id;
        state
            .apply(Edit::AddAlias {
                product,
                source: "Test".to_owned(),
                name: "Fund X".to_owned(),
            })
            .unwrap();
        let table =
            tabular::read_text("name,value\nFund X,\"1,000\"\nUnknown,5\ntotal,-\n").unwrap();
        let preview = state.keep_table(table).unwrap();
        assert_eq!(preview.total_rows, 4);

        let mapping = ColumnMapping {
            name: 0,
            value: 1,
            price: None,
            price_unit: 1,
            header_rows: 1,
            rows_per_record: 1,
            name_row: 0,
            value_row: 0,
        };
        let result = state.plan_table_import("Test", "JPY", &mapping).unwrap();

        assert_eq!(result.rows_read, 2);
        assert_eq!(result.plan.matched.len(), 1);
        assert_eq!(result.plan.unmatched.len(), 1);
        assert_eq!(result.skipped.len(), 1);

        state.clear_table().unwrap();
        assert!(state.plan_table_import("Test", "JPY", &mapping).is_err());
    }

    fn account_names(state: &AppState) -> Vec<String> {
        state
            .view()
            .unwrap()
            .portfolio
            .accounts
            .iter()
            .map(|a| a.name.clone())
            .collect()
    }

    #[test]
    fn a_newer_file_saved_by_another_computer_is_picked_up() {
        let path = temp_path("newer");
        let a = AppState::load(path.clone());
        let b = AppState::load(path);
        a.apply(add_account("From A")).unwrap();

        assert_eq!(account_names(&b), vec!["From A"]);
        assert_eq!(b.view().unwrap().portfolio.revision, 1);
    }

    #[test]
    fn an_edit_on_a_stale_copy_adopts_the_newer_file_first_and_loses_nothing() {
        let path = temp_path("stale");
        let a = AppState::load(path.clone());
        let b = AppState::load(path);
        a.apply(add_account("From A")).unwrap();

        b.apply(add_account("From B")).unwrap();

        assert_eq!(account_names(&b), vec!["From A", "From B"]);
        assert_eq!(account_names(&a), vec!["From A", "From B"]);
        assert_eq!(a.view().unwrap().portfolio.revision, 2);
    }

    #[test]
    fn a_save_that_lost_the_race_is_refused_and_the_newer_file_is_kept() {
        let path = temp_path("race");
        let a = AppState::load(path.clone());
        let b = AppState::load(path.clone());
        a.apply(add_account("From A")).unwrap();
        // B has not looked at the disk yet and wrongly believes it is up to date.
        b.disk.lock().unwrap().seen = store::fingerprint(&path);

        let error = b.apply(add_account("From B")).unwrap_err();

        assert!(error.contains("changed on another computer"));
        assert_eq!(store::peek_revision(&path), Some(1));
        // The next look reads the newer file, and the edit can be made again.
        b.apply(add_account("From B")).unwrap();
        assert_eq!(account_names(&b), vec!["From A", "From B"]);
    }

    #[test]
    fn moving_the_data_folder_copies_the_data_and_keeps_the_old_file() {
        let local = temp_path("move-local");
        let local_dir = local.parent().unwrap().to_path_buf();
        let state = AppState::load(local.clone());
        state.apply(add_account("Kept")).unwrap();
        let shared = local_dir.join("shared-folder");

        let view = state.set_data_folder(Some(shared.clone()), false).unwrap();

        assert_eq!(view.portfolio.accounts.len(), 1);
        assert!(!view.data.is_local);
        assert!(shared.join(DATA_FILE).is_file());
        assert!(local.is_file(), "the old file stays as a backup");
        assert_eq!(data_folder::resolve(&local_dir), shared);
        // Edits now go to the new folder only.
        state.apply(add_account("New place")).unwrap();
        assert_eq!(store::peek_revision(&shared.join(DATA_FILE)), Some(2));
        assert_eq!(store::peek_revision(&local), Some(1));
    }

    #[test]
    fn a_folder_that_already_has_data_needs_confirmation_and_is_then_used_as_it_is() {
        let local = temp_path("existing-local");
        let local_dir = local.parent().unwrap().to_path_buf();
        let state = AppState::load(local);
        state.apply(add_account("Local")).unwrap();
        let shared = local_dir.join("shared-with-data");
        let other = AppState::load(shared.join(DATA_FILE));
        std::fs::create_dir_all(&shared).unwrap();
        other.apply(add_account("Other computer")).unwrap();

        let error = state
            .set_data_folder(Some(shared.clone()), false)
            .unwrap_err();
        assert_eq!(error, FOLDER_HAS_DATA);
        assert_eq!(account_names(&state), vec!["Local"], "nothing changed");

        state.set_data_folder(Some(shared), true).unwrap();
        assert_eq!(account_names(&state), vec!["Other computer"]);
    }

    #[test]
    fn going_back_to_the_local_folder_and_bad_folders() {
        let local = temp_path("back-local");
        let local_dir = local.parent().unwrap().to_path_buf();
        let state = AppState::load(local);
        let shared = local_dir.join("shared-back");
        state.set_data_folder(Some(shared), false).unwrap();

        let view = state.set_data_folder(None, true).unwrap();

        assert!(view.data.is_local);
        assert_eq!(data_folder::resolve(&local_dir), local_dir);
        assert!(
            state
                .set_data_folder(Some(PathBuf::from("relative")), false)
                .is_err()
        );
    }

    #[test]
    fn conflict_copies_in_the_folder_are_reported() {
        let path = temp_path("conflict");
        let state = AppState::load(path.clone());
        state.apply(add_account("A")).unwrap();
        std::fs::write(path.with_file_name("portfolio 2.json"), b"{}").unwrap();

        let view = state.view().unwrap();

        assert_eq!(view.data.notes.conflict_copies, vec!["portfolio 2.json"]);
    }
}
