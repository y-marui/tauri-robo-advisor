mod app_state;
mod browser_session;
mod data_folder;
mod exporter;
mod secrets;
mod store;

use std::sync::Arc;

use app_state::{
    AppState, PortfolioView, RecipeOutcome, RecipeRunView, TableImportPlan, TablePreview,
};
use bitflyer::{Client, Holdings, ReqwestTransport};
use browser_session::{BrowserInfo, BrowserSession, FetchedTables, PageInfo};
use portfolio::{Edit, ImportPlan, ImportRow};
use rebalance::{Holding, Params, Proposal, Target};
use suggest::{AccountBalance, Overview, Request as SuggestRequest, Suggestion};
use tauri::{Manager, State};

// Thin boundary only: the calculation lives in the `rebalance` crate. Amounts
// cross the IPC boundary as decimal strings so JavaScript never rounds them.
#[tauri::command]
fn propose_rebalance(
    holdings: Vec<Holding>,
    targets: Vec<Target>,
    params: Params,
) -> Result<Vec<Proposal>, String> {
    rebalance::propose(&holdings, &targets, &params).map_err(|e| e.to_string())
}

// The commands below only ever hand the frontend a yes/no or the fetched
// holdings; the API key and secret never cross the IPC boundary outward.
// Keychain access can block (and may prompt the user), so it runs off the
// async runtime's worker threads.
#[tauri::command]
async fn save_bitflyer_credentials(api_key: String, api_secret: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || secrets::save_bitflyer(&api_key, &api_secret))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn has_bitflyer_credentials() -> Result<bool, String> {
    tauri::async_runtime::spawn_blocking(|| secrets::load_bitflyer().map(|c| c.is_some()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn delete_bitflyer_credentials() -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(secrets::delete_bitflyer)
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn fetch_bitflyer_holdings() -> Result<Holdings, String> {
    let client = bitflyer_client().await?;
    client.holdings().await.map_err(|e| e.to_string())
}

// Quantities from the API, prices from the public price page: the table the person
// loaded from the page (kept here, never sent through the frontend) gives one yen
// price per currency. The app opens no connection to the page; the browser path read it.
#[tauri::command]
async fn fetch_bitflyer_holdings_with_page_prices(
    state: State<'_, Arc<AppState>>,
) -> Result<Holdings, String> {
    let prices = {
        let state = Arc::clone(&state);
        tauri::async_runtime::spawn_blocking(move || state.bitflyer_page_prices())
            .await
            .map_err(|e| e.to_string())??
    };
    let client = bitflyer_client().await?;
    client
        .holdings_with_page_prices(&prices)
        .await
        .map_err(|e| e.to_string())
}

async fn bitflyer_client() -> Result<Client<ReqwestTransport>, String> {
    let credentials = tauri::async_runtime::spawn_blocking(secrets::load_bitflyer)
        .await
        .map_err(|e| e.to_string())??
        .ok_or_else(|| "No bitFlyer API credentials are saved.".to_owned())?;
    let transport = ReqwestTransport::new().map_err(|e| e.to_string())?;
    Ok(Client::new(transport, credentials))
}

// The portfolio lives in Rust. The frontend asks for a snapshot and sends
// edits; every rule is enforced here, and each accepted edit is saved to disk
// before the new state is returned. File access runs off the async workers.
#[tauri::command]
async fn get_portfolio(state: State<'_, Arc<AppState>>) -> Result<PortfolioView, String> {
    let state = Arc::clone(&state);
    tauri::async_runtime::spawn_blocking(move || state.view())
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn apply_portfolio_edit(
    state: State<'_, Arc<AppState>>,
    edit: Edit,
    location: u64,
) -> Result<PortfolioView, String> {
    let state = Arc::clone(&state);
    tauri::async_runtime::spawn_blocking(move || state.apply_at(edit, location))
        .await
        .map_err(|e| e.to_string())?
}

// A read-only preview of an import: which rows are linked to products by an
// alias the user confirmed, and which are not. Nothing is changed or saved.
#[tauri::command]
async fn plan_import(
    state: State<'_, Arc<AppState>>,
    source: String,
    currency: String,
    rows: Vec<ImportRow>,
) -> Result<ImportPlan, String> {
    let state = Arc::clone(&state);
    tauri::async_runtime::spawn_blocking(move || state.plan_import(&source, &currency, &rows))
        .await
        .map_err(|e| e.to_string())?
}

// Reading a table for an import. The input comes from a file the user chose or
// from pasted text; both are untrusted, so the `tabular` crate limits the size
// and returns an error for anything it cannot read. The table is kept here and
// only its top is returned.
#[tauri::command]
async fn load_table_file(
    state: State<'_, Arc<AppState>>,
    bytes: Vec<u8>,
) -> Result<TablePreview, String> {
    let state = Arc::clone(&state);
    tauri::async_runtime::spawn_blocking(move || {
        let table = tabular::read_file(&bytes).map_err(|e| e.to_string())?;
        state.keep_table(table)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn load_table_text(
    state: State<'_, Arc<AppState>>,
    text: String,
) -> Result<TablePreview, String> {
    let state = Arc::clone(&state);
    tauri::async_runtime::spawn_blocking(move || {
        let table = tabular::read_text(&text).map_err(|e| e.to_string())?;
        state.keep_table(table)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn clear_table(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let state = Arc::clone(&state);
    tauri::async_runtime::spawn_blocking(move || state.clear_table())
        .await
        .map_err(|e| e.to_string())?
}

// A read-only preview of importing the loaded table with the chosen columns.
#[tauri::command]
async fn plan_table_import(
    state: State<'_, Arc<AppState>>,
    source: String,
    currency: String,
    mapping: tabular::ColumnMapping,
) -> Result<TableImportPlan, String> {
    let state = Arc::clone(&state);
    tauri::async_runtime::spawn_blocking(move || {
        state.plan_table_import(&source, &currency, &mapping)
    })
    .await
    .map_err(|e| e.to_string())?
}

// Fetching tables from a page of a browser the user logs in to by hand. The
// browser to start is picked by kind from the ones found here (never by a path
// from the frontend); the commands only list pages and read their tables.
#[tauri::command]
fn list_browsers() -> Vec<BrowserInfo> {
    BrowserSession::installed()
}

#[tauri::command]
async fn open_browser(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    kind: browser::Kind,
    profile: Option<String>,
) -> Result<(), String> {
    let profile = match profile.filter(|p| !p.trim().is_empty()) {
        Some(folder) => std::path::PathBuf::from(folder.trim()),
        None => {
            browser_session::default_profile(&app.path().app_data_dir().map_err(|e| e.to_string())?)
        }
    };
    let state = Arc::clone(&state);
    tauri::async_runtime::spawn_blocking(move || state.browser.open(kind, &profile))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn browser_pages(state: State<'_, Arc<AppState>>) -> Result<Vec<PageInfo>, String> {
    let state = Arc::clone(&state);
    tauri::async_runtime::spawn_blocking(move || state.browser.pages())
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn fetch_browser_tables(
    state: State<'_, Arc<AppState>>,
    page: String,
) -> Result<FetchedTables, String> {
    let state = Arc::clone(&state);
    tauri::async_runtime::spawn_blocking(move || state.browser.fetch_tables(&page))
        .await
        .map_err(|e| e.to_string())?
}

// Moves one fetched table into the import, where the user picks the columns.
#[tauri::command]
async fn use_fetched_table(
    state: State<'_, Arc<AppState>>,
    index: usize,
) -> Result<TablePreview, String> {
    let state = Arc::clone(&state);
    tauri::async_runtime::spawn_blocking(move || {
        let table = state.browser.take_table(index)?;
        state.keep_table(table)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn close_browser(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let state = Arc::clone(&state);
    tauri::async_runtime::spawn_blocking(move || state.browser.close())
        .await
        .map_err(|e| e.to_string())?
}

// Saved fetch recipes: follow the recipe's links in the browser the user logged in to
// (plain links that only show another page), read its table, and keep the rows. The
// holdings change only when `apply_fetch_all` is called, after the user has seen the result.
#[tauri::command]
async fn run_recipe(
    state: State<'_, Arc<AppState>>,
    name: String,
    page: String,
) -> Result<RecipeOutcome, String> {
    let state = Arc::clone(&state);
    tauri::async_runtime::spawn_blocking(move || state.run_recipe(&name, &page))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn fetch_all_plan(state: State<'_, Arc<AppState>>) -> Result<Vec<RecipeRunView>, String> {
    let state = Arc::clone(&state);
    tauri::async_runtime::spawn_blocking(move || state.fetch_all_plan())
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn set_link_reference(
    state: State<'_, Arc<AppState>>,
    recipe: portfolio::Recipe,
    link: usize,
    reference: Option<String>,
) -> Result<PortfolioView, String> {
    let state = Arc::clone(&state);
    tauri::async_runtime::spawn_blocking(move || state.set_link_reference(&recipe, link, reference))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn apply_fetch_all(state: State<'_, Arc<AppState>>) -> Result<PortfolioView, String> {
    let state = Arc::clone(&state);
    tauri::async_runtime::spawn_blocking(move || state.apply_fetch_all())
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn discard_fetch_run(state: State<'_, Arc<AppState>>, name: String) -> Result<(), String> {
    let state = Arc::clone(&state);
    tauri::async_runtime::spawn_blocking(move || state.discard_run(&name))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn clear_fetch_runs(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let state = Arc::clone(&state);
    tauri::async_runtime::spawn_blocking(move || state.clear_runs())
        .await
        .map_err(|e| e.to_string())?
}

// Where the data file lives. The folder is chosen by the user on their own
// computer (for example one that iCloud Drive syncs); the app sends nothing.
#[tauri::command]
async fn set_data_folder(
    state: State<'_, Arc<AppState>>,
    folder: Option<String>,
    use_existing: bool,
) -> Result<PortfolioView, String> {
    let state = Arc::clone(&state);
    let folder = folder
        .filter(|f| !f.trim().is_empty())
        .map(|f| std::path::PathBuf::from(f.trim()));
    tauri::async_runtime::spawn_blocking(move || state.set_data_folder(folder, use_existing))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
fn icloud_data_folder() -> Option<String> {
    data_folder::icloud_folder().map(|p| p.display().to_string())
}

// Opens the login page of each site a recipe reads from, in the browser the app opened. The
// browser only goes to the address; the person logs in by hand.
#[tauri::command]
async fn open_login_pages(
    state: State<'_, Arc<AppState>>,
) -> Result<browser_session::LoginPages, String> {
    let state = Arc::clone(&state);
    tauri::async_runtime::spawn_blocking(move || state.open_login_pages())
        .await
        .map_err(|e| e.to_string())?
}

// The recipes that ship with the app (data, not code). Nothing is changed.
#[tauri::command]
fn list_presets() -> Vec<portfolio::PresetInfo> {
    portfolio::presets()
}

// A read-only valuation for the balance charts.
#[tauri::command]
async fn get_overview(state: State<'_, Arc<AppState>>) -> Result<Overview, String> {
    let state = Arc::clone(&state);
    tauri::async_runtime::spawn_blocking(move || state.overview())
        .await
        .map_err(|e| e.to_string())?
}

// A read-only comparison of one account's products with their weights.
#[tauri::command]
async fn get_account_balance(
    state: State<'_, Arc<AppState>>,
    account: portfolio::AccountId,
) -> Result<AccountBalance, String> {
    let state = Arc::clone(&state);
    tauri::async_runtime::spawn_blocking(move || state.account_balance(account))
        .await
        .map_err(|e| e.to_string())?
}

// A read-only calculation: nothing is changed or saved.
#[tauri::command]
async fn suggest_rebalance(
    state: State<'_, Arc<AppState>>,
    request: SuggestRequest,
) -> Result<Suggestion, String> {
    let state = Arc::clone(&state);
    tauri::async_runtime::spawn_blocking(move || state.suggest(&request))
        .await
        .map_err(|e| e.to_string())?
}

// Writes the full-record export (CSV files and JSON) into a new folder in the
// user's Downloads and returns the folder. The user triggers it; the names and
// location are chosen here, never by the frontend.
#[tauri::command]
async fn export_all(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    let base = app
        .path()
        .download_dir()
        .or_else(|_| app.path().document_dir())
        .map_err(|_| "Could not find a folder to export to.".to_owned())?;
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let (stamp, exported_at) = exporter::utc_stamp(seconds);
    let state = Arc::clone(&state);
    tauri::async_runtime::spawn_blocking(move || {
        let info = export::Info {
            app_version: env!("CARGO_PKG_VERSION"),
            exported_at: &exported_at,
        };
        let files = state.export_files(&info)?;
        let folder = exporter::write_export(&base, &stamp, &files)?;
        Ok(folder.to_string_lossy().into_owned())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let directory = app.path().app_data_dir()?;
            std::fs::create_dir_all(&directory)?;
            app.manage(Arc::new(AppState::open(directory)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            propose_rebalance,
            save_bitflyer_credentials,
            has_bitflyer_credentials,
            delete_bitflyer_credentials,
            fetch_bitflyer_holdings,
            fetch_bitflyer_holdings_with_page_prices,
            get_portfolio,
            apply_portfolio_edit,
            get_overview,
            list_presets,
            open_login_pages,
            plan_import,
            load_table_file,
            load_table_text,
            clear_table,
            plan_table_import,
            set_data_folder,
            icloud_data_folder,
            run_recipe,
            fetch_all_plan,
            set_link_reference,
            apply_fetch_all,
            clear_fetch_runs,
            discard_fetch_run,
            list_browsers,
            open_browser,
            browser_pages,
            fetch_browser_tables,
            use_fetched_table,
            close_browser,
            get_account_balance,
            suggest_rebalance,
            export_all
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            // Do not leave the dedicated browser open when the app quits.
            if let tauri::RunEvent::Exit = event
                && let Some(state) = app.try_state::<Arc<AppState>>()
            {
                let _ = state.browser.close();
            }
        });
}
