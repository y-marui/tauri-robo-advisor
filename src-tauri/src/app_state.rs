//! The portfolio held in memory together with the file it is saved to.
//!
//! This is separate from the Tauri commands so it can be tested without Tauri.

use std::path::PathBuf;
use std::sync::Mutex;

use portfolio::{Edit, ImportPlan, ImportRow, Issue, Portfolio};
use serde::Serialize;
use suggest::{AccountBalance, Overview, Request, Suggestion};
use tabular::{ColumnMapping, Skipped, Table};

use crate::store;

/// The portfolio plus the problems that block suggestions.
#[derive(Debug, Serialize)]
pub struct PortfolioView {
    pub portfolio: Portfolio,
    pub issues: Vec<Issue>,
}

impl PortfolioView {
    fn new(portfolio: &Portfolio) -> Self {
        Self {
            portfolio: portfolio.clone(),
            issues: portfolio.issues(),
        }
    }
}

pub struct AppState {
    path: PathBuf,
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
    pub fn load(path: PathBuf) -> Self {
        let portfolio = Mutex::new(store::load(&path));
        Self {
            path,
            portfolio,
            table: Mutex::new(None),
            browser: crate::browser_session::BrowserSession::default(),
        }
    }

    pub fn view(&self) -> Result<PortfolioView, String> {
        let guard = self.lock()?;
        guard.as_ref().map(PortfolioView::new).map_err(Clone::clone)
    }

    /// Applies one edit and saves it. The change takes effect in memory only
    /// after the file was written, so memory and disk never disagree.
    pub fn apply(&self, edit: Edit) -> Result<PortfolioView, String> {
        let mut guard = self.lock()?;
        let mut next = guard.as_ref().map_err(Clone::clone)?.clone();
        next.apply(edit).map_err(|e| e.to_string())?;
        store::save(&self.path, &next)?;
        let view = PortfolioView::new(&next);
        *guard = Ok(next);
        Ok(view)
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

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Result<Portfolio, String>>, String> {
        self.portfolio
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
}
