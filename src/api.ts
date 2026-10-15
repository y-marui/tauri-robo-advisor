import { invoke } from "@tauri-apps/api/core";

import type {
  AccountBalance,
  BitflyerHoldings,
  Edit,
  ImportPlan,
  ImportRow,
  Overview,
  PortfolioView,
  Suggestion,
  SuggestRequest,
} from "./types.ts";

/** The current portfolio and the problems that block suggestions. */
export function getPortfolio(): Promise<PortfolioView> {
  return invoke<PortfolioView>("get_portfolio");
}

/**
 * Writes every record as CSV and JSON files into a new folder and returns the
 * folder. Rust chooses the location and the names.
 */
export function exportAll(): Promise<string> {
  return invoke<string>("export_all");
}

/**
 * Sends one edit. Rust validates it, saves it, and returns the new state; a
 * rejected edit rejects this promise with a message and changes nothing.
 */
export function applyEdit(edit: Edit): Promise<PortfolioView> {
  return invoke<PortfolioView>("apply_portfolio_edit", { edit });
}

/** Values every category in JPY for the balance charts. */
export function getOverview(): Promise<Overview> {
  return invoke<Overview>("get_overview");
}

/** A read-only suggestion; nothing is changed or saved. */
export function suggestRebalance(request: SuggestRequest): Promise<Suggestion> {
  return invoke<Suggestion>("suggest_rebalance", { request });
}

/** Compares one account's products with their weights. */
export function getAccountBalance(account: number): Promise<AccountBalance> {
  return invoke<AccountBalance>("get_account_balance", { account });
}

/** Links imported rows to products by alias; nothing is changed. */
export function planImport(source: string, currency: string, rows: ImportRow[]): Promise<ImportPlan> {
  return invoke<ImportPlan>("plan_import", { source, currency, rows });
}

/** Saves the key in the OS keychain. It is never returned to the frontend. */
export function saveBitflyerCredentials(apiKey: string, apiSecret: string): Promise<void> {
  return invoke<void>("save_bitflyer_credentials", { apiKey, apiSecret });
}

export function hasBitflyerCredentials(): Promise<boolean> {
  return invoke<boolean>("has_bitflyer_credentials");
}

export function deleteBitflyerCredentials(): Promise<void> {
  return invoke<void>("delete_bitflyer_credentials");
}

/** Fetches the balances (read-only) and values them in JPY. */
export function fetchBitflyerHoldings(): Promise<BitflyerHoldings> {
  return invoke<BitflyerHoldings>("fetch_bitflyer_holdings");
}
