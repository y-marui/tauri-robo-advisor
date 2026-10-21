use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// The on-disk schema version. A file with a newer version is refused rather
/// than guessed at, so an older app never rewrites data it does not understand.
/// Version 2 added the saved import mappings, the revision, and the fetch recipes.
/// Version 3 added the price (and its unit) of a holding and the recorded quantity column
/// of a mapping. Version 4 added the table-merging flag and the allowed sites of a recipe. Version 5 added the
/// following-tables flag and the start site of a recipe. Version 6 added the record limit of a
/// mapping (`max_records`). Version 7 dropped the funding of an account (cash or points), which
/// nothing used (an older app needs the field, so it refuses). Version 8 moved the relative
/// weight of a product onto the record of the product in an account (an older app would read
/// every weight as missing, so it refuses). Version 9 added the menu click of a link (`menu` and
/// `heading`: an older app would try to follow it as a plain link, so it refuses). Version 10 added
/// the preset id of a recipe (`preset`: a recipe made from a preset carries only its account, and an
/// older app would read it as an empty custom recipe, so it refuses). Version 11 added the groups
/// of categories (`groups`, and the `group` of a category: an older app would drop the grouping on
/// its next save, so it refuses). Version 12 added the card source of a recipe (`cards`:
/// an older app would read a recipe that reads cards as an empty ordinary recipe, so it refuses).
/// Version 13 added the kind column and kind rules of a mapping (`kind_column`, `kind_rules`: an
/// older app would ignore them and read every row of a mixed table the same way, misvaluing some, so
/// it refuses).
/// Version 14 moved the account type from the account to the record (`Holding::account_type`, and
/// the user's own types in `custom_types`; `Account::kind` was dropped: an older app would read every
/// record as untyped and write the old kind back, so it refuses). An older file is converted on
/// load (see `migrate`).
/// Version 15 added the evaluation column of a mapping (`evaluation_column`, `evaluation_part`: an
/// older app would read the value as quantity x price and skip the check against the page's
/// evaluation, so it refuses). Records that were saved before keep working as they are.
/// Version 16 added the products an account leaves out of trading (`excluded`: an older app would
/// suggest trading them, so it refuses). An older file has none excluded.
/// Version 17 dropped the rebalancing account (`settings.target_account`: the All tab now trades
/// across every account, so an older app would still read a target that no longer exists). An older
/// file loses the setting on load (see `migrate`).
/// Everything new has a default, so an older file loads and is saved as the current version,
/// and an older app refuses a newer file instead of dropping its fields.
pub const SCHEMA_VERSION: u32 = 17;

/// The oldest version this app opens. Version 8 moved the weights of products into the records
/// of the accounts, and nothing converts an older file: it is refused and left untouched.
pub const MIN_SCHEMA_VERSION: u32 = 8;

macro_rules! id_type {
    ($name:ident) => {
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(pub u64);
    };
}

id_type!(GroupId);
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
    /// Products have explicit relative weights (`Holding::weight`, added up over the accounts).
    Weighted,
}

/// A main category above the categories. It has no target of its own: its target and its current
/// share are the sums of its categories'.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Group {
    pub id: GroupId,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Category {
    pub id: CategoryId,
    pub name: String,
    /// The group the category belongs to; `None`: ungrouped.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<GroupId>,
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
    #[serde(default)]
    pub aliases: Vec<Alias>,
}

/// The account types that ship with the app: the id (stored in a record) and whether a record of
/// the type may be sold by a suggestion. The display names belong to the screen (translated).
/// NISA and old NISA are not sellable: selling would forfeit tax-free room.
pub const BUILTIN_ACCOUNT_TYPES: [(&str, bool); 8] = [
    ("general", true),
    ("specific", true),
    ("nisa", false),
    ("nisa_growth", false),
    ("nisa_tsumitate", false),
    ("old_nisa", false),
    ("deposit", true),
    ("crypto", true),
];

/// An account type the user added (for manual entry and custom recipes).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomType {
    /// `custom-<n>`, from the id counter, never reused.
    pub id: String,
    pub name: String,
    /// Whether suggestions may propose selling a record of this type.
    pub sellable: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Account {
    pub id: AccountId,
    pub name: String,
    /// Whether suggestions may propose selling in this account (a record is also bound by the
    /// selling rule of its type).
    pub sellable: bool,
    /// Products that can be bought here; `None` means any product.
    #[serde(default)]
    pub tradable: Option<Vec<ProductId>>,
    /// Products suggestions never buy or sell in this account. Their holdings still count toward
    /// the value of their category.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub excluded: Vec<ProductId>,
    /// Suggested amounts are rounded to a multiple of this value.
    pub rounding_unit: Decimal,
}

/// The record of a product in an account: what is held and how much of it is wanted.
/// `value` is in the product's own currency.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Holding {
    pub account: AccountId,
    pub product: ProductId,
    pub value: Decimal,
    #[serde(default)]
    pub quantity: Option<Decimal>,
    /// The price the value was worked out from, in the product's currency, when it is known
    /// (an import of quantity x price). Per `price_unit` units.
    #[serde(default)]
    pub price: Option<Decimal>,
    /// How many units the price is quoted for: 1, or 10,000 for a fund's price per 10,000 units.
    #[serde(default = "one")]
    pub price_unit: u64,
    /// Relative weight of this record inside its category, used when the category is
    /// `Weighted`. A record can have a weight and no value yet (a product to start buying).
    #[serde(default)]
    pub weight: Option<Decimal>,
    /// The custody type of the record (a built-in id such as `nisa`, or a custom type's id); `None`:
    /// not set. The record is identified by the account, the product, and this type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account_type: Option<String>,
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
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            usd_jpy: None,
            category_tolerance: Decimal::new(1, 2),
            product_tolerance: Decimal::ZERO,
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
    /// When set, the value is quantity x price / `price_unit` (or, with `evaluation_column`, the
    /// evaluation, which quantity x price checks).
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
    /// Which value of a cell that holds several (counted from 0); `None`: the whole cell.
    #[serde(default)]
    pub name_part: Option<usize>,
    #[serde(default)]
    pub value_part: Option<usize>,
    #[serde(default)]
    pub price_part: Option<usize>,
    /// An evaluation column (and part) read next to the quantity (the value column) and the price:
    /// the evaluation is then the value of the record, and quantity x price only checks it (a
    /// difference is shown on the confirmation screen). Needs `price_column`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evaluation_column: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evaluation_part: Option<usize>,
    /// Read the first number of a cell that has other text around it ("123.45US$").
    #[serde(default)]
    pub loose_numbers: bool,
    /// A quantity column to record when the value is read from an evaluation column (when
    /// the value is quantity x price, the value column is the quantity).
    #[serde(default)]
    pub quantity_column: Option<usize>,
    #[serde(default)]
    pub quantity_part: Option<usize>,
    /// Read at most this many records from the top (after the heading rows): for a table that
    /// lists several figures of which only the first is wanted (a cash balance above the totals).
    #[serde(default)]
    pub max_records: Option<usize>,
    /// A cell of the name row that says what kind of holding a row is (a table, or a card, that
    /// lists stocks and funds together), and how each kind is read. A row of any other kind is
    /// left out and reported. `None` and no rules: every row is read the same way.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind_column: Option<usize>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub kind_rules: Vec<KindRule>,
}

/// How the rows of one kind are read (see `ImportMapping::kind_column`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KindRule {
    /// The text of the kind cell, compared whole (line breaks and runs of spaces as one space).
    pub kind: String,
    pub read: KindRead,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum KindRead {
    /// The value is quantity (the value column) x price (the price column) / `price_unit`.
    QuantityTimesPrice { price_unit: u64 },
    /// The value is read from this column (and part of its cell); the value column is then the
    /// quantity, which is recorded next to it. With a price column in the mapping, the price is
    /// recorded as well (per unit) and quantity x price checks the value.
    Evaluation {
        column: usize,
        #[serde(default)]
        part: Option<usize>,
    },
}

/// One link to follow, by its visible text. `reference` says which of several
/// links with that text is meant (where it goes, without the parts that change
/// per visit); the app fills it in the first time the link is followed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FetchLink {
    pub text: String,
    #[serde(default)]
    pub reference: Option<String>,
    /// For a menu entry that the page handles by its own script (a site that works by form posts):
    /// the text of the heading link of the menu group the entry is in. The entry is then clicked,
    /// not followed (only on vetted sites; see `heading`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub menu: Option<String>,
    /// For a menu entry: the first heading the page must show after the click, otherwise the
    /// batch stops.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heading: Option<String>,
}

/// Where a recipe finds repeated blocks ("cards") instead of a table: under the one visible
/// heading that starts with `heading`, each block `item_depth` levels below the region after it
/// is one item. An item's text lines are its cells, so `ImportMapping` names the parts as for a
/// table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardSource {
    pub heading: String,
    /// How many levels below the region the items are (1 to 3).
    pub item_depth: u8,
}

/// What a table title says about the type of the records under it: a table whose title starts
/// with `title` (compared without whitespace) holds records of `account_type`. The person (or a
/// preset) names each title; a title is never guessed. The longest matching `title` wins.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeRule {
    pub title: String,
    /// A built-in type id or the id of a custom type.
    pub account_type: String,
}

/// A saved way to fetch one table after the user has logged in: follow these
/// links, find the table whose first cell starts with `table_starts_with`, read
/// it with `mapping`, and put the result into `account`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recipe {
    pub name: String,
    #[serde(default)]
    pub links: Vec<FetchLink>,
    #[serde(default)]
    pub table_starts_with: String,
    /// Read every table whose first cell starts with `table_starts_with` (one table per kind of
    /// account on a page) and put their rows together; off: exactly one table must match.
    #[serde(default)]
    pub all_tables: bool,
    /// The matching table is a heading, and each holding is a table of its own after it (a page
    /// laid out one table per row): the tables after it that have the same number of columns are
    /// read as its rows.
    #[serde(default)]
    pub following_tables: bool,
    /// Read repeated blocks under a heading instead of a table. Exclusive with
    /// `table_starts_with`; `all_tables` and `following_tables` are then off.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cards: Option<Box<CardSource>>,
    /// The site (like `example.co.jp`) of the page the recipe reads: with several brokerages open,
    /// "fetch everything" reads each recipe from the open page of its own site. Empty: the page
    /// that was chosen on the screen.
    #[serde(default)]
    pub start_site: String,
    /// Sites (like `example.co.jp`) whose other hosts the recipe may follow links to. Only a site
    /// that contains the host the recipe starts on counts: the recipe can reach the other hosts
    /// of its own company, never another site.
    #[serde(default)]
    pub allowed_sites: Vec<String>,
    #[serde(default)]
    pub currency: String,
    /// What the title of each table of a page (read with `all_tables`) says about the type of its
    /// records. With rules, a record gets the type of its table, and an import replaces only the
    /// types the rules name (a table with another title is read without a type, and replaces the
    /// records without a type). No rules: records are read without a type and the import replaces
    /// the whole account.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub type_rules: Vec<TypeRule>,
    pub account: AccountId,
    #[serde(default)]
    pub mapping: ImportMapping,
    /// The id of a built-in preset (see `presets`). A recipe made from a preset is stored with only
    /// its name, this id, the account, and the remembered destinations of its links: the rest of
    /// the definition is read from the app's current preset data (`Recipe::effective`), so an app
    /// update updates it. `None`: an ordinary recipe that holds its own definition.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preset: Option<String>,
}

impl Default for ImportMapping {
    fn default() -> Self {
        Self {
            source: String::new(),
            name_column: 0,
            value_column: 0,
            price_column: None,
            price_unit: 1,
            header_rows: 0,
            rows_per_record: 1,
            name_row: 0,
            value_row: 0,
            name_part: None,
            value_part: None,
            price_part: None,
            evaluation_column: None,
            evaluation_part: None,
            loose_numbers: false,
            quantity_column: None,
            quantity_part: None,
            max_records: None,
            kind_column: None,
            kind_rules: Vec::new(),
        }
    }
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
    pub groups: Vec<Group>,
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub custom_types: Vec<CustomType>,
}

impl Default for Portfolio {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            next_id: 1,
            groups: Vec::new(),
            categories: Vec::new(),
            products: Vec::new(),
            accounts: Vec::new(),
            holdings: Vec::new(),
            settings: Settings::default(),
            import_mappings: Vec::new(),
            revision: 0,
            recipes: Vec::new(),
            custom_types: Vec::new(),
        }
    }
}

impl Portfolio {
    /// Whether an account type id is a built-in one or one of the user's.
    pub fn has_account_type(&self, id: &str) -> bool {
        BUILTIN_ACCOUNT_TYPES.iter().any(|(b, _)| *b == id)
            || self.custom_types.iter().any(|t| t.id == id)
    }

    /// Whether a record of this type may be sold (not set: yes).
    pub fn type_sellable(&self, id: Option<&str>) -> bool {
        let Some(id) = id else { return true };
        BUILTIN_ACCOUNT_TYPES
            .iter()
            .find(|(b, _)| *b == id)
            .map(|(_, sellable)| *sellable)
            .or_else(|| {
                self.custom_types
                    .iter()
                    .find(|t| t.id == id)
                    .map(|t| t.sellable)
            })
            .unwrap_or(true)
    }

    /// The relative weight of a product over the whole portfolio: the weights of its records
    /// added up. `None` when no record of it has a weight.
    pub fn product_weight(&self, product: ProductId) -> Option<Decimal> {
        self.holdings
            .iter()
            .filter(|h| h.product == product)
            .filter_map(|h| h.weight)
            // Stored weights are only checked to be positive, so the sum saturates rather than
            // overflowing (the calculations refuse a weight above their own limit).
            .reduce(|sum, w| sum.checked_add(w).unwrap_or(Decimal::MAX))
    }

    /// The relative weight of the record of a product in one account (the weights of its records
    /// of the several types added up).
    pub fn record_weight(&self, account: AccountId, product: ProductId) -> Option<Decimal> {
        self.holdings
            .iter()
            .filter(|h| h.account == account && h.product == product)
            .filter_map(|h| h.weight)
            .reduce(|sum, w| sum.checked_add(w).unwrap_or(Decimal::MAX))
    }
}
