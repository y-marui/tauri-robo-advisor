use rebalance::{Holding, Params, Proposal, Target};

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![greet, propose_rebalance])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
