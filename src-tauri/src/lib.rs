mod app_state;
mod exporter;
mod secrets;
mod store;

use std::sync::Arc;

use app_state::{AppState, PortfolioView};
use bitflyer::{Client, Holdings, ReqwestTransport};
use portfolio::Edit;
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
    let credentials = tauri::async_runtime::spawn_blocking(secrets::load_bitflyer)
        .await
        .map_err(|e| e.to_string())??
        .ok_or_else(|| "No bitFlyer API credentials are saved.".to_owned())?;
    let transport = ReqwestTransport::new().map_err(|e| e.to_string())?;
    Client::new(transport, credentials)
        .holdings()
        .await
        .map_err(|e| e.to_string())
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
) -> Result<PortfolioView, String> {
    let state = Arc::clone(&state);
    tauri::async_runtime::spawn_blocking(move || state.apply(edit))
        .await
        .map_err(|e| e.to_string())?
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
            app.manage(Arc::new(AppState::load(directory.join("portfolio.json"))));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            propose_rebalance,
            save_bitflyer_credentials,
            has_bitflyer_credentials,
            delete_bitflyer_credentials,
            fetch_bitflyer_holdings,
            get_portfolio,
            apply_portfolio_edit,
            get_overview,
            get_account_balance,
            suggest_rebalance,
            export_all
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
