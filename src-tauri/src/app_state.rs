//! The portfolio held in memory together with the file it is saved to.
//!
//! This is separate from the Tauri commands so it can be tested without Tauri.

use std::path::PathBuf;
use std::sync::Mutex;

use portfolio::{Edit, Issue, Portfolio};
use serde::Serialize;
use suggest::{Overview, Request, Suggestion};

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
}

impl AppState {
    pub fn load(path: PathBuf) -> Self {
        let portfolio = Mutex::new(store::load(&path));
        Self { path, portfolio }
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
}
