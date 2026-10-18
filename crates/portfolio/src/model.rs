use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// The on-disk schema version. A file with a newer version is refused rather
/// than guessed at, so an older app never rewrites data it does not understand.
/// Version 2 added the saved import mappings, the revision, and the fetch recipes
/// (all with defaults, so a version 1 file loads and is saved as version 2).
pub const SCHEMA_VERSION: u32 = 2;

macro_rules! id_type {
    ($name:ident) => {
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(pub u64);
    };
}

id_type!(CategoryId);
id_type!(ProductId);
id_type!(AccountId);

/// How the products inside a category are targeted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProductTargeting {
    /// Only the category has a target; products are not constrained.
    #[default]
    Free,
    /// Every product in the category gets the same share.
    Equal,
    /// Products keep their current proportions.
    KeepCurrent,
    /// Products have explicit relative weights (`Product::weight`).
    Weighted,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Category {
    pub id: CategoryId,
    pub name: String,
    /// Share of the whole portfolio as a fraction of 1.
    pub target_weight: Decimal,
    #[serde(default)]
    pub targeting: ProductTargeting,
}

/// A name a data source uses for a product, linked by the user.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Alias {
    pub source: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Product {
    pub id: ProductId,
    pub name: String,
    pub category: CategoryId,
    /// Three upper-case letters, for example `JPY` or `USD`.
    pub currency: String,
    /// Relative weight inside the category; used when it is `Weighted`.
    #[serde(default)]
    pub weight: Option<Decimal>,
    #[serde(default)]
    pub aliases: Vec<Alias>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountKind {
    Taxable,
    Nisa,
    Ideco,
    Points,
    Crypto,
    Deposit,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Funding {
    Cash,
    Points,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Account {
    pub id: AccountId,
    pub name: String,
    pub kind: AccountKind,
    /// Whether suggestions may propose selling in this account.
    pub sellable: bool,
    /// Products that can be bought here; `None` means any product.
    #[serde(default)]
    pub tradable: Option<Vec<ProductId>>,
    pub funding: Funding,
    /// Suggested amounts are rounded to a multiple of this value.
    pub rounding_unit: Decimal,
}

/// A product held in an account. `value` is in the product's own currency.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Holding {
    pub account: AccountId,
    pub product: ProductId,
    pub value: Decimal,
    #[serde(default)]
    pub quantity: Option<Decimal>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    /// Manually entered USD/JPY rate.
    #[serde(default)]
    pub usd_jpy: Option<Decimal>,
    /// Deviation from a category target tolerated before suggesting a trade.
    pub category_tolerance: Decimal,
    /// Deviation from a product target tolerated before suggesting a trade.
    pub product_tolerance: Decimal,
    /// The account used for whole-portfolio rebalancing.
    #[serde(default)]
    pub target_account: Option<AccountId>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            usd_jpy: None,
            category_tolerance: Decimal::new(1, 2),
            product_tolerance: Decimal::ZERO,
            target_account: None,
        }
    }
}

/// Which columns of a source's table hold the name and the value, saved so the
/// next import from the same source needs no setup. Columns count from 0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportMapping {
    pub source: String,
    pub name_column: usize,
    /// The value column, or the quantity column when `price_column` is set.
    pub value_column: usize,
    /// When set, the value is quantity x price / `price_unit`.
    #[serde(default)]
    pub price_column: Option<usize>,
    #[serde(default = "one")]
    pub price_unit: u64,
    #[serde(default)]
    pub header_rows: usize,
    /// How many table rows make one record, and which of them holds the name
    /// and which the numbers (counted from 0).
    #[serde(default = "one_row")]
    pub rows_per_record: usize,
    #[serde(default)]
    pub name_row: usize,
    #[serde(default)]
    pub value_row: usize,
}

/// One link to follow, by its visible text. `reference` says which of several
/// links with that text is meant (where it goes, without the parts that change
/// per visit); the app fills it in the first time the link is followed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FetchLink {
    pub text: String,
    #[serde(default)]
    pub reference: Option<String>,
}

/// A saved way to fetch one table after the user has logged in: follow these
/// links, find the table whose first cell starts with `table_starts_with`, read
/// it with `mapping`, and put the result into `account`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recipe {
    pub name: String,
    #[serde(default)]
    pub links: Vec<FetchLink>,
    pub table_starts_with: String,
    pub currency: String,
    pub account: AccountId,
    pub mapping: ImportMapping,
}

fn one() -> u64 {
    1
}

fn one_row() -> usize {
    1
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Portfolio {
    pub schema_version: u32,
    /// The next unused id; ids are never reused, so a stale reference can
    /// never silently point at a different item.
    pub next_id: u64,
    #[serde(default)]
    pub categories: Vec<Category>,
    #[serde(default)]
    pub products: Vec<Product>,
    #[serde(default)]
    pub accounts: Vec<Account>,
    #[serde(default)]
    pub holdings: Vec<Holding>,
    #[serde(default)]
    pub settings: Settings,
    #[serde(default)]
    pub import_mappings: Vec<ImportMapping>,
    /// Counts the accepted edits. When the file is shared between computers
    /// (a synced folder), a copy with a lower revision is stale and must not
    /// overwrite a newer one.
    #[serde(default)]
    pub revision: u64,
    #[serde(default)]
    pub recipes: Vec<Recipe>,
}

impl Default for Portfolio {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            next_id: 1,
            categories: Vec::new(),
            products: Vec::new(),
            accounts: Vec::new(),
            holdings: Vec::new(),
            settings: Settings::default(),
            import_mappings: Vec::new(),
            revision: 0,
            recipes: Vec::new(),
        }
    }
}
