// Mirrors the JSON of the Rust `portfolio` crate. Amounts and weights are
// decimal strings: they are never converted to numbers, so nothing is rounded.

/** The suggestion modes; each keeps its own product targeting (`Portfolio.product_shares`). */
export type SuggestionMode = "fixed_add" | "buy_only_to_target" | "full_rebalance" | "fixed_sell";
/** How a product's target inside its category is held. Mirrors `portfolio::ProductShare`. */
export type ProductShare =
  | { kind: "equal" }
  | { kind: "weighted"; weight: string }
  | { kind: "keep_current" };
/** An explicit setting of a product for a mode; a product without one has an equal share. */
export interface ProductShareEntry {
  mode: SuggestionMode;
  product: number;
  share: ProductShare;
}
/** A built-in account type id, or the id of a custom type (`custom-<n>`). */
export type AccountTypeId = string;

/** An account type the user added. */
export interface CustomType {
  id: string;
  name: string;
  sellable: boolean;
}

/** A main category above the categories. It has no target of its own. */
export interface Group {
  id: number;
  name: string;
}

export interface Category {
  id: number;
  name: string;
  /** The group the category belongs to; absent when it is ungrouped. */
  group?: number | null;
  target_weight: string;
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
  sellable: boolean;
  /** Products that do not take part in trading in this account: suggestions never buy or sell them. */
  excluded?: number[];
  /** The same for the All tab's suggestions. */
  excluded_all?: number[];
  /** What suggestions aim at; absent: follow the portfolio. */
  strategy?: Strategy;
  rounding_unit: string;
}

/** Mirrors `portfolio::Strategy`. A blank ratio means an equal share. */
export type Strategy =
  | { type: "portfolio" }
  | { type: "products"; items: { product: number; ratio: string | null }[] };

export interface Holding {
  account: number;
  product: number;
  value: string;
  quantity: string | null;
  /** The price the value was worked out from, per `price_unit` units; null when not known. */
  price: string | null;
  price_unit: number;
  /** The type of the record; absent when not set. A record is (account, product, type). */
  account_type?: AccountTypeId | null;
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
  /** An evaluation column read next to the quantity and the price; quantity x price checks it. */
  evaluation_column?: number | null;
  evaluation_part?: number | null;
  /** Read the first number of a cell that has other text around it ("123.45US$"). */
  loose_numbers?: boolean;
  /** A quantity column to record next to an evaluation column. */
  quantity_column?: number | null;
  quantity_part?: number | null;
  /** Read at most this many records from the top. */
  max_records?: number | null;
  /** A cell of the name row that says the kind of a holding, and how each kind is read. */
  kind_column?: number | null;
  kind_rules?: KindRule[];
}

export interface KindRule {
  kind: string;
  read: { type: "quantity_times_price"; price_unit: number } | { type: "evaluation"; column: number; part?: number | null };
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

/** Where a recipe finds repeated blocks (cards) instead of a table. */
export interface CardSource {
  heading: string;
  /** How many levels below the region after the heading the items are (1 to 3). */
  item_depth: number;
}

/** A saved way to fetch one table after the login. */
export interface Recipe {
  name: string;
  links: FetchLink[];
  table_starts_with: string;
  /** Read the cards under a heading instead of a table (then `table_starts_with` is empty). */
  cards?: CardSource | null;
  /** Read every table that starts with the title and put their rows together. */
  all_tables?: boolean;
  /** The matching table is a heading and each holding is a table of its own after it. */
  following_tables?: boolean;
  /** The site of the page the recipe reads; empty: the page chosen on the screen. */
  start_site?: string;
  /** Sites whose other hosts the recipe may follow links to (only those containing the host it starts on count). */
  allowed_sites?: string[];
  currency: string;
  /** What the title of each table says about the type of its records (then `all_tables` is on). */
  type_rules?: { title: string; account_type: AccountTypeId }[];
  account: number;
  mapping: ImportMapping;
  /** The id of the built-in preset this recipe refers to (its definition is the app's current preset). */
  preset?: string | null;
}

/** A recipe as it is run: for a preset recipe, the definition of the preset the app has now. */
export interface RecipeView extends Recipe {
  /** The preset it refers to does not exist in this version of the app (the definition is empty). */
  preset_missing: boolean;
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
  /** The site the preset reads (empty: any page) and the company's name, for grouping. */
  site: string;
  company: string;
}

export interface Settings {
  usd_jpy: string | null;
  category_tolerance: string;
  product_tolerance: string;
}

export interface Portfolio {
  schema_version: number;
  next_id: number;
  groups: Group[];
  categories: Category[];
  products: Product[];
  accounts: Account[];
  holdings: Holding[];
  settings: Settings;
  import_mappings: ImportMapping[];
  /** Counts the accepted edits; a copy with a lower number is stale. */
  revision: number;
  recipes: Recipe[];
  /** Absent when there is none. */
  custom_types?: CustomType[];
  /** Absent when every product has an equal share in every mode. */
  product_shares?: ProductShareEntry[];
}

export type Issue =
  | { type: "category_weights_do_not_sum_to_one"; sum: string }
  | { type: "missing_exchange_rate"; currency: string };

/** Where the data lives and what the folder shows. */
export interface DataInfo {
  folder: string;
  /** `true` when no folder was chosen and the data is in the app's own local folder. */
  is_local: boolean;
  /** Changes when the data folder is switched; sent back with each edit. */
  location: number;
  /** Other copies of the data file made by the sync (for example "portfolio 2.json"). */
  conflict_copies: string[];
  /** The data file has not been downloaded to this computer yet. */
  waiting_for_download: boolean;
  /** Why a newer file on disk could not be used. */
  warning: string | null;
}

export interface PortfolioView {
  portfolio: Portfolio;
  /**
   * Each holding's value rounded to the smallest unit of its currency, for display only. The
   * holding's own `value` stays exact (edit fields show it as it is).
   */
  shown_values: { account: number; product: number; account_type: AccountTypeId | null; value: string }[];
  /** The recipes as they are run (`portfolio.recipes` holds what is stored). */
  recipes: RecipeView[];
  issues: Issue[];
  data: DataInfo;
}

export type Edit =
  | { type: "add_group"; name: string }
  | { type: "rename_group"; id: number; name: string }
  | { type: "remove_group"; id: number }
  | { type: "set_category_group"; category: number; group: number | null }
  | { type: "add_category"; name: string; target_weight: string }
  | {
      type: "update_category";
      id: number;
      name?: string;
      target_weight?: string;
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
  | { type: "set_product_share"; mode: SuggestionMode; product: number; share: ProductShare }
  | { type: "add_account_type"; name: string; sellable: boolean }
  | { type: "update_account_type"; id: string; name?: string; sellable?: boolean }
  | { type: "remove_account_type"; id: string }
  | {
      type: "set_record_type";
      account: number;
      product: number;
      from: AccountTypeId | null;
      to: AccountTypeId | null;
    }
  | { type: "remove_product"; id: number }
  | { type: "add_account"; name: string }
  | {
      type: "update_account";
      id: number;
      name?: string;
      sellable?: boolean;
      rounding_unit?: string;
    }
  | { type: "set_excluded"; id: number; products: number[]; all?: boolean }
  | { type: "set_strategy"; id: number; strategy: Strategy }
  | { type: "remove_account"; id: number }
  | { type: "move_account"; id: number; up: boolean }
  | { type: "move_group"; id: number; up: boolean }
  | { type: "move_category"; id: number; up: boolean }
  | { type: "move_account_to"; id: number; index: number }
  | { type: "move_group_to"; id: number; index: number }
  | { type: "move_category_to"; id: number; index: number }
  | {
      type: "set_holding";
      account: number;
      product: number;
      account_type?: AccountTypeId | null;
      value: string;
      quantity: string | null;
    }
  | { type: "remove_holding"; account: number; product: number; account_type?: AccountTypeId | null }
  | { type: "replace_holdings"; account: number; types?: (AccountTypeId | null)[] | null; holdings: { product: number; account_type?: AccountTypeId | null; value: string; quantity?: string | null; price?: string | null; price_unit?: number }[] }
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
      kind_column?: number | null;
      kind_rules?: KindRule[];
    }
  | { type: "remove_import_mapping"; source: string }
  | { type: "set_recipe"; recipe: Recipe }
  | { type: "remove_recipe"; name: string }
  | { type: "add_preset_recipe"; preset: string; account: number }
  | { type: "detach_preset_recipe"; name: string }
  | { type: "set_recipe_account"; name: string; account: number }
  | { type: "add_alias"; product: number; source: string; name: string }
  | { type: "add_product_with_alias"; name: string; category: number; currency: string; source: string; alias: string }
  | { type: "set_products_category"; products: number[]; category?: number; new_category?: string }
  | { type: "merge_products"; keep: number; remove: number; on_conflict: "keep" | "take" }
  | { type: "split_product"; product: number; source: string; alias: string; name: string; accounts: number[] }
  | { type: "remove_alias"; product: number; source: string; name: string }
  | { type: "set_exchange_rate"; usd_jpy: string | null }
  | { type: "set_tolerances"; category: string; product: string };

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

/**
 * One group's value and share, next to its target (the sum of its categories' targets).
 * `group` is `null` for the categories that belong to no group.
 */
export interface GroupOverview {
  group: number | null;
  current: string;
  current_share: string;
  target_share: string;
  difference: string;
}

/** One holding record valued in JPY. */
export interface HoldingOverview {
  account: number;
  product: number;
  account_type: AccountTypeId | null;
  value: string;
  share: string;
}

export interface AccountOverview {
  account: number;
  /** The value of the account's holdings in JPY. */
  value: string;
  share: string;
}

export interface Overview {
  total: string;
  /** Empty when no group exists. */
  groups: GroupOverview[];
  categories: CategoryOverview[];
  holdings: HoldingOverview[];
  accounts: AccountOverview[];
  /** Only types that have records; ordered by id, records without a type first. */
  types: { account_type: AccountTypeId | null; value: string; share: string }[];
}

export type SuggestMode =
  | { type: "fixed_add"; amount: string }
  | { type: "buy_only_to_target" }
  | { type: "full_rebalance" }
  | { type: "fixed_sell"; amount: string };

export interface SuggestRequest {
  /** The account to trade in; `null` trades across every account. */
  account: number | null;
  mode: SuggestMode;
  /** Only buy products the account holds. Omitted means false. */
  held_only?: boolean;
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
  account: number;
  product: number;
  delta: string;
}

export type Note =
  | { type: "amount_rounded"; requested: string; applied: string }
  | { type: "category_not_filled"; category: number; shortfall: string }
  | { type: "product_not_filled"; product: number; shortfall: string }
  | { type: "already_balanced" };

export interface Suggestion {
  mode: SuggestMode["type"];
  /** `null` when the suggestion spans every account. */
  account: number | null;
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

/** An account with a product strategy: its products against their ratios. */
export interface StrategyBalance {
  total: string;
  products: ProductBalance[];
}

export interface AccountBalance {
  account: number;
  categories: CategoryBalance[];
  /** Present for an account with a product strategy (the categories are then empty). */
  strategy?: StrategyBalance;
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
  /** The type the table title gave the rows; null: none. */
  account_type: AccountTypeId | null;
}

export interface ImportPlan {
  matched: MatchedRow[];
  unmatched: ImportRow[];
  wrong_currency: ImportRow[];
  /** Rows whose evaluation differs from quantity x price by more than the smallest currency unit. */
  differences: ValueDifference[];
  /** Hints for unmatched names that have a product of the same or a similar name. */
  suggestions?: LinkSuggestion[];
}

/** Mirrors `portfolio::LinkSuggestion`: only a hint, never a link. */
export interface LinkSuggestion {
  /** The trimmed name of the unmatched row. */
  name: string;
  /** A product with the same name (trimmed, ignoring case). */
  exact: number | null;
  /** Else a product with a similar name. */
  similar: number | null;
}

/** A row whose read value and computed value (quantity x price / unit) disagree. */
export interface ValueDifference {
  name: string;
  value: string;
  computed: string;
}

/** What the bitFlyer command returns: balances in JPY, and what could not be valued. */
export interface BitflyerHoldings {
  holdings: {
    symbol: string;
    value: string;
    /** How much of the currency is held. */
    amount: string;
    /** The yen price from the price page the value was worked out with (page prices only). */
    price?: string | null;
    /** The Lightning last price, for comparison only. */
    api_price?: string | null;
    /** (page price - API price) / API price in percent, for reference. */
    difference_percent?: string | null;
  }[];
  unpriced: string[];
}

/** What the Coincheck command returns: balances in JPY, and what could not be valued. */
export interface CoincheckHoldings {
  holdings: {
    symbol: string;
    value: string;
    /** How much of the currency is held. */
    amount: string;
    /** The yen price (the sales rate) the value was worked out with. */
    price?: string | null;
  }[];
  unpriced: string[];
}

/** What the GMO Coin command returns: balances in JPY, with the basis of each price. */
export interface GmocoinHoldings {
  holdings: {
    symbol: string;
    value: string;
    /** How much of the currency is held. */
    amount: string;
    /** The yen price the value was worked out with. */
    price?: string | null;
    /** "sales": the sales desk's selling price; "board": the exchange's last price (no sales desk price). */
    basis?: "sales" | "board" | null;
  }[];
  unpriced: string[];
}

/** What the Binance command returns: balances in JPY, with the basis of each price. */
export interface BinanceHoldings {
  holdings: {
    symbol: string;
    value: string;
    /** How much of the currency is held. */
    amount: string;
    /** The yen price the value was worked out with. */
    price?: string | null;
    /** "board": the JPY market's last price; "board_via_usdt": the USDT market's last price times USDT/JPY. */
    basis?: "board" | "board_via_usdt" | null;
  }[];
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

export type SkipReason = "missing_column" | "no_name" | "not_a_number" | "out_of_range" | "unknown_kind";

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

/** The browser and profile folder last used, as the app kept them. */
export interface BrowserSettings {
  kind: BrowserKind | null;
  /** Empty: the default folder inside the app's data folder. */
  profile: string;
}

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
  /** Set when the candidate is a card list (one row per card): where to find it again. */
  card: CardSource | null;
}

/** What reading a page offered: its tables and card lists. */
export interface FetchedTables {
  tables: FetchedTable[];
  /** The page has more headings than the app reads: card lists below them are not offered. */
  headings_cut: boolean;
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
