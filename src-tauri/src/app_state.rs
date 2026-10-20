//! The portfolio held in memory together with the file it is saved to.
//!
//! This is separate from the Tauri commands so it can be tested without Tauri.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::SystemTime;

use portfolio::{
    AccountId, Edit, HoldingInput, ImportMapping, ImportPlan, ImportRow, Issue, Portfolio,
    ProductId, Recipe, ShownValue,
};
use serde::Serialize;
use suggest::{AccountBalance, Overview, Request, Suggestion};
use tabular::{ColumnMapping, Skipped, Table};

use crate::browser_session::{LinkChoiceView, ReadSource, RecipeRead};
use crate::data_folder::{self, DATA_FILE, SyncNotes};
use crate::store;

/// Where the data lives and what the folder shows, for the settings card.
#[derive(Debug, Serialize)]
pub struct DataInfo {
    /// The folder with the data file.
    pub folder: String,
    /// `true` when it is the app's own local folder (no folder was chosen).
    pub is_local: bool,
    /// Counts the changes of the data location. An edit carries the value its screen was
    /// drawn from, and is refused if the location changed since.
    pub location: u64,
    #[serde(flatten)]
    pub notes: SyncNotes,
    /// Why a newer file on disk could not be used (never its contents).
    pub warning: Option<String>,
}

/// The portfolio plus the problems that block suggestions.
#[derive(Debug, Serialize)]
pub struct PortfolioView {
    pub portfolio: Portfolio,
    /// Each holding's value rounded to the smallest unit of its currency, for display only
    /// (`portfolio` keeps the exact values, which the edit fields show).
    pub shown_values: Vec<ShownValue>,
    /// The recipes as they are run: a recipe made from a preset with the definition of the preset
    /// the app has now (the stored one only refers to it).
    pub recipes: Vec<RecipeView>,
    pub issues: Vec<Issue>,
    pub data: DataInfo,
}

/// One recipe for the screen: its effective definition. When the preset it refers to does not
/// exist in this version of the app, `preset_missing` is set and the definition is empty (the
/// stored recipe is kept untouched; it can be removed).
#[derive(Debug, Clone, Serialize)]
pub struct RecipeView {
    #[serde(flatten)]
    pub recipe: Recipe,
    pub preset_missing: bool,
}

impl RecipeView {
    fn new(stored: &Recipe) -> Self {
        match stored.effective() {
            Ok(recipe) => Self {
                recipe,
                preset_missing: false,
            },
            Err(_) => Self {
                recipe: stored.clone(),
                preset_missing: true,
            },
        }
    }
}

impl PortfolioView {
    fn new(portfolio: &Portfolio, data: DataInfo) -> Self {
        Self {
            portfolio: portfolio.clone(),
            shown_values: portfolio.shown_values(),
            recipes: portfolio.recipes.iter().map(RecipeView::new).collect(),
            issues: portfolio.issues(),
            data,
        }
    }
}

type Fingerprint = (SystemTime, u64);

/// The data file on disk, and what was last seen of it.
struct Disk {
    path: PathBuf,
    /// Bumped each time `path` changes (the data folder was switched).
    location: u64,
    seen: Option<Fingerprint>,
    /// A hash of the file's content as last read or written by this app.
    hash: Option<u64>,
    warning: Option<String>,
}

/// The message when another computer saved first; the frontend shows it as is.
const CHANGED_ELSEWHERE: &str =
    "The data file was changed on another computer. It has been reloaded; please try again.";
/// The message when the data folder was switched after the screen was drawn.
const LOCATION_CHANGED: &str =
    "The data folder was changed after this screen was shown. Nothing was saved; please try again.";
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
        disk.hash = None;
        disk.warning = None; // the bad file is gone
        return;
    }
    match store::load_hashed(&disk.path) {
        Ok((on_disk, hash)) => {
            disk.hash = hash;
            disk.warning = None;
            match portfolio {
                Ok(current) if on_disk.revision > current.revision => *portfolio = Ok(on_disk),
                // Two computers saved the same revision before their files were exchanged:
                // the file that arrived is taken (what is in memory was already saved by
                // this computer, so nothing is unsaved), and the user is told.
                Ok(current) if on_disk.revision == current.revision && on_disk != *current => {
                    *portfolio = Ok(on_disk);
                    disk.warning = Some(
                        "two computers saved at the same time; the other version was loaded, so please check your latest change"
                            .to_owned(),
                    );
                }
                Ok(_) => {}
                Err(_) => *portfolio = Ok(on_disk),
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
    /// The rows each recipe read, until they are applied or dropped.
    runs: Mutex<Vec<StoredRun>>,
    /// Counts every change of the kept runs, so an apply can tell that the runs are no
    /// longer the ones that were shown.
    runs_generation: AtomicU64,
    /// What the user was last shown: the portfolio the plan was made from, and the runs
    /// generation. An apply is accepted only for exactly this.
    shown: Mutex<Option<(Portfolio, u64)>>,
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
        name_part: mapping.name_part,
        value_part: mapping.value_part,
        price_part: mapping.price_part,
        loose_numbers: mapping.loose_numbers,
        quantity: mapping.quantity_column,
        quantity_part: mapping.quantity_part,
        max_records: mapping.max_records,
        kind_column: mapping.kind_column,
        kind_rules: mapping
            .kind_rules
            .iter()
            .map(|rule| tabular::KindRule {
                kind: rule.kind.clone(),
                read: match rule.read {
                    portfolio::KindRead::QuantityTimesPrice { price_unit } => {
                        tabular::KindRead::QuantityTimesPrice { price_unit }
                    }
                    portfolio::KindRead::Evaluation { column, part } => {
                        tabular::KindRead::Evaluation { column, part }
                    }
                },
            })
            .collect(),
    }
}

/// The rows of an extraction as import rows. The price unit comes with a price only.
fn import_rows(extraction: &tabular::Extraction) -> Vec<ImportRow> {
    extraction
        .rows
        .iter()
        .map(|r| ImportRow {
            name: r.name.clone(),
            value: r.value,
            quantity: r.quantity,
            price: r.price,
            price_unit: r.price_unit,
        })
        .collect()
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
        let seen = store::fingerprint(&path);
        let (loaded, hash) = match store::load_hashed(&path) {
            Ok((portfolio, hash)) => (Ok(portfolio), hash),
            Err(error) => (Err(error), None),
        };
        let portfolio = Mutex::new(loaded);
        Self {
            app_data,
            disk: Mutex::new(Disk {
                path,
                location: 0,
                seen,
                hash,
                warning: None,
            }),
            portfolio,
            table: Mutex::new(None),
            runs: Mutex::new(Vec::new()),
            runs_generation: AtomicU64::new(0),
            shown: Mutex::new(None),
            browser: crate::browser_session::BrowserSession::default(),
        }
    }

    fn data_info(&self, disk: &Disk) -> DataInfo {
        let folder = disk.path.parent().map(PathBuf::from).unwrap_or_default();
        DataInfo {
            is_local: folder == self.app_data,
            location: disk.location,
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
    #[cfg(test)]
    pub fn apply(&self, edit: Edit) -> Result<PortfolioView, String> {
        let mut guard = self.lock()?;
        let mut disk = self.lock_disk()?;
        let mut next = guard.as_ref().map_err(Clone::clone)?.clone();
        next.apply(edit).map_err(|e| e.to_string())?;
        self.commit(&mut guard, &mut disk, next)
    }

    /// Like `apply`, for an edit sent from a screen drawn at data location `location` (the
    /// `data.location` of the view it showed). If the data folder was switched since, the
    /// edit refers to ids of other data and is refused without being applied.
    pub fn apply_at(&self, edit: Edit, location: u64) -> Result<PortfolioView, String> {
        let mut guard = self.lock()?;
        let mut disk = self.lock_disk()?;
        if disk.location != location {
            return Err(LOCATION_CHANGED.to_owned());
        }
        let mut next = guard.as_ref().map_err(Clone::clone)?.clone();
        next.apply(edit).map_err(|e| e.to_string())?;
        self.commit(&mut guard, &mut disk, next)
    }

    /// Makes `next` the state: refused if the file on disk has a newer revision than the
    /// state `next` was made from (another computer saved first); otherwise written, and
    /// only then taken over in memory. Every write of the portfolio goes through here.
    fn commit(
        &self,
        guard: &mut Result<Portfolio, String>,
        disk: &mut Disk,
        next: Portfolio,
    ) -> Result<PortfolioView, String> {
        let base_revision = guard.as_ref().map_err(Clone::clone)?.revision;
        // The check and the replacement happen under a lock on the folder, so two instances
        // cannot both pass the check and then overwrite each other.
        // A data file that exists only as an iCloud placeholder is not downloaded yet: creating
        // a new file next to it could clash with the real one once it arrives.
        if !disk.path.exists()
            && disk
                .path
                .parent()
                .is_some_and(data_folder::is_waiting_for_download)
        {
            return Err("The data file is still being downloaded from the synced folder. Wait a moment, then try again.".to_owned());
        }
        let Some(written) = store::save_unless_newer(&disk.path, &next, base_revision, disk.hash)?
        else {
            disk.seen = None; // read the newer file on the next look
            return Err(CHANGED_ELSEWHERE.to_owned());
        };
        disk.seen = store::fingerprint(&disk.path);
        // The hash of what was written: if a sync client replaced the file after the write, the
        // next save sees that the content is not this one and is refused.
        disk.hash = Some(written);
        disk.warning = None;
        let view = PortfolioView::new(&next, self.data_info(disk));
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
        let mut switched = false;
        if target != disk.path {
            // The data in memory is replaced only after the pointer is written, so a failure
            // leaves the old file and the old data exactly as they were.
            if store::fingerprint(&target).is_none()
                && data_folder::is_waiting_for_download(&target_dir)
            {
                return Err("The data file in that folder is still being downloaded. Wait a moment and try again.".to_owned());
            }
            // The fingerprint is taken before the file is read, so a change made after the read
            // shows up as a change on the next look.
            let seen_before = store::fingerprint(&target);
            let mut created = None;
            let adopted = if seen_before.is_some() {
                if !use_existing {
                    return Err(FOLDER_HAS_DATA.to_owned());
                }
                Some(store::load_hashed(&target)?)
            } else {
                // Created only if still absent (checked under the lock): a file that arrived
                // meanwhile is not replaced without the confirmation.
                let Some(written) =
                    store::save_if_absent(&target, guard.as_ref().map_err(Clone::clone)?)?
                else {
                    return Err(FOLDER_HAS_DATA.to_owned());
                };
                created = Some(written);
                None
            };
            data_folder::write_pointer(&self.app_data, folder.as_deref())?;
            match adopted {
                Some((portfolio, hash)) => {
                    *guard = Ok(portfolio);
                    disk.seen = seen_before;
                    disk.hash = hash;
                }
                None => {
                    disk.seen = store::fingerprint(&target);
                    disk.hash = created;
                }
            }
            disk.path = target;
            disk.location += 1;
            disk.warning = None;
            switched = true;
        }
        let portfolio = guard.as_ref().map_err(Clone::clone)?;
        let view = PortfolioView::new(portfolio, self.data_info(&disk));
        drop(disk); // `run_recipe` takes the runs lock before the disk lock: never hold both here
        drop(guard);
        if switched {
            // Rows read for the old data must not be applied to the new data.
            self.lock_runs()?.clear();
            *lock_shown(&self.shown)? = None;
            self.runs_generation.fetch_add(1, Ordering::SeqCst);
        }
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
        let rows = import_rows(&extraction);
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
        let stored = portfolio
            .recipes
            .iter()
            .find(|r| r.name == name)
            .ok_or_else(|| "That recipe does not exist.".to_owned())?;
        stored.effective().map_err(|_| {
            "The preset of this recipe does not exist in this version of the app. Remove the recipe."
                .to_owned()
        })
    }

    /// Saves `new` in place of the recipe `expected` (an effective recipe) only if the stored recipe
    /// still stands for exactly `expected`; the check and the write are made under one lock, so an edit
    /// made meanwhile is never put back and a recipe removed meanwhile is never recreated.
    fn replace_recipe_if_unchanged(
        &self,
        expected: &Recipe,
        new: Recipe,
    ) -> Result<PortfolioView, String> {
        let mut guard = self.lock()?;
        let mut disk = self.lock_disk()?;
        let mut next = guard.as_ref().map_err(Clone::clone)?.clone();
        if !next
            .recipes
            .iter()
            .any(|r| r.effective().as_ref() == Ok(expected))
        {
            return Err(
                "The recipe was changed or removed in the meantime. Fetch again.".to_owned(),
            );
        }
        next.apply(Edit::SetRecipe { recipe: new })
            .map_err(|e| e.to_string())?;
        self.commit(&mut guard, &mut disk, next)
    }

    /// Sets (or clears) the saved reference of one link of a recipe, if the recipe is still
    /// the one the question was asked about.
    pub fn set_link_reference(
        &self,
        expected: &Recipe,
        link: usize,
        reference: Option<String>,
    ) -> Result<PortfolioView, String> {
        let mut new = expected.clone();
        new.links
            .get_mut(link)
            .ok_or_else(|| "That link does not exist.".to_owned())?
            .reference = reference;
        self.replace_recipe_if_unchanged(expected, new)
    }

    /// Opens the login page of every site that a recipe reads from (the person logs in by hand),
    /// unless a tab of that site is already open.
    pub fn open_login_pages(&self) -> Result<crate::browser_session::LoginPages, String> {
        let sites: Vec<String> = {
            let guard = self.lock()?;
            let portfolio = guard.as_ref().map_err(Clone::clone)?;
            let mut sites: Vec<String> = Vec::new();
            for recipe in portfolio.recipes.iter().filter_map(|r| r.effective().ok()) {
                if !recipe.start_site.is_empty() && !sites.contains(&recipe.start_site) {
                    sites.push(recipe.start_site.clone());
                }
            }
            sites
        };
        let mut known = Vec::new();
        let mut unknown = Vec::new();
        for site in sites {
            match portfolio::login_url_for(&site) {
                Some(url) => known.push((site, url.to_owned())),
                None => unknown.push(site),
            }
        }
        // Nothing to open: the browser is not needed, and the unknown sites are still reported.
        let mut result = if known.is_empty() {
            crate::browser_session::LoginPages::default()
        } else {
            self.browser.open_login_pages(&known)?
        };
        result.unknown = unknown;
        Ok(result)
    }

    /// Follows the recipe's links in the browser, reads its table, and keeps the
    /// rows for `fetch_all_plan` / `apply_fetch_all`. A link that was followed
    /// without a saved reference gets one now, so the same link is found next time.
    pub fn run_recipe(&self, name: &str, page: &str) -> Result<RecipeOutcome, String> {
        let location = self.lock_disk()?.location;
        let mut recipe = self.recipe(name)?;
        // An earlier result of this recipe must never survive a run that fails or needs
        // attention: it would be applied together with the fresh ones.
        {
            let mut runs = self.lock_runs()?;
            runs.retain(|r| r.name != name);
            self.runs_generation.fetch_add(1, Ordering::SeqCst);
        }
        let source = match &recipe.cards {
            Some(cards) => ReadSource::Cards {
                heading: &cards.heading,
                item_depth: cards.item_depth,
            },
            None => ReadSource::Table {
                title: &recipe.table_starts_with,
                merge: recipe.all_tables.then_some(recipe.mapping.header_rows),
                follow: recipe.following_tables,
            },
        };
        let read = self
            .browser
            .run_recipe(page, &recipe.links, source, &recipe.allowed_sites)?;
        let (table, references) = match read {
            RecipeRead::NeedsChoice { link, choices } => {
                return Ok(RecipeOutcome::NeedsChoice { link, choices });
            }
            RecipeRead::LinkChanged { link } => return Ok(RecipeOutcome::LinkChanged { link }),
            RecipeRead::Table { table, references } => (table, references),
        };

        // Only fill in the references if the recipe is still the one that ran: an edit made
        // while the page was being read must not be put back the way it was. The check and
        // the write happen under one lock.
        if recipe
            .links
            .iter()
            .any(|l| l.menu.is_none() && l.reference.is_none())
        {
            let learned = with_learned_references(&recipe, references);
            // Best effort: the run itself does not depend on it being saved.
            if self
                .replace_recipe_if_unchanged(&recipe, learned.clone())
                .is_ok()
            {
                recipe = learned;
            }
        }

        let extraction = tabular::extract(&table.rows, &column_mapping(&recipe.mapping));
        let rows = import_rows(&extraction);
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
        // The rows were read for the recipe of the data location the run started in: after the
        // data folder was switched they must not be kept for the new data.
        if self.lock_disk()?.location != location {
            return Err(LOCATION_CHANGED.to_owned());
        }
        runs.retain(|r| r.name != run.name);
        runs.push(run);
        self.runs_generation.fetch_add(1, Ordering::SeqCst);
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

    /// The kept runs, planned again with the aliases as they are now. What is returned is
    /// what `apply_fetch_all` will accept: it remembers the portfolio the plan was made
    /// from and the state of the runs.
    pub fn fetch_all_plan(&self) -> Result<Vec<RecipeRunView>, String> {
        // Taken before the plan is made, so an edit at any time after it is caught.
        let snapshot = self.portfolio_snapshot()?;
        let (views, generation) = self.plan_views()?;
        *lock_shown(&self.shown)? = Some((snapshot, generation));
        Ok(views)
    }

    fn portfolio_snapshot(&self) -> Result<Portfolio, String> {
        let guard = self.lock()?;
        Ok(guard.as_ref().map_err(Clone::clone)?.clone())
    }

    /// The planned runs, and the generation of the runs they were made from (read while
    /// the runs are locked, so it describes exactly these).
    fn plan_views(&self) -> Result<(Vec<RecipeRunView>, u64), String> {
        let mut runs = self.lock_runs()?;
        // A recipe that was changed or removed after its run must not be applied.
        let current = {
            let guard = self.lock()?;
            guard
                .as_ref()
                .map(|p| {
                    p.recipes
                        .iter()
                        .filter_map(|r| r.effective().ok())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        };
        let before = runs.len();
        runs.retain(|run| current.iter().any(|r| r == &run.recipe));
        if runs.len() != before {
            self.runs_generation.fetch_add(1, Ordering::SeqCst);
        }
        let views = runs
            .iter()
            .map(|run| self.run_view(run))
            .collect::<Result<Vec<_>, _>>()?;
        Ok((views, self.runs_generation.load(Ordering::SeqCst)))
    }

    /// Forgets the earlier result of one recipe (a retry that could not start must not leave it
    /// to be applied together with the fresh ones).
    pub fn discard_run(&self, name: &str) -> Result<(), String> {
        let mut runs = self.lock_runs()?;
        let before = runs.len();
        runs.retain(|r| r.name != name);
        if runs.len() != before {
            self.runs_generation.fetch_add(1, Ordering::SeqCst);
        }
        Ok(())
    }

    pub fn clear_runs(&self) -> Result<(), String> {
        {
            let mut runs = self.lock_runs()?;
            runs.clear();
            self.runs_generation.fetch_add(1, Ordering::SeqCst);
        }
        *lock_shown(&self.shown)? = None;
        self.browser.unpin();
        Ok(())
    }

    /// Replaces the holdings of each account with the linked rows of all the
    /// recipes that feed it (products that appear twice are added up). An account
    /// with no linked row is left alone, so an unlinked first run never empties it.
    pub fn apply_fetch_all(&self) -> Result<PortfolioView, String> {
        // Only what the user was shown can be applied: the portfolio the displayed plan was
        // made from, and the runs as they were then. Without a displayed plan (nothing was
        // shown yet) the state as of now is used.
        let (snapshot, generation) = match lock_shown(&self.shown)?.clone() {
            Some(shown) => shown,
            None => (
                self.portfolio_snapshot()?,
                self.runs_generation.load(Ordering::SeqCst),
            ),
        };
        if self.runs_generation.load(Ordering::SeqCst) != generation {
            return Err("The results changed while they were shown. Fetch again.".to_owned());
        }
        let (views, planned) = self.plan_views()?;
        if planned != generation {
            return Err("The results changed while they were shown. Fetch again.".to_owned());
        }
        // An account is replaced by what ALL of its recipes read. If one of them has no
        // result (it failed, needs a choice, or was added after the others ran), replacing
        // would delete the holdings that recipe was meant to supply.
        let missing: Vec<String> = {
            let portfolio = &snapshot;
            portfolio
                .recipes
                .iter()
                .filter(|r| views.iter().any(|v| v.account == r.account))
                .filter(|r| !views.iter().any(|v| v.name == r.name))
                .map(|r| r.name.clone())
                .collect()
        };
        if !missing.is_empty() {
            return Err(format!(
                "These recipes have no result yet: {}. Run them, or remove them, before applying.",
                missing.join(", ")
            ));
        }
        let mut per_account: BTreeMap<AccountId, BTreeMap<ProductId, HoldingInput>> =
            BTreeMap::new();
        for view in &views {
            let products = per_account.entry(view.account).or_default();
            for row in &view.plan.matched {
                match products.get_mut(&row.product) {
                    // Another recipe feeds the same product: the values and quantities are
                    // added up, and the price is kept only if it is the same.
                    Some(held) => {
                        held.value += row.value;
                        held.quantity = match (held.quantity, row.quantity) {
                            (Some(a), Some(b)) => a.checked_add(b),
                            _ => None,
                        };
                        if held.price != row.price || held.price_unit != row.price_unit {
                            held.price = None;
                            held.price_unit = 1;
                        }
                    }
                    None => {
                        products.insert(
                            row.product,
                            HoldingInput {
                                product: row.product,
                                value: row.value,
                                quantity: row.quantity,
                                price: row.price,
                                price_unit: row.price_unit,
                            },
                        );
                    }
                }
            }
        }
        // The plan was made from `snapshot`. Everything is checked and replaced under one
        // lock and saved once, so an edit made meanwhile (a recipe, an alias, a product)
        // cannot let stale rows through, and no account is replaced without the others.
        {
            // The runs are locked first (the same order as when planning) and stay locked until
            // the rows are consumed: a retry cannot replace a run between the check and the save.
            let mut runs = self.lock_runs()?;
            if self.runs_generation.load(Ordering::SeqCst) != generation {
                return Err("The results changed while they were shown. Fetch again.".to_owned());
            }
            let mut guard = self.lock()?;
            let mut disk = self.lock_disk()?;
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
                    holdings: products.into_values().collect(),
                })
                .map_err(|e| e.to_string())?;
            }
            self.commit(&mut guard, &mut disk, next)?;
            runs.clear();
            self.runs_generation.fetch_add(1, Ordering::SeqCst);
        }
        *lock_shown(&self.shown)? = None;
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

fn lock_shown(
    shown: &Mutex<Option<(Portfolio, u64)>>,
) -> Result<std::sync::MutexGuard<'_, Option<(Portfolio, u64)>>, String> {
    shown
        .lock()
        .map_err(|_| "The data is temporarily unavailable.".to_owned())
}

/// The recipe with the destination of each plain link filled in, the first time it is followed.
/// A menu entry that is clicked has no destination to remember, so it is left as it is (an empty
/// reference would make the saved recipe differ from the one that ran).
fn with_learned_references(recipe: &Recipe, references: Vec<String>) -> Recipe {
    let mut learned = recipe.clone();
    for (link, reference) in learned.links.iter_mut().zip(references) {
        if link.menu.is_none() {
            link.reference.get_or_insert(reference);
        }
    }
    learned
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
            name_part: None,
            value_part: None,
            price_part: None,
            loose_numbers: false,
            quantity: None,
            quantity_part: None,
            max_records: None,
            kind_column: None,
            kind_rules: Vec::new(),
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
    fn an_edit_from_before_a_folder_switch_is_refused() {
        let local = temp_path("stale-local");
        let local_dir = local.parent().unwrap().to_path_buf();
        let state = AppState::load(local);
        let before = state.view().unwrap().data.location;
        // Another folder with unrelated data (ids restart at 1 there).
        let shared = local_dir.join("shared-stale");
        let other = AppState::load(shared.join(DATA_FILE));
        std::fs::create_dir_all(&shared).unwrap();
        other.apply(add_account("Other computer")).unwrap();
        let after = state
            .set_data_folder(Some(shared), true)
            .unwrap()
            .data
            .location;
        assert_ne!(before, after);

        let error = state.apply_at(add_account("Stale"), before).unwrap_err();
        assert_eq!(error, LOCATION_CHANGED);
        assert_eq!(
            account_names(&state),
            vec!["Other computer"],
            "nothing applied"
        );

        state.apply_at(add_account("Fresh"), after).unwrap();
        assert_eq!(account_names(&state), vec!["Other computer", "Fresh"]);
    }

    #[test]
    fn switching_the_folder_drops_the_fetched_rows() {
        let local = temp_path("runs-local");
        let local_dir = local.parent().unwrap().to_path_buf();
        let state = AppState::load(local);
        state
            .lock_runs()
            .unwrap()
            .push(stored("r", AccountId(1), &[("A", 100)]));
        let before = state.runs_generation.load(Ordering::SeqCst);
        state
            .set_data_folder(Some(local_dir.join("shared-runs")), false)
            .unwrap();
        assert!(state.lock_runs().unwrap().is_empty());
        assert_ne!(state.runs_generation.load(Ordering::SeqCst), before);
    }

    #[test]
    fn the_location_changes_only_when_the_path_does() {
        let local = temp_path("loc-same");
        let local_dir = local.parent().unwrap().to_path_buf();
        let state = AppState::load(local);
        let start = state.view().unwrap().data.location;
        state.apply(add_account("A")).unwrap();
        assert_eq!(state.view().unwrap().data.location, start);
        let shared = local_dir.join("shared-loc");
        let moved = state.set_data_folder(Some(shared.clone()), false).unwrap();
        assert_eq!(moved.data.location, start + 1);
        let again = state.set_data_folder(Some(shared), false).unwrap();
        assert_eq!(again.data.location, start + 1, "same folder: no change");
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
                    quantity: None,
                    price: None,
                    price_unit: 1,
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
                    all_tables: false,
                    following_tables: false,
                    cards: None,
                    start_site: String::new(),
                    allowed_sites: Vec::new(),
                    currency: "JPY".to_owned(),
                    preset: None,
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
                        name_part: None,
                        value_part: None,
                        price_part: None,
                        loose_numbers: false,
                        quantity_column: None,
                        quantity_part: None,
                        max_records: None,
                        kind_column: None,
                        kind_rules: Vec::new(),
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
            all_tables: false,
            following_tables: false,
            cards: None,
            start_site: String::new(),
            allowed_sites: Vec::new(),
            currency: "JPY".to_owned(),
            preset: None,
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
                name_part: None,
                value_part: None,
                price_part: None,
                loose_numbers: false,
                quantity_column: None,
                quantity_part: None,
                max_records: None,
                kind_column: None,
                kind_rules: Vec::new(),
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

    #[test]
    fn a_link_reference_is_saved_only_if_the_recipe_is_still_the_one_asked_about() {
        use portfolio::FetchLink;
        let (state, main, _, _, _) = recipe_state("reference");
        let recipe = Recipe {
            links: vec![FetchLink {
                text: "List".to_owned(),
                reference: None,
                menu: None,
                heading: None,
            }],
            ..domestic_recipe(main)
        };
        state
            .apply(Edit::SetRecipe {
                recipe: recipe.clone(),
            })
            .unwrap();

        let view = state
            .set_link_reference(&recipe, 0, Some("h:0123456789abcdef".to_owned()))
            .unwrap();
        assert_eq!(
            view.portfolio.recipes[0].links[0].reference.as_deref(),
            Some("h:0123456789abcdef")
        );

        // The same question again: the recipe has changed since, so nothing is written.
        let error = state.set_link_reference(&recipe, 0, None).unwrap_err();
        assert!(error.contains("changed or removed"));
        assert!(state.set_link_reference(&recipe, 5, None).is_err());

        // A removed recipe is never brought back.
        let current = view.portfolio.recipes[0].clone();
        state
            .apply(Edit::RemoveRecipe {
                name: current.name.clone(),
            })
            .unwrap();
        assert!(state.set_link_reference(&current, 0, None).is_err());
        assert!(state.view().unwrap().portfolio.recipes.is_empty());
    }

    #[test]
    fn discarding_one_recipe_keeps_the_runs_of_the_others() {
        let (state, main, _, _, _) = recipe_state("discard");
        save_recipe(&state, "Domestic", main);
        save_recipe(&state, "Funds", main);
        *state.runs.lock().unwrap() = vec![
            stored("Domestic", main, &[("Stock A", 100)]),
            stored("Funds", main, &[("Fund B", 50)]),
        ];

        state.discard_run("Domestic").unwrap();

        let runs = state.runs.lock().unwrap();
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].name, "Funds");
        drop(runs);
        // Discarding a recipe that has no run is not an error.
        state.discard_run("Domestic").unwrap();
    }

    #[test]
    fn only_the_plan_that_was_shown_can_be_applied() {
        let (state, main, _, stock, _) = recipe_state("shown");
        link(&state, stock, "Stock A");
        save_recipe(&state, "Domestic", main);
        *state.runs.lock().unwrap() = vec![stored("Domestic", main, &[("Stock A", 100)])];
        state.fetch_all_plan().unwrap();

        // The runs change after the plan was shown (a retry finished): refused.
        state.runs_generation.fetch_add(1, Ordering::SeqCst);
        let error = state.apply_fetch_all().unwrap_err();
        assert!(error.contains("Fetch again"), "{error}");
        assert!(held(&state, main).is_empty(), "nothing was applied");

        // Shown again, then an alias is changed before the confirmation: refused.
        state.fetch_all_plan().unwrap();
        state
            .apply(Edit::RemoveAlias {
                product: stock,
                source: "SBI".to_owned(),
                name: "Stock A".to_owned(),
            })
            .unwrap();
        assert!(state.apply_fetch_all().is_err());
        assert!(held(&state, main).is_empty());
    }

    #[test]
    fn a_file_with_the_same_revision_but_other_content_is_taken_and_reported() {
        let path = temp_path("divergent");
        let a = AppState::load(path.clone());
        a.apply(add_account("From A")).unwrap();
        // Another computer saved its own revision 1, and that file replaces this one (a sync).
        let other = temp_path("divergent-other");
        let c = AppState::load(other.clone());
        c.apply(add_account("From C")).unwrap();
        std::fs::copy(&other, &path).unwrap();

        let view = a.view().unwrap();

        assert_eq!(account_names(&a), vec!["From C"]);
        assert!(view.data.warning.is_some());
    }

    #[test]
    fn quantities_and_prices_of_recipes_feeding_one_product_are_combined_when_applied() {
        let (state, main, _, stock, _) = recipe_state("qty");
        link(&state, stock, "Stock A");
        link(&state, stock, "Stock B");
        save_recipe(&state, "Domestic", main);
        save_recipe(&state, "Other stocks", main);
        let with = |name: &str, row: &str, quantity: i64, price: i64| {
            let mut run = stored(name, main, &[(row, quantity * price)]);
            run.rows[0].quantity = Some(rust_decimal::Decimal::from(quantity));
            run.rows[0].price = Some(rust_decimal::Decimal::from(price));
            run
        };
        *state.runs.lock().unwrap() = vec![
            with("Domestic", "Stock A", 2, 50),
            with("Other stocks", "Stock B", 3, 50),
        ];

        state.apply_fetch_all().unwrap();

        let held = state.view().unwrap().portfolio.holdings;
        assert_eq!(held.len(), 1);
        assert_eq!(held[0].value, rust_decimal::Decimal::from(250));
        assert_eq!(held[0].quantity, Some(rust_decimal::Decimal::from(5)));
        assert_eq!(held[0].price, Some(rust_decimal::Decimal::from(50)));
    }

    #[test]
    fn the_shipped_presets_are_valid_recipes_with_links_the_app_may_follow() {
        let (state, main, _, _, _) = recipe_state("presets");
        for preset in portfolio::presets() {
            state
                .apply(Edit::AddPresetRecipe {
                    preset: preset.id.to_owned(),
                    account: main,
                })
                .unwrap();
        }
        let recipes: Vec<Recipe> = state
            .view()
            .unwrap()
            .recipes
            .into_iter()
            .map(|r| r.recipe)
            .collect();
        assert_eq!(recipes.len(), portfolio::presets().len());
        for recipe in &recipes {
            assert!(recipe.all_tables || recipe.following_tables);
            for link in &recipe.links {
                assert_eq!(
                    browser::link_text_allowed(&link.text),
                    Ok(()),
                    "{}",
                    link.text
                );
            }
        }
        // The same preset again changes nothing; for another account the name carries the account.
        state
            .apply(Edit::AddPresetRecipe {
                preset: "sbi-funds".to_owned(),
                account: main,
            })
            .unwrap();
        assert_eq!(state.view().unwrap().portfolio.recipes.len(), recipes.len());
        let (_, other) = {
            let v = state.view().unwrap();
            (v.portfolio.accounts[0].id, v.portfolio.accounts[1].id)
        };
        state
            .apply(Edit::AddPresetRecipe {
                preset: "sbi-funds".to_owned(),
                account: other,
            })
            .unwrap();
        let after = state.view().unwrap().portfolio.recipes;
        assert_eq!(after.len(), recipes.len() + 1);
        assert!(after.iter().any(|r| r.name.contains("Other")));
        assert!(
            state
                .apply(Edit::AddPresetRecipe {
                    preset: "nope".to_owned(),
                    account: main,
                })
                .is_err()
        );
    }

    #[test]
    fn a_preset_reads_a_page_laid_out_like_the_real_one() {
        // A made-up page with the layout of the real tables: a title row, a heading row, then two
        // rows per holding (a name row, then quantity, cost, price, profit).
        let table = |title: &str,
                     headings: [&str; 4],
                     first: (&str, &str, &str),
                     second: (&str, &str, &str)| {
            let row = |cells: &[&str]| cells.iter().map(|c| (*c).to_owned()).collect::<Vec<_>>();
            vec![
                row(&[title, "", "", ""]),
                row(&headings),
                row(&[first.0, "", "Buttons", ""]),
                row(&[first.1, "100", first.2, "+5"]),
                row(&[second.0, "", "Buttons", ""]),
                row(&[second.1, "50", second.2, "+1"]),
            ]
        };
        let a = table(
            "Stocks A",
            ["Quantity", "Cost", "Price", "Profit"],
            ("1111 Alpha", "10", "12.50"),
            ("2222 Beta", "3", "100"),
        );
        let b = table(
            "Stocks B",
            ["Quantity", "Cost", "Price", "Profit"],
            ("3333 Gamma", "2", "1,000"),
            ("4444 Delta", "1", "7"),
        );
        let (state, main, _, _, _) = recipe_state("preset-page");
        state
            .apply(Edit::AddPresetRecipe {
                preset: "sbi-domestic-stocks".to_owned(),
                account: main,
            })
            .unwrap();
        let recipe = state.view().unwrap().recipes[0].recipe.clone();
        let mut rows = a.clone();
        rows.extend(b.into_iter().skip(recipe.mapping.header_rows));
        let table = tabular::from_rows(rows).unwrap();

        let result = tabular::extract(&table.rows, &column_mapping(&recipe.mapping));

        let got: Vec<(String, String, String)> = result
            .rows
            .iter()
            .map(|r| {
                (
                    r.name.clone(),
                    r.value.to_string(),
                    r.price.map_or(String::new(), |p| p.to_string()),
                )
            })
            .collect();
        assert_eq!(result.skipped, vec![]);
        assert_eq!(
            got,
            vec![
                (
                    "1111 Alpha".to_owned(),
                    "125.00".to_owned(),
                    "12.50".to_owned()
                ),
                ("2222 Beta".to_owned(), "300".to_owned(), "100".to_owned()),
                (
                    "3333 Gamma".to_owned(),
                    "2000".to_owned(),
                    "1000".to_owned()
                ),
                ("4444 Delta".to_owned(), "7".to_owned(), "7".to_owned()),
            ]
        );
    }

    #[test]
    fn the_daiwa_preset_reads_a_page_laid_out_like_the_real_one() {
        // A made-up page with the layout of the real one: summary tables, a heading table, then
        // one one-row table per holding (code and name on two lines, account, quantity with a
        // unit, sellable quantity, cost, price, evaluation with a unit, profit, a trade button).
        let row = |cells: &[&str]| vec![cells.iter().map(|c| (*c).to_owned()).collect::<Vec<_>>()];
        let raw = vec![
            row(&["Total", "+1", "1", "1"]),
            row(&["Search", ""]),
            row(&[
                "コード▲ /[市場]\n銘柄名",
                "口座",
                "保有株数",
                "売付可能株数",
                "取得単価",
                "現在値",
                "評価額",
                "評価損益",
                "取引",
            ]),
            row(&[
                "1111 [T]\nAlpha Co",
                "Taxable",
                "10株",
                "10株",
                "1,000円",
                "1,250円",
                "12,500円",
                "+2,500円",
                "",
            ]),
            row(&[
                "2222 [T]\nBeta Co",
                "NISA",
                "3株",
                "3株",
                "2,000.5円",
                "2,100円",
                "6,300円",
                "+298円",
                "",
            ]),
            row(&["Note", "x"]),
        ];
        let (state, main, _, _, _) = recipe_state("daiwa");
        state
            .apply(Edit::AddPresetRecipe {
                preset: "daiwa-connect-stocks".to_owned(),
                account: main,
            })
            .unwrap();
        let recipe = state.view().unwrap().recipes[0].recipe.clone();
        assert!(recipe.following_tables && !recipe.all_tables);
        assert_eq!(recipe.start_site, "connect-sec.co.jp");
        // It opens the list of holdings by a click on its menu entry, and the texts pass the same
        // rules as any link.
        assert_eq!(recipe.links.len(), 1);
        let link = &recipe.links[0];
        assert!(link.menu.is_some() && link.heading.is_some());
        assert_eq!(recipe.allowed_sites, vec!["connect-sec.co.jp".to_owned()]);
        for text in [
            link.text.as_str(),
            link.menu.as_deref().unwrap(),
            link.heading.as_deref().unwrap(),
        ] {
            assert_eq!(browser::link_text_allowed(text), Ok(()), "{text}");
        }

        let table = crate::browser_session::pick_table_for_test(
            raw,
            &recipe.table_starts_with,
            recipe.following_tables,
        );
        let result = tabular::extract(&table.rows, &column_mapping(&recipe.mapping));

        assert_eq!(result.skipped, vec![]);
        let got: Vec<(String, String, String)> = result
            .rows
            .iter()
            .map(|r| {
                (
                    r.name.clone(),
                    r.value.to_string(),
                    r.quantity.map_or(String::new(), |q| q.to_string()),
                )
            })
            .collect();
        assert_eq!(
            got,
            vec![
                (
                    "1111 [T] Alpha Co".to_owned(),
                    "12500".to_owned(),
                    "10".to_owned()
                ),
                (
                    "2222 [T] Beta Co".to_owned(),
                    "6300".to_owned(),
                    "3".to_owned()
                ),
            ]
        );
    }

    #[test]
    fn opening_the_login_pages_needs_the_browser_only_when_there_is_one_to_open() {
        let (state, main, _, _, _) = recipe_state("login");
        // No recipe names a site: nothing to open, and no browser is needed.
        let result = state.open_login_pages().unwrap();
        assert!(
            result.opened.is_empty() && result.already_open.is_empty() && result.unknown.is_empty()
        );

        // A site without a known login page is reported, again without the browser.
        let mut recipe = domestic_recipe(main);
        recipe.start_site = "unknown.example".to_owned();
        state.apply(Edit::SetRecipe { recipe }).unwrap();
        let result = state.open_login_pages().unwrap();
        assert_eq!(result.unknown, vec!["unknown.example".to_owned()]);

        // A site with a login page needs the browser.
        for preset in portfolio::presets() {
            state
                .apply(Edit::AddPresetRecipe {
                    preset: preset.id.to_owned(),
                    account: main,
                })
                .unwrap();
        }
        let error = state.open_login_pages().unwrap_err();
        assert!(error.contains("Open the browser first"), "{error}");
    }

    #[test]
    fn a_clicked_menu_link_learns_no_reference_and_the_recipe_stays_the_saved_one() {
        let (state, main, _, _, _) = recipe_state("daiwa-learn");
        state
            .apply(Edit::AddPresetRecipe {
                preset: "daiwa-connect-cash".to_owned(),
                account: main,
            })
            .unwrap();
        let saved = state.view().unwrap().recipes[0].recipe.clone();
        // What a successful click run hands back: one empty reference per clicked link.
        let learned = with_learned_references(&saved, vec![String::new()]);
        assert_eq!(learned, saved, "a menu link keeps no reference");

        // A plain link still learns where it goes.
        let mut plain = saved.clone();
        plain.links[0].menu = None;
        plain.links[0].heading = None;
        let learned = with_learned_references(&plain, vec!["h:abc".to_owned()]);
        assert_eq!(learned.links[0].reference.as_deref(), Some("h:abc"));
    }

    #[test]
    fn the_monex_funds_preset_reads_one_table_among_several_that_start_with_the_same_cell() {
        // A made-up page with the layout of the real one: every table of the page starts with the
        // same heading cell, the other kinds of holdings have no rows (one cell saying so), and
        // the investment trust table has a heading row and one row per holding.
        let row = |cells: &[&str]| cells.iter().map(|c| (*c).to_owned()).collect::<Vec<_>>();
        let empty_table = |heading: &[&str]| vec![row(heading), row(&["No holdings"])];
        let raw = vec![
            vec![row(&["Total", "1"])],
            empty_table(&["銘柄", "Other heading", "", "", "", "", "", "", ""]),
            vec![
                row(&[
                    "銘柄",
                    "口座区分\n預り区分",
                    "基準価額（円）\n前日比",
                    "分配金の取扱い",
                    "保有数（口）",
                    "平均取得単価（円）",
                    "概算評価額（円）",
                    "評価損益（円）評価損益率",
                    "取引",
                ]),
                row(&[
                    "Alpha Fund",
                    "Taxable",
                    "12,000\n+10",
                    "x",
                    "5,000",
                    "9,000",
                    "6,000",
                    "+1,000\n+10%",
                    "",
                ]),
                row(&[
                    "Beta Fund",
                    "NISA",
                    "20,000\n-20",
                    "x",
                    "1,500",
                    "9,000",
                    "3,000",
                    "-500\n-5%",
                    "",
                ]),
            ],
            empty_table(&[
                "銘柄",
                "口座区分\n預り区分",
                "基準価額（円）\n前日比",
                "分配金の取扱い",
                "保有数（口）",
                "平均取得単価（円）",
                "概算評価額（円）",
                "評価損益（円）評価損益率",
                "取引",
            ])
            .into_iter()
            .skip(1)
            .collect(),
        ];
        let (state, main, _, _, _) = recipe_state("monex");
        state
            .apply(Edit::AddPresetRecipe {
                preset: "monex-funds".to_owned(),
                account: main,
            })
            .unwrap();
        let recipe = state.view().unwrap().recipes[0].recipe.clone();
        assert_eq!(recipe.start_site, "monex.co.jp");
        assert_eq!(recipe.allowed_sites, vec!["monex.co.jp".to_owned()]);
        assert_eq!(recipe.links.len(), 1);
        assert!(recipe.links[0].menu.is_none());
        assert_eq!(browser::link_text_allowed(&recipe.links[0].text), Ok(()));

        let table = crate::browser_session::pick_table_for_test(
            raw,
            &recipe.table_starts_with,
            recipe.following_tables,
        );
        let result = tabular::extract(&table.rows, &column_mapping(&recipe.mapping));

        assert_eq!(result.skipped, vec![]);
        let got: Vec<(String, String, String)> = result
            .rows
            .iter()
            .map(|r| {
                (
                    r.name.clone(),
                    r.value.to_string(),
                    r.quantity.map_or(String::new(), |q| q.to_string()),
                )
            })
            .collect();
        assert_eq!(
            got,
            vec![
                (
                    "Alpha Fund".to_owned(),
                    "6000".to_owned(),
                    "5000".to_owned()
                ),
                ("Beta Fund".to_owned(), "3000".to_owned(), "1500".to_owned()),
            ]
        );
    }

    #[test]
    fn the_rakuten_all_preset_reads_every_holding_of_the_outer_table_and_not_the_summary() {
        // A made-up page with the layout of the real one: the holdings of every kind are one table
        // (an outer table: some of its cells hold small tables, which are leaf tables of their own
        // with an amount and no name), with one visible row per holding after the heading row, and
        // a row of one empty cell at the end. Next to it is a summary table by kind that does not
        // start with the same cells. The evaluation cell has two lines (the evaluation, the profit).
        let (state, main, _, _, _) = recipe_state("rakuten");
        state
            .apply(Edit::AddPresetRecipe {
                preset: "rakuten-all".to_owned(),
                account: main,
            })
            .unwrap();
        let recipe = state.view().unwrap().recipes[0].recipe.clone();
        assert_eq!(recipe.start_site, "rakuten-sec.co.jp");
        assert_eq!(recipe.allowed_sites, vec!["rakuten-sec.co.jp".to_owned()]);
        assert_eq!(recipe.links.len(), 1);
        assert!(recipe.links[0].menu.is_none());
        assert_eq!(browser::link_text_allowed(&recipe.links[0].text), Ok(()));
        // The page shows yen, so a name linked to a product in another currency is reported
        // instead of being valued as if its yen amount were in that currency.
        assert_eq!(recipe.currency, "JPY");

        let row = |cells: &[&str]| cells.iter().map(|c| (*c).to_owned()).collect::<Vec<_>>();
        let heading = {
            let mut cells = vec![recipe.table_starts_with.as_str()];
            cells.resize(7, "");
            row(&cells)
        };
        let holding = |kind: &str, name: &str, units: &str, value: &str| {
            row(&[
                kind,
                name,
                "Taxable",
                units,
                "5,000.00 円\n6,000 円",
                "12,000 円\n+5 円",
                value,
            ])
        };
        let holdings = vec![
            heading,
            holding("投資信託", "Alpha Fund", "5,000 口", "6,000 円\n+100 円"),
            holding("投資信託", "Beta Fund", "1,500 口", "3,000 円\n-50 円"),
            // A stock: its price is per share, so the value is the evaluation, not units x price.
            holding("国内株式", "Gamma Corp", "100 株", "1,250 円\n+10 円"),
            // A kind that no rule names is left out and reported.
            holding("Other kind", "Delta Note", "1", "9 円"),
            row(&[""]),
        ];
        let page = browser::PageTables {
            leaf: vec![
                vec![row(&["6,000 円"])],
                vec![row(&["+1,000 円"])],
                vec![row(&["Total", "9,000 円"])],
            ],
            outer: vec![vec![row(&["Kind", "Total", "Gain"])], holdings],
        };

        let table = crate::browser_session::pick_page_table_for_test(
            page,
            &recipe.table_starts_with,
            recipe.following_tables,
        );
        let result = tabular::extract(&table.rows, &column_mapping(&recipe.mapping));

        // A row of a kind with no rule is reported (row 5: the heading is row 1); the empty row at
        // the end is dropped when the table is made.
        assert_eq!(
            result.skipped,
            vec![tabular::Skipped {
                row: 5,
                reason: tabular::SkipReason::UnknownKind
            }]
        );
        let got: Vec<(String, String, String)> = result
            .rows
            .iter()
            .map(|r| {
                (
                    r.name.clone(),
                    r.value.to_string(),
                    r.quantity.map_or(String::new(), |q| q.to_string()),
                )
            })
            .collect();
        assert_eq!(
            got,
            vec![
                (
                    "Alpha Fund".to_owned(),
                    "6000".to_owned(),
                    "5000".to_owned()
                ),
                ("Beta Fund".to_owned(), "1800".to_owned(), "1500".to_owned()),
                ("Gamma Corp".to_owned(), "1250".to_owned(), "100".to_owned()),
            ]
        );
    }

    #[test]
    fn a_preset_recipe_is_run_from_the_current_preset_and_an_unknown_one_fails_alone() {
        let (state, main, other, _, _) = recipe_state("preset-ref");
        state
            .apply(Edit::AddPresetRecipe {
                preset: "sbi-funds".to_owned(),
                account: main,
            })
            .unwrap();
        save_recipe(&state, "Custom", other);
        let view = state.view().unwrap();
        // The stored recipe only refers to the preset; the screen gets the definition.
        assert!(view.portfolio.recipes[0].table_starts_with.is_empty());
        assert_eq!(view.recipes[0].recipe.table_starts_with, "投資信託（金額/");
        assert!(!view.recipes[0].preset_missing);
        assert_eq!(view.recipes[1].recipe, view.portfolio.recipes[1]);

        // The question about a link is answered against what the recipe stands for, and only
        // the destination is stored.
        let effective = view.recipes[0].recipe.clone();
        let view = state
            .set_link_reference(&effective, 0, Some("h:0123456789abcdef".to_owned()))
            .unwrap();
        assert!(view.portfolio.recipes[0].table_starts_with.is_empty());
        assert_eq!(
            view.recipes[0].recipe.links[0].reference.as_deref(),
            Some("h:0123456789abcdef")
        );
        // Asked again about the old state: refused.
        assert!(state.set_link_reference(&effective, 0, None).is_err());

        // Running a recipe whose preset is gone is an error of that recipe only.
        let mut gone = state.portfolio_snapshot().unwrap();
        gone.recipes[0].preset = Some("removed-in-a-later-version".to_owned());
        *state.portfolio.lock().unwrap() = Ok(gone);
        let view = state.view().unwrap();
        assert!(view.recipes[0].preset_missing);
        assert!(!view.recipes[1].preset_missing);
        assert!(
            state
                .run_recipe(&view.recipes[0].recipe.name, "page")
                .unwrap_err()
                .contains("preset")
        );
        // The other recipe is not affected (it only fails here for lack of a browser).
        assert!(
            !state
                .run_recipe("Custom", "page")
                .unwrap_err()
                .contains("preset")
        );
    }

    #[test]
    fn the_daiwa_cash_preset_reads_only_the_yen_balance_of_a_page_like_the_real_one() {
        // A made-up page with the layout of the real one: a table of label and amount pairs
        // (the yen balance first, then valuations and the total) and a second table.
        let pairs = |rows: &[(&str, &str)]| {
            rows.iter()
                .map(|(a, b)| vec![(*a).to_owned(), (*b).to_owned()])
                .collect::<Vec<_>>()
        };
        let raw = vec![
            pairs(&[
                ("金銭残高", "123,456円"),
                ("株式評価額", "9,999円"),
                ("投資信託評価額", "0円"),
                ("外国株式評価額", "0円"),
                ("合計評価額", "133,455円"),
            ]),
            pairs(&[("買付可能額", "123,456円"), ("出金可能額", "100,000円")]),
        ];
        let (state, main, _, _, _) = recipe_state("daiwa-cash");
        state
            .apply(Edit::AddPresetRecipe {
                preset: "daiwa-connect-cash".to_owned(),
                account: main,
            })
            .unwrap();
        let recipe = state.view().unwrap().recipes[0].recipe.clone();
        assert!(!recipe.following_tables);
        assert_eq!(recipe.links.len(), 1);
        let link = &recipe.links[0];
        assert!(link.menu.is_some() && link.heading.is_some());
        for text in [
            link.text.as_str(),
            link.menu.as_deref().unwrap(),
            link.heading.as_deref().unwrap(),
        ] {
            assert_eq!(browser::link_text_allowed(text), Ok(()), "{text}");
        }

        let table =
            crate::browser_session::pick_table_for_test(raw, &recipe.table_starts_with, false);
        let result = tabular::extract(&table.rows, &column_mapping(&recipe.mapping));

        assert_eq!(result.skipped, vec![]);
        assert_eq!(
            result.rows.len(),
            1,
            "only the yen balance, not the valuations or the total"
        );
        assert_eq!(result.rows[0].name, "金銭残高");
        assert_eq!(result.rows[0].value, rust_decimal::Decimal::from(123_456));
    }
}
