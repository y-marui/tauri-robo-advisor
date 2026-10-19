// Mirrors the JSON of the Rust `portfolio` crate. Amounts and weights are
// decimal strings: they are never converted to numbers, so nothing is rounded.

export type ProductTargeting = "free" | "equal" | "keep_current" | "weighted";
export type AccountKind =
  | "taxable"
  | "nisa"
  | "ideco"
  | "points"
  | "crypto"
  | "deposit"
  | "other";

export interface Category {
  id: number;
  name: string;
  target_weight: string;
  targeting: ProductTargeting;
}

export interface Product {
  id: number;
  name: string;
  category: number;
  currency: string;
  aliases: { source: string; name: string }[];
}

export interface Account {
  id: number;
  name: string;
  kind: AccountKind;
  sellable: boolean;
  /** `null` means the account can buy any product. */
  tradable: number[] | null;
  rounding_unit: string;
}

export interface Holding {
  account: number;
  product: number;
  value: string;
  quantity: string | null;
  /** The price the value was worked out from, per `price_unit` units; null when not known. */
  price: string | null;
  price_unit: number;
  /** Relative weight of this record inside its category; null when none is set. */
  weight: string | null;
}

/** Which columns of a source's table hold the name and the value (counted from 0). */
export interface ImportMapping {
  source: string;
  name_column: number;
  /** The value column, or the quantity column when `price_column` is set. */
  value_column: number;
  /** When set, the value is quantity x price / `price_unit`. */
  price_column: number | null;
  price_unit: number;
  header_rows: number;
  rows_per_record: number;
  name_row: number;
  value_row: number;
  /** Which value of a cell that holds several (counted from 0); null: the whole cell. */
  name_part?: number | null;
  value_part?: number | null;
  price_part?: number | null;
  /** Read the first number of a cell that has other text around it ("123.45US$"). */
  loose_numbers?: boolean;
  /** A quantity column to record next to an evaluation column. */
  quantity_column?: number | null;
  quantity_part?: number | null;
  /** Read at most this many records from the top. */
  max_records?: number | null;
}

/** One link of a recipe, by its visible text. */
export interface FetchLink {
  text: string;
  /** Where the link goes (without the parts that change per visit); filled in the first time. */
  reference: string | null;
  /** A menu entry the page handles by its own script (vetted sites only): the text of its menu group. It is clicked, not followed. */
  menu?: string | null;
  /** For a menu entry: the first heading the page must show after the click. */
  heading?: string | null;
}

/** A saved way to fetch one table after the login. */
export interface Recipe {
  name: string;
  links: FetchLink[];
  table_starts_with: string;
  /** Read every table that starts with the title and put their rows together. */
  all_tables?: boolean;
  /** The matching table is a heading and each holding is a table of its own after it. */
  following_tables?: boolean;
  /** The site of the page the recipe reads; empty: the page chosen on the screen. */
  start_site?: string;
  /** Sites whose other hosts the recipe may follow links to (only those containing the host it starts on count). */
  allowed_sites?: string[];
  currency: string;
  account: number;
  mapping: ImportMapping;
}

/** What opening the login pages did. */
export interface LoginPages {
  opened: string[];
  already_open: string[];
  unknown: string[];
}

/** A recipe that ships with the app. */
export interface PresetInfo {
  id: string;
  label: string;
  description: string;
  /** What identifies the recipe of the preset (the same fields the backend matches on). */
  table_starts_with: string;
  source: string;
  links: string[];
}

export interface Settings {
  usd_jpy: string | null;
  category_tolerance: string;
  product_tolerance: string;
  target_account: number | null;
}

export interface Portfolio {
  schema_version: number;
  next_id: number;
  categories: Category[];
  products: Product[];
  accounts: Account[];
  holdings: Holding[];
  settings: Settings;
  import_mappings: ImportMapping[];
  /** Counts the accepted edits; a copy with a lower number is stale. */
  revision: number;
  recipes: Recipe[];
}

export type Issue =
  | { type: "category_weights_do_not_sum_to_one"; sum: string }
  | { type: "weighted_product_missing_weight"; category: number }
  | { type: "category_has_no_products"; category: number }
  | { type: "missing_exchange_rate"; currency: string };

/** Where the data lives and what the folder shows. */
export interface DataInfo {
  folder: string;
  /** `true` when no folder was chosen and the data is in the app's own local folder. */
  is_local: boolean;
  /** Other copies of the data file made by the sync (for example "portfolio 2.json"). */
  conflict_copies: string[];
  /** The data file has not been downloaded to this computer yet. */
  waiting_for_download: boolean;
  /** Why a newer file on disk could not be used. */
  warning: string | null;
}

export interface PortfolioView {
  portfolio: Portfolio;
  issues: Issue[];
  data: DataInfo;
}

export type Edit =
  | { type: "add_category"; name: string; target_weight: string }
  | {
      type: "update_category";
      id: number;
      name?: string;
      target_weight?: string;
      targeting?: ProductTargeting;
    }
  | { type: "remove_category"; id: number }
  | { type: "add_product"; name: string; category: number; currency: string }
  | {
      type: "update_product";
      id: number;
      name?: string;
      category?: number;
      currency?: string;
    }
  | { type: "set_holding_weight"; account: number; product: number; weight: string | null }
  | { type: "remove_product"; id: number }
  | { type: "add_account"; name: string; kind: AccountKind }
  | {
      type: "update_account";
      id: number;
      name?: string;
      kind?: AccountKind;
      sellable?: boolean;
      rounding_unit?: string;
    }
  | { type: "set_tradable"; id: number; products: number[] | null }
  | { type: "remove_account"; id: number }
  | { type: "move_account"; id: number; up: boolean }
  | {
      type: "set_holding";
      account: number;
      product: number;
      value: string;
      quantity: string | null;
    }
  | { type: "remove_holding"; account: number; product: number }
  | { type: "replace_holdings"; account: number; holdings: { product: number; value: string; quantity?: string | null; price?: string | null; price_unit?: number }[] }
  | {
      type: "set_import_mapping";
      source: string;
      name_column: number;
      value_column: number;
      price_column: number | null;
      price_unit: number;
      header_rows: number;
      rows_per_record: number;
      name_row: number;
      value_row: number;
      name_part?: number | null;
      value_part?: number | null;
      price_part?: number | null;
      loose_numbers?: boolean;
      quantity_column?: number | null;
      quantity_part?: number | null;
      max_records?: number | null;
    }
  | { type: "remove_import_mapping"; source: string }
  | { type: "set_recipe"; recipe: Recipe }
  | { type: "remove_recipe"; name: string }
  | { type: "add_preset_recipe"; preset: string; account: number }
  | { type: "add_alias"; product: number; source: string; name: string }
  | { type: "add_product_with_alias"; name: string; category: number; currency: string; source: string; alias: string }
  | { type: "remove_alias"; product: number; source: string; name: string }
  | { type: "set_exchange_rate"; usd_jpy: string | null }
  | { type: "set_tolerances"; category: string; product: string }
  | { type: "set_target_account"; account: number | null };

/** One category's value and share, next to its target. Mirrors `suggest::Overview`. */
export interface CategoryOverview {
  category: number;
  /** JPY. */
  current: string;
  /** Fractions, e.g. "0.7778". */
  current_share: string;
  target_share: string;
  /** Current share minus target share; positive when over target. */
  difference: string;
}

/** One holding record valued in JPY. */
export interface HoldingOverview {
  account: number;
  product: number;
  value: string;
  share: string;
}

export interface Overview {
  total: string;
  categories: CategoryOverview[];
  holdings: HoldingOverview[];
}

export type SuggestMode =
  | { type: "fixed_add"; amount: string }
  | { type: "buy_only_to_target" }
  | { type: "full_rebalance" }
  | { type: "fixed_sell"; amount: string };

export interface SuggestRequest {
  account: number;
  mode: SuggestMode;
}

export interface CategoryLine {
  category: number;
  current: string;
  /** Positive to buy, negative to sell. */
  delta: string;
  after: string;
  target_after: string;
}

export interface ProductLine {
  product: number;
  delta: string;
}

export type Note =
  | { type: "amount_rounded"; requested: string; applied: string }
  | { type: "category_not_filled"; category: number; shortfall: string }
  | { type: "already_balanced" };

export interface Suggestion {
  mode: SuggestMode["type"];
  account: number;
  requested: string | null;
  applied: string;
  required_total: string | null;
  total_after: string;
  categories: CategoryLine[];
  products: ProductLine[];
  notes: Note[];
}

/** One product in one account against its weight. Mirrors `suggest::AccountBalance`. */
export interface ProductBalance {
  product: number;
  /** JPY held in the account. */
  current: string;
  /** Whole yen the weights ask for; `null` when the category asks for no split. */
  target: string | null;
  /** `target` minus `current`: positive when short, negative when over. */
  shortfall: string | null;
}

export interface CategoryBalance {
  category: number;
  total: string;
  products: ProductBalance[];
}

export interface AccountBalance {
  account: number;
  categories: CategoryBalance[];
}

/** A name a source uses and its value. Mirrors `portfolio::ImportRow`. */
export interface ImportRow {
  name: string;
  value: string;
  quantity?: string | null;
  price?: string | null;
  price_unit?: number;
}

export interface MatchedRow {
  names: string[];
  product: number;
  value: string;
  /** Added up when every row had one. */
  quantity: string | null;
  /** Kept when every row had the same price. */
  price: string | null;
  price_unit: number;
}

export interface ImportPlan {
  matched: MatchedRow[];
  unmatched: ImportRow[];
  wrong_currency: ImportRow[];
}

/** What the bitFlyer command returns: balances in JPY, and what could not be valued. */
export interface BitflyerHoldings {
  holdings: { symbol: string; value: string; /** How much of the currency is held. */ amount: string }[];
  unpriced: string[];
}

/** The top of a table read for an import. Mirrors `TablePreview` in Rust. */
export interface TablePreview {
  rows: string[][];
  total_rows: number;
  columns: number;
  encoding: "utf8" | "shift_jis" | null;
  delimiter: "comma" | "tab" | null;
}

export interface ColumnMapping {
  name: number;
  value: number;
  price: number | null;
  price_unit: number;
  header_rows: number;
  rows_per_record: number;
  name_row: number;
  value_row: number;
  name_part?: number | null;
  value_part?: number | null;
  price_part?: number | null;
  loose_numbers?: boolean;
  quantity?: number | null;
  quantity_part?: number | null;
}

export type SkipReason = "missing_column" | "no_name" | "not_a_number" | "out_of_range";

export interface Skipped {
  /** The row's position in the table, counted from 1. */
  row: number;
  reason: SkipReason;
}

export interface TableImportPlan {
  plan: ImportPlan;
  skipped: Skipped[];
  rows_read: number;
}

export type BrowserKind = "chrome" | "edge" | "brave";

export interface BrowserInfo {
  kind: BrowserKind;
}

/** An open tab: the host and the title only. */
export interface BrowserPage {
  id: string;
  host: string;
  title: string;
}

/** A table found on a page. */
export interface FetchedTable {
  index: number;
  rows: number;
  columns: number;
  label: string;
}

/** One recipe's result. Mirrors `RecipeRunView` in Rust. */
export interface RecipeRun {
  name: string;
  account: number;
  source: string;
  currency: string;
  plan: ImportPlan;
  skipped: Skipped[];
  rows_read: number;
}

export type RecipeOutcome =
  | { status: "ready"; run: RecipeRun }
  | { status: "needs_choice"; link: number; choices: { reference: string; label: string }[] }
  | { status: "link_changed"; link: number };
