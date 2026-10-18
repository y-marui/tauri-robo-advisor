//! The portfolio held in memory together with the file it is saved to.
//!
//! This is separate from the Tauri commands so it can be tested without Tauri.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;

use portfolio::{
    AccountId, Edit, HoldingInput, ImportMapping, ImportPlan, ImportRow, Issue, Portfolio,
    ProductId, Recipe,
};
use rust_decimal::Decimal;
use serde::Serialize;
use suggest::{AccountBalance, Overview, Request, Suggestion};
use tabular::{ColumnMapping, Skipped, Table};

use crate::browser_session::{LinkChoiceView, RecipeRead};

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
    /// The rows each recipe read, until they are applied or dropped.
    runs: Mutex<Vec<StoredRun>>,
    /// The browser opened for a fetch, and the tables read from it.
    pub browser: crate::browser_session::BrowserSession,
}

/// The rows a recipe read, kept until the user applies them. The plan is made
/// again from these rows each time, so it follows the aliases as they are then.
struct StoredRun {
    /// The recipe as it was when it ran. A run whose recipe has since been changed
    /// or removed no longer says what to do and is dropped.
    recipe: Recipe,
    name: String,
    account: AccountId,
    source: String,
    currency: String,
    rows: Vec<ImportRow>,
    skipped: Vec<Skipped>,
}

/// One recipe's result for the screen.
#[derive(Debug, Serialize)]
pub struct RecipeRunView {
    pub name: String,
    pub account: AccountId,
    pub source: String,
    pub currency: String,
    pub plan: ImportPlan,
    pub skipped: Vec<Skipped>,
    pub rows_read: usize,
}

/// What running a recipe gave.
#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum RecipeOutcome {
    Ready {
        run: RecipeRunView,
    },
    /// Several different links have the link's text: the user picks one.
    NeedsChoice {
        link: usize,
        choices: Vec<LinkChoiceView>,
    },
    /// A saved link is gone: the page has changed.
    LinkChanged {
        link: usize,
    },
}

fn column_mapping(mapping: &ImportMapping) -> ColumnMapping {
    ColumnMapping {
        name: mapping.name_column,
        value: mapping.value_column,
        price: mapping.price_column,
        price_unit: mapping.price_unit,
        header_rows: mapping.header_rows,
        rows_per_record: mapping.rows_per_record,
        name_row: mapping.name_row,
        value_row: mapping.value_row,
    }
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
            runs: Mutex::new(Vec::new()),
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

    fn recipe(&self, name: &str) -> Result<Recipe, String> {
        let guard = self.lock()?;
        let portfolio = guard.as_ref().map_err(Clone::clone)?;
        portfolio
            .recipes
            .iter()
            .find(|r| r.name == name)
            .cloned()
            .ok_or_else(|| "That recipe does not exist.".to_owned())
    }

    /// Follows the recipe's links in the browser, reads its table, and keeps the
    /// rows for `fetch_all_plan` / `apply_fetch_all`. A link that was followed
    /// without a saved reference gets one now, so the same link is found next time.
    pub fn run_recipe(&self, name: &str, page: &str) -> Result<RecipeOutcome, String> {
        let mut recipe = self.recipe(name)?;
        // An earlier result of this recipe must never survive a run that fails or needs
        // attention: it would be applied together with the fresh ones.
        self.lock_runs()?.retain(|r| r.name != name);
        let read = self
            .browser
            .run_recipe(page, &recipe.links, &recipe.table_starts_with)?;
        let (table, references) = match read {
            RecipeRead::NeedsChoice { link, choices } => {
                return Ok(RecipeOutcome::NeedsChoice { link, choices });
            }
            RecipeRead::LinkChanged { link } => return Ok(RecipeOutcome::LinkChanged { link }),
            RecipeRead::Table { table, references } => (table, references),
        };

        // Only fill in the references if the recipe is still the one that ran: an edit made
        // while the page was being read must not be put back the way it was.
        let unchanged = self.recipe(name).ok().as_ref() == Some(&recipe);
        if unchanged && recipe.links.iter().any(|l| l.reference.is_none()) {
            let mut learned = recipe.clone();
            for (link, reference) in learned.links.iter_mut().zip(references) {
                link.reference.get_or_insert(reference);
            }
            // Best effort: the run itself does not depend on it being saved.
            if self
                .apply(Edit::SetRecipe {
                    recipe: learned.clone(),
                })
                .is_ok()
            {
                recipe = learned;
            }
        }

        let extraction = tabular::extract(&table.rows, &column_mapping(&recipe.mapping));
        let rows: Vec<ImportRow> = extraction
            .rows
            .iter()
            .map(|r| ImportRow {
                name: r.name.clone(),
                value: r.value,
            })
            .collect();
        let run = StoredRun {
            recipe: recipe.clone(),
            name: recipe.name.clone(),
            account: recipe.account,
            source: recipe.mapping.source.clone(),
            currency: recipe.currency.clone(),
            rows,
            skipped: extraction.skipped,
        };
        let view = self.run_view(&run)?;
        let mut runs = self.lock_runs()?;
        runs.retain(|r| r.name != run.name);
        runs.push(run);
        Ok(RecipeOutcome::Ready { run: view })
    }

    fn run_view(&self, run: &StoredRun) -> Result<RecipeRunView, String> {
        Ok(RecipeRunView {
            name: run.name.clone(),
            account: run.account,
            source: run.source.clone(),
            currency: run.currency.clone(),
            plan: self.plan_import(&run.source, &run.currency, &run.rows)?,
            skipped: run.skipped.clone(),
            rows_read: run.rows.len(),
        })
    }

    /// The kept runs, planned again with the aliases as they are now.
    pub fn fetch_all_plan(&self) -> Result<Vec<RecipeRunView>, String> {
        let mut runs = self.lock_runs()?;
        // A recipe that was changed or removed after its run must not be applied.
        let current = {
            let guard = self.lock()?;
            guard
                .as_ref()
                .map(|p| p.recipes.clone())
                .unwrap_or_default()
        };
        runs.retain(|run| current.iter().any(|r| r == &run.recipe));
        runs.iter().map(|run| self.run_view(run)).collect()
    }

    pub fn clear_runs(&self) -> Result<(), String> {
        self.lock_runs()?.clear();
        Ok(())
    }

    /// Replaces the holdings of each account with the linked rows of all the
    /// recipes that feed it (products that appear twice are added up). An account
    /// with no linked row is left alone, so an unlinked first run never empties it.
    pub fn apply_fetch_all(&self) -> Result<PortfolioView, String> {
        let views = self.fetch_all_plan()?;
        // An account is replaced by what ALL of its recipes read. If one of them has no
        // result (it failed, needs a choice, or was added after the others ran), replacing
        // would delete the holdings that recipe was meant to supply.
        let (snapshot, missing): (Portfolio, Vec<String>) = {
            let guard = self.lock()?;
            let portfolio = guard.as_ref().map_err(Clone::clone)?;
            let missing = portfolio
                .recipes
                .iter()
                .filter(|r| views.iter().any(|v| v.account == r.account))
                .filter(|r| !views.iter().any(|v| v.name == r.name))
                .map(|r| r.name.clone())
                .collect();
            (portfolio.clone(), missing)
        };
        if !missing.is_empty() {
            return Err(format!(
                "These recipes have no result yet: {}. Run them, or remove them, before applying.",
                missing.join(", ")
            ));
        }
        let mut per_account: BTreeMap<AccountId, BTreeMap<ProductId, Decimal>> = BTreeMap::new();
        for view in &views {
            let products = per_account.entry(view.account).or_default();
            for row in &view.plan.matched {
                *products.entry(row.product).or_default() += row.value;
            }
        }
        // The plan was made from `snapshot`. Everything is checked and replaced under one
        // lock and saved once, so an edit made meanwhile (a recipe, an alias, a product)
        // cannot let stale rows through, and no account is replaced without the others.
        {
            let mut guard = self.lock()?;
            let current = guard.as_ref().map_err(Clone::clone)?;
            if *current != snapshot {
                return Err(
                    "The data changed while the result was waiting. Fetch again.".to_owned(),
                );
            }
            let mut next = current.clone();
            for (account, products) in per_account {
                if products.is_empty() {
                    continue;
                }
                next.apply(Edit::ReplaceHoldings {
                    account,
                    holdings: products
                        .into_iter()
                        .map(|(product, value)| HoldingInput { product, value })
                        .collect(),
                })
                .map_err(|e| e.to_string())?;
            }
            store::save(&self.path, &next)?;
            *guard = Ok(next);
        }
        self.clear_runs()?;
        self.view()
    }

    fn lock_runs(&self) -> Result<std::sync::MutexGuard<'_, Vec<StoredRun>>, String> {
        self.runs
            .lock()
            .map_err(|_| "The data is temporarily unavailable.".to_owned())
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

    // ---- recipe runs ------------------------------------------------------------

    /// A portfolio with one JPY category, two products linked (or not) by alias to
    /// source "SBI", and two accounts.
    fn recipe_state(label: &str) -> (AppState, AccountId, AccountId, ProductId, ProductId) {
        let state = AppState::load(temp_path(label));
        state
            .apply(Edit::AddCategory {
                name: "All".to_owned(),
                target_weight: rust_decimal::Decimal::ONE,
            })
            .unwrap();
        let category = state.view().unwrap().portfolio.categories[0].id;
        for name in ["Stock", "Fund"] {
            state
                .apply(Edit::AddProduct {
                    name: name.to_owned(),
                    category,
                    currency: "JPY".to_owned(),
                })
                .unwrap();
        }
        for name in ["Main", "Other"] {
            state.apply(add_account(name)).unwrap();
        }
        let view = state.view().unwrap();
        let (a, b) = (view.portfolio.accounts[0].id, view.portfolio.accounts[1].id);
        let (stock, fund) = (view.portfolio.products[0].id, view.portfolio.products[1].id);
        (state, a, b, stock, fund)
    }

    fn link(state: &AppState, product: ProductId, name: &str) {
        state
            .apply(Edit::AddAlias {
                product,
                source: "SBI".to_owned(),
                name: name.to_owned(),
            })
            .unwrap();
    }

    fn stored(name: &str, account: AccountId, rows: &[(&str, i64)]) -> StoredRun {
        StoredRun {
            recipe: Recipe {
                name: name.to_owned(),
                ..domestic_recipe(account)
            },
            name: name.to_owned(),
            account,
            source: "SBI".to_owned(),
            currency: "JPY".to_owned(),
            rows: rows
                .iter()
                .map(|(n, v)| ImportRow {
                    name: (*n).to_owned(),
                    value: rust_decimal::Decimal::from(*v),
                })
                .collect(),
            skipped: Vec::new(),
        }
    }

    fn held(state: &AppState, account: AccountId) -> Vec<(ProductId, rust_decimal::Decimal)> {
        state
            .view()
            .unwrap()
            .portfolio
            .holdings
            .iter()
            .filter(|h| h.account == account)
            .map(|h| (h.product, h.value))
            .collect()
    }

    #[test]
    fn recipes_that_feed_one_account_are_added_up_before_it_is_replaced() {
        let (state, main, _, stock, fund) = recipe_state("merge");
        link(&state, stock, "Stock A");
        link(&state, stock, "Stock B");
        link(&state, fund, "Fund X");
        state
            .apply(Edit::SetHolding {
                account: main,
                product: fund,
                value: rust_decimal::Decimal::from(1),
                quantity: None,
            })
            .unwrap();
        save_recipe(&state, "Domestic", main);
        save_recipe(&state, "Other stocks", main);
        *state.runs.lock().unwrap() = vec![
            stored("Domestic", main, &[("Stock A", 100), ("Unknown", 5)]),
            stored("Other stocks", main, &[("Stock B", 50), ("Fund X", 7)]),
        ];

        let view = state.apply_fetch_all().unwrap();

        let mut rows = held(&state, main);
        rows.sort();
        assert_eq!(
            rows,
            vec![
                (stock, rust_decimal::Decimal::from(150)),
                (fund, rust_decimal::Decimal::from(7))
            ]
        );
        assert_eq!(view.portfolio.holdings.len(), 2);
        assert!(
            state.runs.lock().unwrap().is_empty(),
            "the runs are consumed"
        );
    }

    #[test]
    fn an_account_with_no_linked_row_is_left_alone() {
        let (state, main, other, stock, _) = recipe_state("untouched");
        state
            .apply(Edit::SetHolding {
                account: other,
                product: stock,
                value: rust_decimal::Decimal::from(42),
                quantity: None,
            })
            .unwrap();
        // Nothing is linked yet, as on a first run.
        save_recipe(&state, "Other", other);
        save_recipe(&state, "Main", main);
        *state.runs.lock().unwrap() = vec![
            stored("Other", other, &[("New name", 1)]),
            stored("Main", main, &[]),
        ];

        state.apply_fetch_all().unwrap();

        assert_eq!(
            held(&state, other),
            vec![(stock, rust_decimal::Decimal::from(42))]
        );
        assert!(held(&state, main).is_empty());
    }

    #[test]
    fn the_plan_follows_the_aliases_as_they_are_when_it_is_asked_for() {
        let (state, main, _, stock, _) = recipe_state("replan");
        save_recipe(&state, "Domestic", main);
        *state.runs.lock().unwrap() = vec![stored("Domestic", main, &[("Stock A", 100)])];
        assert_eq!(state.fetch_all_plan().unwrap()[0].plan.unmatched.len(), 1);

        link(&state, stock, "Stock A");

        let plan = state.fetch_all_plan().unwrap();
        assert!(plan[0].plan.unmatched.is_empty());
        assert_eq!(plan[0].plan.matched[0].product, stock);
        assert_eq!(plan[0].rows_read, 1);
    }

    #[test]
    fn a_failed_run_does_not_leave_an_old_result_of_the_same_recipe_behind() {
        let (state, main, _, _, _) = recipe_state("stale");
        state
            .apply(Edit::SetRecipe {
                recipe: Recipe {
                    name: "Domestic".to_owned(),
                    links: Vec::new(),
                    table_starts_with: "Stocks".to_owned(),
                    currency: "JPY".to_owned(),
                    account: main,
                    mapping: ImportMapping {
                        source: "SBI".to_owned(),
                        name_column: 0,
                        value_column: 1,
                        price_column: None,
                        price_unit: 1,
                        header_rows: 0,
                        rows_per_record: 1,
                        name_row: 0,
                        value_row: 0,
                    },
                },
            })
            .unwrap();
        *state.runs.lock().unwrap() = vec![
            stored("Domestic", main, &[("Old", 1)]),
            stored("Other", main, &[("Keep", 2)]),
        ];

        // No browser is open, so this run fails after the old result is dropped.
        assert!(state.run_recipe("Domestic", "page").is_err());

        let names: Vec<String> = state
            .runs
            .lock()
            .unwrap()
            .iter()
            .map(|r| r.name.clone())
            .collect();
        assert_eq!(names, vec!["Other"]);
    }

    #[test]
    fn running_an_unknown_recipe_or_without_a_browser_is_an_error() {
        let (state, ..) = recipe_state("norecipe");

        assert!(state.run_recipe("missing", "page").is_err());
    }

    /// Saves a recipe of this name for the account, as a run needs one to stay valid.
    fn save_recipe(state: &AppState, name: &str, account: AccountId) {
        state
            .apply(Edit::SetRecipe {
                recipe: Recipe {
                    name: name.to_owned(),
                    ..domestic_recipe(account)
                },
            })
            .unwrap();
    }

    fn domestic_recipe(account: AccountId) -> Recipe {
        Recipe {
            name: "Domestic".to_owned(),
            links: Vec::new(),
            table_starts_with: "Stocks".to_owned(),
            currency: "JPY".to_owned(),
            account,
            mapping: ImportMapping {
                source: "SBI".to_owned(),
                name_column: 0,
                value_column: 1,
                price_column: None,
                price_unit: 1,
                header_rows: 0,
                rows_per_record: 1,
                name_row: 0,
                value_row: 0,
            },
        }
    }

    #[test]
    fn a_run_is_dropped_when_its_recipe_is_changed_or_removed() {
        let (state, main, other, _, _) = recipe_state("changed");
        let recipe = domestic_recipe(main);
        state
            .apply(Edit::SetRecipe {
                recipe: recipe.clone(),
            })
            .unwrap();
        let keep = |state: &AppState| {
            *state.runs.lock().unwrap() = vec![StoredRun {
                recipe: recipe.clone(),
                ..stored("Domestic", main, &[("A", 1)])
            }];
        };

        keep(&state);
        assert_eq!(
            state.fetch_all_plan().unwrap().len(),
            1,
            "unchanged: still there"
        );

        let mut moved = recipe.clone();
        moved.account = other;
        state.apply(Edit::SetRecipe { recipe: moved }).unwrap();
        keep(&state);
        assert!(
            state.fetch_all_plan().unwrap().is_empty(),
            "retargeted to another account"
        );

        state
            .apply(Edit::SetRecipe {
                recipe: recipe.clone(),
            })
            .unwrap();
        keep(&state);
        state
            .apply(Edit::RemoveRecipe {
                name: "Domestic".to_owned(),
            })
            .unwrap();
        assert!(state.fetch_all_plan().unwrap().is_empty(), "removed");
        assert!(
            state
                .apply_fetch_all()
                .unwrap()
                .portfolio
                .holdings
                .is_empty()
        );
    }

    #[test]
    fn nothing_is_replaced_while_a_recipe_of_the_account_has_no_result() {
        let (state, main, _, stock, _) = recipe_state("missing");
        link(&state, stock, "Stock A");
        save_recipe(&state, "Domestic", main);
        save_recipe(&state, "Funds", main);
        state
            .apply(Edit::SetHolding {
                account: main,
                product: stock,
                value: rust_decimal::Decimal::from(9),
                quantity: None,
            })
            .unwrap();
        // Only one of the two recipes of the account has run.
        *state.runs.lock().unwrap() = vec![stored("Domestic", main, &[("Stock A", 100)])];

        let error = state.apply_fetch_all().unwrap_err();

        assert!(error.contains("Funds"));
        assert_eq!(
            held(&state, main),
            vec![(stock, rust_decimal::Decimal::from(9))],
            "untouched"
        );
    }
}
