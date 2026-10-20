import { invoke } from "@tauri-apps/api/core";

import type {
  AccountBalance,
  BrowserInfo,
  BrowserKind,
  BrowserPage,
  BrowserSettings,
  FetchedTables,
  BitflyerHoldings,
  CoincheckHoldings,
  Edit,
  ImportPlan,
  ImportRow,
  Overview,
  ColumnMapping,
  LoginPages,
  PortfolioView,
  PresetInfo,
  Recipe,
  RecipeOutcome,
  RecipeRun,
  TableImportPlan,
  TablePreview,
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
 * rejected edit rejects this promise with a message and changes nothing. `location` is
 * `data.location` of the view the edit was made from; Rust refuses it if the data folder
 * was switched since.
 */
export function applyEdit(edit: Edit, location: number): Promise<PortfolioView> {
  return invoke<PortfolioView>("apply_portfolio_edit", { edit, location });
}

/**
 * Sets (or clears) the saved reference of one link, only if the recipe is still the
 * one given; Rust checks and writes under one lock.
 */
export function setLinkReference(recipe: Recipe, link: number, reference: string | null): Promise<PortfolioView> {
  return invoke<PortfolioView>("set_link_reference", { recipe, link, reference });
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

/**
 * Fetches the balances (read-only) and values them with the prices of the bitFlyer price page,
 * read from the page open in the browser. The Lightning price is read for comparison.
 */
export function fetchBitflyerHoldingsWithPagePrices(): Promise<BitflyerHoldings> {
  return invoke<BitflyerHoldings>("fetch_bitflyer_holdings_with_page_prices");
}

/** Starts the browser and sends it to the bitFlyer price page (nothing is typed or clicked). */
export function openBitflyerPricePage(): Promise<void> {
  return invoke<void>("open_bitflyer_price_page");
}

/** Saves the Coincheck key in the OS keychain. It is never returned to the frontend. */
export function saveCoincheckCredentials(apiKey: string, apiSecret: string): Promise<void> {
  return invoke<void>("save_coincheck_credentials", { apiKey, apiSecret });
}

export function hasCoincheckCredentials(): Promise<boolean> {
  return invoke<boolean>("has_coincheck_credentials");
}

export function deleteCoincheckCredentials(): Promise<void> {
  return invoke<void>("delete_coincheck_credentials");
}

/** Fetches the Coincheck balances (read-only) and values them in JPY with the public ticker. */
export function fetchCoincheckHoldings(): Promise<CoincheckHoldings> {
  return invoke<CoincheckHoldings>("fetch_coincheck_holdings");
}

/** Sends a CSV or text file's bytes to Rust, which decodes (UTF-8 or Shift-JIS) and parses them. */
export function loadTableFile(bytes: number[]): Promise<TablePreview> {
  return invoke<TablePreview>("load_table_file", { bytes });
}

/** Parses text pasted from a brokerage's table. */
export function loadTableText(text: string): Promise<TablePreview> {
  return invoke<TablePreview>("load_table_text", { text });
}

export function clearTable(): Promise<void> {
  return invoke<void>("clear_table");
}

/** Picks the chosen columns of the loaded table and links the rows to products; nothing is changed. */
export function planTableImport(
  source: string,
  currency: string,
  mapping: ColumnMapping,
): Promise<TableImportPlan> {
  return invoke<TableImportPlan>("plan_table_import", { source, currency, mapping });
}

/** The Chrome, Edge, and Brave found on this computer. */
export function listBrowsers(): Promise<BrowserInfo[]> {
  return invoke<BrowserInfo[]>("list_browsers");
}

/** The browser and profile folder last used (kept in a local file, not in the portfolio data). */
export function getBrowserSettings(): Promise<BrowserSettings> {
  return invoke<BrowserSettings>("get_browser_settings");
}

/** Starts the browser with a profile of its own (or reuses one that is running). */
export function openBrowser(kind: BrowserKind, profile: string | null): Promise<void> {
  return invoke<void>("open_browser", { kind, profile });
}

export function browserPages(): Promise<BrowserPage[]> {
  return invoke<BrowserPage[]>("browser_pages");
}

/** Reads the tables of a page the user has logged in to and opened. */
export function fetchBrowserTables(page: string): Promise<FetchedTables> {
  return invoke<FetchedTables>("fetch_browser_tables", { page });
}

/** Moves a fetched table into the import, where the columns are chosen. */
export function useFetchedTable(index: number): Promise<TablePreview> {
  return invoke<TablePreview>("use_fetched_table", { index });
}

export function closeBrowser(): Promise<void> {
  return invoke<void>("close_browser");
}

/** Opens the login page of each of the given sites that a recipe reads from; the person logs in by hand. */
export function openLoginPages(sites: string[]): Promise<LoginPages> {
  return invoke<LoginPages>("open_login_pages", { sites });
}

/** The recipes that ship with the app. */
export function listPresets(): Promise<PresetInfo[]> {
  return invoke<PresetInfo[]>("list_presets");
}

/** Follows the recipe's links in the browser and reads its table; the holdings are not changed. */
export function runRecipe(name: string, page: string): Promise<RecipeOutcome> {
  return invoke<RecipeOutcome>("run_recipe", { name, page });
}

/** The rows the recipes read, planned again with the aliases as they are now. */
export function fetchAllPlan(): Promise<RecipeRun[]> {
  return invoke<RecipeRun[]>("fetch_all_plan");
}

/** Replaces each account's holdings with the linked rows of all the recipes that feed it. */
export function applyFetchAll(): Promise<PortfolioView> {
  return invoke<PortfolioView>("apply_fetch_all");
}

/** Forgets the earlier result of one recipe (a retry that could not even start). */
export function discardFetchRun(name: string): Promise<void> {
  return invoke<void>("discard_fetch_run", { name });
}

export function clearFetchRuns(): Promise<void> {
  return invoke<void>("clear_fetch_runs");
}

/**
 * Moves the data to another folder (`null`: back to the local one). When the
 * folder already has data, this rejects with "FOLDER_HAS_DATA" until it is
 * called again with `useExisting`.
 */
export function setDataFolder(folder: string | null, useExisting: boolean): Promise<PortfolioView> {
  return invoke<PortfolioView>("set_data_folder", { folder, useExisting });
}

/** The folder iCloud Drive offers for this app, or `null` when iCloud Drive is not set up. */
export function icloudDataFolder(): Promise<string | null> {
  return invoke<string | null>("icloud_data_folder");
}
