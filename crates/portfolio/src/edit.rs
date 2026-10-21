use rust_decimal::Decimal;
use serde::Deserialize;

use crate::CustomType;
use crate::presets::Preset;
use crate::validate::{currency, fraction, name, non_negative, positive};
use crate::{
    Account, AccountId, Alias, CardSource, Category, CategoryId, Error, FetchLink, Group, GroupId,
    Holding, ImportMapping, KindRead, KindRule, MAX_PRODUCT_WEIGHT, Portfolio, Product, ProductId,
    ProductShareEntry, Recipe, Strategy, SuggestionMode, TypeRule,
};

/// Which record stays when a merge finds a record of both products in one account.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MergeConflict {
    /// The record of the product that is kept stays.
    #[default]
    Keep,
    /// The record of the product that is removed replaces it.
    Take,
}

/// One user action. The frontend sends the intent and Rust validates it, assigns
/// ids, and applies it, so no rule lives in the UI.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
// An edit is built and applied once per user action, never stored in bulk: the one large variant
// (`SetRecipe`) is not worth boxing and changing every caller.
#[allow(clippy::large_enum_variant)]
pub enum Edit {
    /// Adds a product and links a name of an import source to it in one step (a name the import
    /// did not know yet becomes a product). Both or neither.
    AddProductWithAlias {
        name: String,
        category: CategoryId,
        currency: String,
        source: String,
        alias: String,
    },
    /// Adds a group (a main category above the categories). It has no target of its own.
    AddGroup {
        name: String,
    },
    RenameGroup {
        id: GroupId,
        name: String,
    },
    /// Removes a group; its categories stay and become ungrouped.
    /// Moves a group one place up or down in the list (the order of the groups on the screens and
    /// in the charts). At the end of the list nothing changes.
    MoveGroup {
        id: GroupId,
        up: bool,
    },
    /// Moves a group to a given place in the list: `index` is the place it has afterwards (0 is
    /// the top). An index past the end is refused; the same place changes nothing.
    MoveGroupTo {
        id: GroupId,
        index: usize,
    },
    RemoveGroup {
        id: GroupId,
    },
    /// Puts a category into a group, or (`None`) takes it out of its group.
    SetCategoryGroup {
        category: CategoryId,
        group: Option<GroupId>,
    },
    AddCategory {
        name: String,
        target_weight: Decimal,
    },
    UpdateCategory {
        id: CategoryId,
        name: Option<String>,
        target_weight: Option<Decimal>,
    },
    RemoveCategory {
        id: CategoryId,
    },
    /// Moves a category one place up or down in the list (the order of the categories on the
    /// screens and in the charts). At the end of the list nothing changes.
    MoveCategory {
        id: CategoryId,
        up: bool,
    },
    /// Moves a category to a given place in the list, like `MoveGroupTo`.
    MoveCategoryTo {
        id: CategoryId,
        index: usize,
    },
    AddProduct {
        name: String,
        category: CategoryId,
        currency: String,
    },
    UpdateProduct {
        id: ProductId,
        name: Option<String>,
        category: Option<CategoryId>,
        currency: Option<String>,
    },
    RemoveProduct {
        id: ProductId,
    },
    /// Gives many products one category in one step: an existing `category`, or a `new_category`
    /// made here (no target, no group). Exactly one of the two. All or nothing.
    SetProductsCategory {
        products: Vec<ProductId>,
        #[serde(default)]
        category: Option<CategoryId>,
        #[serde(default)]
        new_category: Option<String>,
    },
    /// Merges the product `remove` into `keep` (the same currency): its aliases and records move to
    /// `keep`, and `remove` is deleted. Where an account holds a record of both, `on_conflict` says
    /// which one stays.
    MergeProducts {
        keep: ProductId,
        remove: ProductId,
        #[serde(default)]
        on_conflict: MergeConflict,
    },
    /// Cuts an alias of a product out as a new product (with the category and the currency of the
    /// original). A record has no source name, so the records to move are picked by account.
    SplitProduct {
        product: ProductId,
        source: String,
        alias: String,
        name: String,
        accounts: Vec<AccountId>,
    },
    AddAlias {
        product: ProductId,
        source: String,
        name: String,
    },
    RemoveAlias {
        product: ProductId,
        source: String,
        name: String,
    },
    AddAccount {
        name: String,
    },
    UpdateAccount {
        id: AccountId,
        name: Option<String>,
        sellable: Option<bool>,
        rounding_unit: Option<Decimal>,
    },
    /// Sets the products that do not take part in trading in the account (suggestions never buy or
    /// sell them): in the account's own suggestions, or with `all` in the suggestions across all
    /// accounts.
    SetExcluded {
        id: AccountId,
        products: Vec<ProductId>,
        #[serde(default)]
        all: bool,
    },
    /// Sets what the account's suggestions aim at. A product strategy needs at least one product.
    SetStrategy {
        id: AccountId,
        strategy: Strategy,
    },
    /// Moves an account to a given place in the list, like `MoveGroupTo`.
    MoveAccountTo {
        id: AccountId,
        index: usize,
    },
    RemoveAccount {
        id: AccountId,
    },
    /// Moves an account one place up or down in the list (the order is the order everywhere:
    /// the tabs, the tables, the choices). At the end of the list nothing changes.
    MoveAccount {
        id: AccountId,
        up: bool,
    },
    /// Adds an account type of the user's own (a custody type the built-in ones lack).
    AddAccountType {
        name: String,
        sellable: bool,
    },
    UpdateAccountType {
        id: String,
        name: Option<String>,
        sellable: Option<bool>,
    },
    /// Removes a custom type; the records that had it are left without a type (merged into the
    /// untyped record of the same product in the same account, when there is one).
    RemoveAccountType {
        id: String,
    },
    /// Changes the type of a record (`None`: not set). Where the account already has a record of
    /// the product with the new type, the edit is refused.
    SetRecordType {
        account: AccountId,
        product: ProductId,
        #[serde(default)]
        from: Option<String>,
        #[serde(default)]
        to: Option<String>,
    },
    SetHolding {
        account: AccountId,
        product: ProductId,
        #[serde(default)]
        account_type: Option<String>,
        value: Decimal,
        quantity: Option<Decimal>,
    },
    RemoveHolding {
        account: AccountId,
        product: ProductId,
        #[serde(default)]
        account_type: Option<String>,
    },
    /// Sets the weight of a product for one suggestion mode (the default weight 1 clears the
    /// setting). Global: it applies to every account.
    SetProductShare {
        mode: SuggestionMode,
        product: ProductId,
        weight: Decimal,
    },
    /// Makes the account hold exactly these products, as an import does. The
    /// values are in each product's own currency.
    ///
    /// With `types`, only the records of these types (`None`: records without a type) are
    /// replaced, and every input must have one of them; the account's records of other types are
    /// left as they are. Without `types`, the whole account is replaced.
    ReplaceHoldings {
        account: AccountId,
        holdings: Vec<HoldingInput>,
        #[serde(default)]
        types: Option<Vec<Option<String>>>,
    },
    /// Saves (or replaces) the column mapping of an import source.
    SetImportMapping {
        source: String,
        name_column: usize,
        value_column: usize,
        #[serde(default)]
        price_column: Option<usize>,
        #[serde(default = "one_unit")]
        price_unit: u64,
        header_rows: usize,
        #[serde(default = "one_row")]
        rows_per_record: usize,
        #[serde(default)]
        name_row: usize,
        #[serde(default)]
        value_row: usize,
        #[serde(default)]
        name_part: Option<usize>,
        #[serde(default)]
        value_part: Option<usize>,
        #[serde(default)]
        price_part: Option<usize>,
        #[serde(default)]
        evaluation_column: Option<usize>,
        #[serde(default)]
        evaluation_part: Option<usize>,
        #[serde(default)]
        loose_numbers: bool,
        #[serde(default)]
        quantity_column: Option<usize>,
        #[serde(default)]
        quantity_part: Option<usize>,
        #[serde(default)]
        max_records: Option<usize>,
        #[serde(default)]
        kind_column: Option<usize>,
        #[serde(default)]
        kind_rules: Vec<KindRule>,
    },
    RemoveImportMapping {
        source: String,
    },
    /// Saves (or replaces, by name) a fetch recipe: the links to follow, the table
    /// to read, how to read it, and the account to update.
    SetRecipe {
        recipe: Recipe,
    },
    RemoveRecipe {
        name: String,
    },
    /// Adds a recipe that ships with the app (see `presets`) for an account. The recipe refers to
    /// the preset and keeps only the account: its definition is read from the app's current preset
    /// data each time, so it cannot be edited (it can be removed or detached). Adding the same
    /// preset for the same account again does nothing; a copy of it made by an older version (an
    /// ordinary recipe that does the same) is turned into a reference. For a second account, the
    /// account's name is added to the recipe's name.
    AddPresetRecipe {
        preset: String,
        account: AccountId,
    },
    /// Makes an ordinary recipe of a recipe that refers to a preset, with the definition the preset
    /// has now (for good: it no longer follows the preset, and can be edited).
    DetachPresetRecipe {
        name: String,
    },
    /// Moves a recipe to another account. A recipe of a preset keeps referring to it, and an
    /// account never gets the same preset twice (the rows would add up twice).
    SetRecipeAccount {
        name: String,
        account: AccountId,
    },
    SetExchangeRate {
        usd_jpy: Option<Decimal>,
    },
    SetTolerances {
        category: Decimal,
        product: Decimal,
    },
}

/// One product's value in a [`Edit::ReplaceHoldings`].
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct HoldingInput {
    pub product: ProductId,
    /// The type the import read for the record; `None`: not read (the record keeps the type of the
    /// existing record of the product in the account, when there is exactly one).
    #[serde(default)]
    pub account_type: Option<String>,
    pub value: Decimal,
    /// What the value was worked out from, when the import knows it.
    #[serde(default)]
    pub quantity: Option<Decimal>,
    #[serde(default)]
    pub price: Option<Decimal>,
    #[serde(default = "one_unit")]
    pub price_unit: u64,
}

/// How many values one cell can be split into.
const MAX_PARTS: usize = 20;

fn one_unit() -> u64 {
    1
}

fn one_row() -> usize {
    1
}

impl ImportMapping {
    /// Checks the columns and rows and trims the source name.
    pub(crate) fn validated(mut self) -> Result<Self, Error> {
        let source = name(&self.source, "import source")?;
        let invalid = |what: &str| Error::Invalid(format!("{what} is out of range"));
        if self.name_column >= MAX_COLUMN
            || self.value_column >= MAX_COLUMN
            || self.price_column.is_some_and(|c| c >= MAX_COLUMN)
            || self.quantity_column.is_some_and(|c| c >= MAX_COLUMN)
            || self.evaluation_column.is_some_and(|c| c >= MAX_COLUMN)
        {
            return Err(invalid("a column"));
        }
        if self.header_rows > MAX_HEADER_ROWS {
            return Err(invalid("the heading count"));
        }
        if !(1..=MAX_ROWS_PER_RECORD).contains(&self.rows_per_record)
            || self.name_row >= self.rows_per_record
            || self.value_row >= self.rows_per_record
        {
            return Err(invalid("the rows per record"));
        }
        if [
            self.name_part,
            self.value_part,
            self.price_part,
            self.quantity_part,
            self.evaluation_part,
        ]
        .iter()
        .any(|p| p.is_some_and(|p| p >= MAX_PARTS))
        {
            return Err(invalid("a part number"));
        }
        if self.max_records.is_some_and(|n| n == 0 || n > MAX_ITEMS) {
            return Err(invalid("the record limit"));
        }
        if !(1..=MAX_PRICE_UNIT).contains(&self.price_unit) {
            return Err(invalid("the price unit"));
        }
        self.validate_kinds()?;
        if self.evaluation_column.is_some() {
            if self.price_column.is_none() {
                return Err(Error::Invalid(
                    "an evaluation column needs a price column to be checked against".to_owned(),
                ));
            }
            if self.evaluation_column == Some(self.value_column)
                || self.evaluation_column == self.price_column
            {
                return Err(Error::Invalid(
                    "the evaluation, the quantity and the price must be in different columns"
                        .to_owned(),
                ));
            }
        } else if self.evaluation_part.is_some() {
            return Err(invalid("an evaluation part without a column"));
        }
        // The name and the value may share a column number only when they are in
        // different rows of the record.
        if self.name_column == self.value_column && self.name_row == self.value_row {
            return Err(Error::Invalid(
                "the name and the value must be in different cells".to_owned(),
            ));
        }
        if self.price_column == Some(self.value_column) {
            return Err(Error::Invalid(
                "the quantity and the price must be in different columns".to_owned(),
            ));
        }
        Ok(Self { source, ..self })
    }
}

/// How many kind rules a mapping can have.
const MAX_KIND_RULES: usize = 10;
/// The longest kind text.
const MAX_KIND_CHARS: usize = 100;

impl ImportMapping {
    /// Checks the kind column and rules: both or neither, no kind twice, columns in range, and a
    /// price column for a rule that multiplies by it.
    fn validate_kinds(&mut self) -> Result<(), Error> {
        let invalid = |what: &str| Error::Invalid(what.to_owned());
        match (self.kind_column, self.kind_rules.is_empty()) {
            (None, true) => return Ok(()),
            (None, false) => return Err(invalid("the kind rules need a kind column")),
            (Some(_), true) => return Err(invalid("a kind column needs at least one kind rule")),
            (Some(column), false) if column >= MAX_COLUMN => {
                return Err(invalid("the kind column is out of range"));
            }
            _ => {}
        }
        if self.kind_rules.len() > MAX_KIND_RULES {
            return Err(invalid("too many kind rules"));
        }
        let mut seen: Vec<String> = Vec::new();
        for rule in &mut self.kind_rules {
            // Compared the way a cell is: line breaks and runs of spaces are one space.
            let kind = rule.kind.split_whitespace().collect::<Vec<_>>().join(" ");
            if kind.is_empty() || kind.chars().count() > MAX_KIND_CHARS {
                return Err(invalid("a kind must be 1 to 100 characters"));
            }
            if seen.contains(&kind) {
                return Err(invalid("a kind is listed twice"));
            }
            seen.push(kind.clone());
            rule.kind = kind;
            match rule.read {
                KindRead::QuantityTimesPrice { price_unit } => {
                    if !(1..=MAX_PRICE_UNIT).contains(&price_unit) {
                        return Err(invalid("the price unit is out of range"));
                    }
                    if self.price_column.is_none() {
                        return Err(invalid(
                            "a rule that multiplies by the price needs a price column",
                        ));
                    }
                }
                KindRead::Evaluation { column, part } => {
                    if column >= MAX_COLUMN {
                        return Err(invalid("the evaluation column is out of range"));
                    }
                    if part.is_some_and(|p| p >= MAX_PARTS) {
                        return Err(invalid("a part number is out of range"));
                    }
                }
            }
        }
        Ok(())
    }
}

/// The sites a recipe may be allowed to cross to. Each is a registered domain of one company
/// (never a domain that many unrelated owners share, such as a blog or hosting service), added
/// here after the company and its pages were checked. A list of what to refuse could never be
/// complete, so only these are accepted; adding a site is a change to this list and its review.
pub const VETTED_SITES: &[&str] = &[
    "sbisec.co.jp",
    "monex.co.jp",
    "connect-sec.co.jp",
    "rakuten-sec.co.jp",
];
const MAX_ALLOWED_SITES: usize = 3;

/// Checks the allowed sites of a recipe: each must be one of the vetted sites (trimmed and
/// lower-cased); duplicates go.
fn allowed_sites(sites: &[String]) -> Result<Vec<String>, Error> {
    if sites.len() > MAX_ALLOWED_SITES {
        return Err(Error::Invalid(
            "a recipe has too many allowed sites".to_owned(),
        ));
    }
    let mut out: Vec<String> = Vec::new();
    for site in sites {
        let site = site.trim().to_lowercase();
        if !VETTED_SITES.contains(&site.as_str()) {
            return Err(Error::Invalid(
                "that site is not one the app may follow links to".to_owned(),
            ));
        }
        if !out.contains(&site) {
            out.push(site);
        }
    }
    Ok(out)
}

/// The site of the page a recipe reads: empty, or a plain lower-case host name with at least two
/// labels. It only chooses which open tab a recipe reads, so it needs no vetting.
fn start_site(site: &str) -> Result<String, Error> {
    let site = site.trim().to_lowercase();
    if site.is_empty() {
        return Ok(site);
    }
    let labels: Vec<&str> = site.split('.').collect();
    let plain = site.len() <= 100
        && labels.len() >= 2
        && labels.iter().all(|l| {
            !l.is_empty()
                && l.len() <= 63
                && !l.starts_with('-')
                && !l.ends_with('-')
                && l.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        });
    if plain {
        Ok(site)
    } else {
        Err(Error::Invalid(
            "the start site is not acceptable".to_owned(),
        ))
    }
}

impl Recipe {
    /// Checks a recipe and trims its names. A recipe that refers to a preset is checked as the
    /// definition it stands for (the vetted sites, the click rules, and the rest apply to that),
    /// and what is kept is only the reference: whatever else it carried is dropped.
    pub(crate) fn validated(self, portfolio: &Portfolio) -> Result<Self, Error> {
        if self.preset.is_none() {
            return self.validated_definition(portfolio);
        }
        Ok(self
            .effective()
            .map_err(|_| Error::Invalid("that preset does not exist".to_owned()))?
            .validated_definition(portfolio)?
            .into_preset_reference())
    }

    /// Whether a stored recipe that refers to a preset holds only what a reference may: no
    /// definition of its own, and links that are only a text and a remembered destination.
    pub(crate) fn is_clean_preset_reference(&self) -> bool {
        let bare = |recipe: &Recipe| Recipe {
            links: Vec::new(),
            ..recipe.clone()
        };
        let bounded = |value: &str, max: usize| {
            value.chars().count() <= max && !value.chars().any(char::is_control)
        };
        self.links.len() <= MAX_RECIPE_LINKS
            && self.links.iter().all(|l| {
                l.menu.is_none()
                    && l.heading.is_none()
                    && bounded(&l.text, MAX_LINK_TEXT_CHARS)
                    && l.reference
                        .as_deref()
                        .is_some_and(|r| !r.is_empty() && bounded(r, MAX_LINK_REFERENCE_CHARS))
            })
            && bare(self) == bare(&self.clone().into_preset_reference())
    }

    fn validated_definition(self, portfolio: &Portfolio) -> Result<Self, Error> {
        let recipe_name = name(&self.name, "recipe")?;
        if self.links.len() > MAX_RECIPE_LINKS {
            return Err(Error::Invalid("a recipe has too many links".to_owned()));
        }
        let mut links = Vec::with_capacity(self.links.len());
        for link in self.links {
            let text = link.text.trim().to_owned();
            if text.is_empty()
                || text.chars().count() > MAX_LINK_TEXT_CHARS
                || text.chars().any(char::is_control)
            {
                return Err(Error::Invalid("a link text is not acceptable".to_owned()));
            }
            let reference = link
                .reference
                .map(|r| r.trim().to_owned())
                .filter(|r| !r.is_empty());
            if reference.as_ref().is_some_and(|r| {
                r.chars().count() > MAX_LINK_REFERENCE_CHARS || r.chars().any(char::is_control)
            }) {
                return Err(Error::Invalid(
                    "a link reference is not acceptable".to_owned(),
                ));
            }
            let clean = |value: Option<String>| {
                value.map(|v| v.trim().to_owned()).filter(|v| !v.is_empty())
            };
            let (menu, heading) = (clean(link.menu), clean(link.heading));
            // A menu entry is clicked, so it names its group and the heading to expect on arrival
            // together (and has no saved destination: a script link has none to remember).
            if menu.is_some() != heading.is_some() || (menu.is_some() && reference.is_some()) {
                return Err(Error::Invalid("a menu link is not acceptable".to_owned()));
            }
            if [&menu, &heading]
                .into_iter()
                .flatten()
                .any(|v| v.chars().count() > MAX_LINK_TEXT_CHARS || v.chars().any(char::is_control))
            {
                return Err(Error::Invalid("a menu link is not acceptable".to_owned()));
            }
            links.push(FetchLink {
                text,
                reference,
                menu,
                heading,
            });
        }
        // Clicks are made only on the vetted sites, and a recipe either clicks all its links or
        // follows them all (a page reached by a click cannot be gone back from, so the two are
        // not mixed).
        let clicks = links.iter().filter(|l| l.menu.is_some()).count();
        if clicks != 0 && (clicks != links.len() || self.allowed_sites.is_empty()) {
            return Err(Error::Invalid("a menu link is not acceptable".to_owned()));
        }
        let cards = match self.cards {
            Some(source) => {
                let heading = source.heading.trim().to_owned();
                if !self.table_starts_with.trim().is_empty()
                    || self.all_tables
                    || self.following_tables
                    || heading.is_empty()
                    || heading.chars().count() > MAX_LINK_TEXT_CHARS
                    || heading.chars().any(char::is_control)
                    || !(1..=3).contains(&source.item_depth)
                    // Every card is one data row: nothing to skip, one row per record.
                    || self.mapping.header_rows != 0
                    || self.mapping.rows_per_record != 1
                {
                    return Err(Error::Invalid(
                        "the card source is not acceptable".to_owned(),
                    ));
                }
                Some(Box::new(CardSource {
                    heading,
                    item_depth: source.item_depth,
                }))
            }
            None => None,
        };
        let table_starts_with = self.table_starts_with.trim().to_owned();
        if cards.is_none()
            && (table_starts_with.is_empty()
                || table_starts_with.chars().count() > MAX_LINK_TEXT_CHARS
                || table_starts_with.chars().any(char::is_control))
        {
            return Err(Error::Invalid(
                "the table title is not acceptable".to_owned(),
            ));
        }
        portfolio.account(self.account)?;
        let type_rules = validated_type_rules(
            &self.type_rules,
            self.all_tables,
            cards.is_some(),
            portfolio,
        )?;
        let allowed_sites = allowed_sites(&self.allowed_sites)?;
        let start_site = start_site(&self.start_site)?;
        Ok(Self {
            name: recipe_name,
            links,
            table_starts_with,
            cards,
            allowed_sites,
            start_site,
            // Empty: the currency of each product the rows are linked to.
            currency: if self.currency.trim().is_empty() {
                String::new()
            } else {
                currency(&self.currency)?
            },
            mapping: self.mapping.validated()?,
            type_rules,
            ..self
        })
    }
}

/// The title rules, trimmed. They name the types of the tables of a page read with
/// `all_tables`, so a recipe that reads another way has none.
fn validated_type_rules(
    type_rules: &[TypeRule],
    all_tables: bool,
    cards: bool,
    portfolio: &Portfolio,
) -> Result<Vec<TypeRule>, Error> {
    if type_rules.is_empty() {
        return Ok(Vec::new());
    }
    let bad = |what: &str| Error::Invalid(format!("the type rules are not acceptable: {what}"));
    if cards || !all_tables || type_rules.len() > MAX_TYPE_RULES {
        return Err(bad("they need a recipe that reads every matching table"));
    }
    let mut rules: Vec<TypeRule> = Vec::with_capacity(type_rules.len());
    for rule in type_rules {
        let title = rule.title.trim().to_owned();
        if title.is_empty()
            || title.chars().count() > MAX_LINK_TEXT_CHARS
            || title.chars().any(char::is_control)
        {
            return Err(bad("a title"));
        }
        portfolio.check_type(Some(&rule.account_type))?;
        let squashed: String = title.split_whitespace().collect();
        if rules
            .iter()
            .any(|r| r.title.split_whitespace().collect::<String>() == squashed)
        {
            return Err(bad("a title appears twice"));
        }
        rules.push(TypeRule {
            title,
            account_type: rule.account_type.clone(),
        });
    }
    Ok(rules)
}

const MAX_TYPE_RULES: usize = 30;
const MAX_RECIPES: usize = 50;
const MAX_RECIPE_LINKS: usize = 5;
const MAX_LINK_TEXT_CHARS: usize = 60;
const MAX_LINK_REFERENCE_CHARS: usize = 500;

const MAX_ITEMS: usize = 10_000;
/// The widest table an import reads (see the `tabular` crate), and the most
/// heading rows a mapping may skip.
const MAX_COLUMN: usize = 200;
const MAX_HEADER_ROWS: usize = 1_000;
const MAX_ROWS_PER_RECORD: usize = 10;
const MAX_PRICE_UNIT: u64 = 1_000_000_000;
const MAX_ALIASES_PER_PRODUCT: usize = 50;

impl Portfolio {
    /// Adds a product and returns its id.
    fn add_product(
        &mut self,
        new_name: &str,
        category: CategoryId,
        code: &str,
    ) -> Result<ProductId, Error> {
        self.check_capacity(self.products.len())?;
        let new_name = name(new_name, "product")?;
        let code = currency(code)?;
        self.category(category)?;
        self.ensure_product_name_free(&new_name, None)?;
        let id = ProductId(self.take_id());
        self.products.push(Product {
            id,
            name: new_name,
            category,
            currency: code,
            aliases: Vec::new(),
        });
        Ok(id)
    }

    /// Links a name of an import source to a product (once; another product may not have it).
    fn add_alias(
        &mut self,
        product: ProductId,
        source: &str,
        alias_name: &str,
    ) -> Result<(), Error> {
        let alias = Alias {
            source: name(source, "alias source")?,
            name: name(alias_name, "alias")?,
        };
        self.product(product)?;
        if let Some(owner) = self
            .products
            .iter()
            .find(|p| p.aliases.contains(&alias))
            .map(|p| p.id)
        {
            if owner == product {
                return Ok(());
            }
            return Err(Error::Conflict(
                "this name is already linked to another product".to_owned(),
            ));
        }
        let product = self.product_mut(product)?;
        if product.aliases.len() >= MAX_ALIASES_PER_PRODUCT {
            return Err(Error::Invalid(
                "too many aliases for one product".to_owned(),
            ));
        }
        product.aliases.push(alias);
        Ok(())
    }

    /// Applies one edit. The edit is fully validated before anything changes,
    /// so a rejected edit leaves the portfolio untouched.
    pub fn apply(&mut self, edit: Edit) -> Result<(), Error> {
        match edit {
            Edit::AddGroup { name: new_name } => {
                self.check_capacity(self.groups.len())?;
                let new_name = name(&new_name, "group")?;
                self.ensure_group_name_free(&new_name, None)?;
                let id = GroupId(self.take_id());
                self.groups.push(Group { id, name: new_name });
            }
            Edit::RenameGroup { id, name: new_name } => {
                let new_name = name(&new_name, "group")?;
                self.group(id)?;
                self.ensure_group_name_free(&new_name, Some(id))?;
                if let Some(group) = self.groups.iter_mut().find(|g| g.id == id) {
                    group.name = new_name;
                }
            }
            Edit::MoveGroup { id, up } => {
                self.group(id)?;
                swap_with_neighbor(&mut self.groups, |g| g.id == id, up);
            }
            Edit::MoveCategory { id, up } => {
                self.category(id)?;
                swap_with_neighbor(&mut self.categories, |c| c.id == id, up);
            }
            Edit::MoveGroupTo { id, index } => {
                self.group(id)?;
                move_to(&mut self.groups, |g| g.id == id, index)?;
            }
            Edit::MoveCategoryTo { id, index } => {
                self.category(id)?;
                move_to(&mut self.categories, |c| c.id == id, index)?;
            }
            Edit::MoveAccountTo { id, index } => {
                self.account(id)?;
                move_to(&mut self.accounts, |a| a.id == id, index)?;
            }
            Edit::RemoveGroup { id } => {
                self.group(id)?;
                self.groups.retain(|g| g.id != id);
                for category in self.categories.iter_mut().filter(|c| c.group == Some(id)) {
                    category.group = None;
                }
            }
            Edit::SetCategoryGroup { category, group } => {
                if let Some(group) = group {
                    self.group(group)?;
                }
                self.category_mut(category)?.group = group;
            }
            Edit::AddCategory {
                name: new_name,
                target_weight,
            } => {
                self.check_capacity(self.categories.len())?;
                let new_name = name(&new_name, "category")?;
                let target_weight = fraction(target_weight, "category target")?;
                self.ensure_category_name_free(&new_name, None)?;
                let id = CategoryId(self.take_id());
                self.categories.push(Category {
                    id,
                    name: new_name,
                    group: None,
                    target_weight,
                });
            }
            Edit::UpdateCategory {
                id,
                name: new_name,
                target_weight,
            } => {
                let new_name = new_name.map(|n| name(&n, "category")).transpose()?;
                let target_weight = target_weight
                    .map(|w| fraction(w, "category target"))
                    .transpose()?;
                self.category(id)?;
                if let Some(n) = &new_name {
                    self.ensure_category_name_free(n, Some(id))?;
                }
                let category = self.category_mut(id)?;
                if let Some(n) = new_name {
                    category.name = n;
                }
                if let Some(w) = target_weight {
                    category.target_weight = w;
                }
            }
            Edit::RemoveCategory { id } => {
                self.category(id)?;
                if self.products.iter().any(|p| p.category == id) {
                    return Err(Error::Conflict(
                        "move or remove the category's products first".to_owned(),
                    ));
                }
                self.categories.retain(|c| c.id != id);
            }
            Edit::AddProduct {
                name: new_name,
                category,
                currency: code,
            } => {
                self.add_product(&new_name, category, &code)?;
            }
            Edit::AddProductWithAlias {
                name: new_name,
                category,
                currency: code,
                source,
                alias,
            } => {
                // Both or neither: an alias that cannot be added leaves no new product behind.
                let before = self.clone();
                let result = self
                    .add_product(&new_name, category, &code)
                    .and_then(|product| self.add_alias(product, &source, &alias));
                if let Err(error) = result {
                    *self = before;
                    return Err(error);
                }
            }
            Edit::UpdateProduct {
                id,
                name: new_name,
                category,
                currency: code,
            } => {
                let new_name = new_name.map(|n| name(&n, "product")).transpose()?;
                let code = code.map(|c| currency(&c)).transpose()?;
                self.product(id)?;
                if let Some(category) = category {
                    self.category(category)?;
                }
                if let Some(n) = &new_name {
                    self.ensure_product_name_free(n, Some(id))?;
                }
                let product = self.product_mut(id)?;
                if let Some(n) = new_name {
                    product.name = n;
                }
                if let Some(c) = category {
                    product.category = c;
                }
                if let Some(c) = code {
                    product.currency = c;
                }
            }
            Edit::SetProductsCategory {
                products,
                category,
                new_category,
            } => {
                if products.is_empty() {
                    return Err(Error::Invalid("choose at least one product".to_owned()));
                }
                // Everything is checked before anything changes.
                for id in &products {
                    self.product(*id)?;
                }
                let target = match (category, new_category) {
                    (Some(id), None) => {
                        self.category(id)?;
                        id
                    }
                    (None, Some(new_name)) => {
                        self.check_capacity(self.categories.len())?;
                        let new_name = name(&new_name, "category")?;
                        self.ensure_category_name_free(&new_name, None)?;
                        let id = CategoryId(self.take_id());
                        self.categories.push(Category {
                            id,
                            name: new_name,
                            group: None,
                            target_weight: Decimal::ZERO,
                        });
                        id
                    }
                    _ => {
                        return Err(Error::Invalid(
                            "choose an existing category or a new one, not both".to_owned(),
                        ));
                    }
                };
                for product in self
                    .products
                    .iter_mut()
                    .filter(|p| products.contains(&p.id))
                {
                    product.category = target;
                }
            }
            Edit::RemoveProduct { id } => {
                self.product(id)?;
                self.products.retain(|p| p.id != id);
                self.holdings.retain(|h| h.product != id);
                self.product_shares.retain(|e| e.product != id);
                for account in &mut self.accounts {
                    account.excluded.retain(|p| *p != id);
                    account.excluded_all.retain(|p| *p != id);
                    account.drop_strategy_product(id);
                }
            }
            Edit::MergeProducts {
                keep,
                remove,
                on_conflict,
            } => {
                if keep == remove {
                    return Err(Error::Invalid("choose two different products".to_owned()));
                }
                // Ratios of a product listed twice add up; refuse a merge whose sum would break the
                // limit of a ratio before anything changes.
                for account in &self.accounts {
                    let ratio = |id| {
                        account
                            .strategy
                            .items()
                            .iter()
                            .find(|i| i.product == id)
                            .and_then(|i| i.ratio)
                    };
                    if let (Some(a), Some(b)) = (ratio(keep), ratio(remove))
                        && a.checked_add(b)
                            .is_none_or(|sum| sum > crate::validate::MAX_STRATEGY_RATIO)
                    {
                        return Err(Error::Invalid(
                            "the merged strategy ratio would be too large".to_owned(),
                        ));
                    }
                }
                let from = self.product(remove)?.clone();
                let into = self.product(keep)?;
                if from.currency != into.currency {
                    return Err(Error::Invalid(
                        "products in different currencies cannot be merged".to_owned(),
                    ));
                }
                if into.aliases.len() + from.aliases.len() > MAX_ALIASES_PER_PRODUCT {
                    return Err(Error::Invalid(
                        "too many aliases for one product".to_owned(),
                    ));
                }
                let kept = |h: &Holding| h.product == keep;
                for h in std::mem::take(&mut self.holdings) {
                    let mut h = h;
                    if h.product == remove {
                        let clash = self.holdings.iter().any(|o| {
                            kept(o) && o.account == h.account && o.account_type == h.account_type
                        });
                        h.product = keep;
                        if clash {
                            if on_conflict == MergeConflict::Keep {
                                continue;
                            }
                            self.holdings.retain(|o| {
                                !(kept(o)
                                    && o.account == h.account
                                    && o.account_type == h.account_type)
                            });
                        }
                    }
                    self.holdings.push(h);
                }
                for account in &mut self.accounts {
                    for list in [&mut account.excluded, &mut account.excluded_all] {
                        if list.contains(&remove) && !list.contains(&keep) {
                            list.push(keep);
                        }
                        list.retain(|p| *p != remove);
                    }
                    account.merge_strategy_product(keep, remove);
                }
                self.product_shares.retain(|e| e.product != remove);
                self.product_mut(keep)?.aliases.extend(from.aliases);
                self.products.retain(|p| p.id != remove);
            }
            Edit::SplitProduct {
                product,
                source,
                alias,
                name: new_name,
                accounts,
            } => {
                self.check_capacity(self.products.len())?;
                let new_name = name(&new_name, "product")?;
                self.ensure_product_name_free(&new_name, None)?;
                let original = self.product(product)?.clone();
                let position = original
                    .aliases
                    .iter()
                    .position(|a| a.source == source.trim() && a.name == alias.trim())
                    .ok_or(Error::NotFound("alias"))?;
                for account in &accounts {
                    self.account(*account)?;
                }
                let id = ProductId(self.take_id());
                let moved = self.product_mut(product)?.aliases.remove(position);
                self.products.push(Product {
                    id,
                    name: new_name,
                    category: original.category,
                    currency: original.currency,
                    aliases: vec![moved],
                });
                for h in &mut self.holdings {
                    if h.product == product && accounts.contains(&h.account) {
                        h.product = id;
                    }
                }
                for account in &mut self.accounts {
                    for list in [&mut account.excluded, &mut account.excluded_all] {
                        if list.contains(&product) {
                            list.push(id);
                        }
                    }
                }
            }
            Edit::AddAlias {
                product,
                source,
                name: alias_name,
            } => {
                self.add_alias(product, &source, &alias_name)?;
            }
            Edit::RemoveAlias {
                product,
                source,
                name: alias_name,
            } => {
                let product = self.product_mut(product)?;
                product
                    .aliases
                    .retain(|a| !(a.source == source.trim() && a.name == alias_name.trim()));
            }
            Edit::AddAccount { name: new_name } => {
                self.check_capacity(self.accounts.len())?;
                let new_name = name(&new_name, "account")?;
                self.ensure_account_name_free(&new_name, None)?;
                let id = AccountId(self.take_id());
                self.accounts.push(Account {
                    id,
                    name: new_name,
                    sellable: true,
                    excluded: Vec::new(),
                    excluded_all: Vec::new(),
                    strategy: Strategy::Portfolio,
                    rounding_unit: Decimal::ONE,
                });
            }
            Edit::UpdateAccount {
                id,
                name: new_name,
                sellable,
                rounding_unit,
            } => {
                let new_name = new_name.map(|n| name(&n, "account")).transpose()?;
                let rounding_unit = rounding_unit
                    .map(|u| positive(u, "rounding unit"))
                    .transpose()?;
                self.account(id)?;
                if let Some(n) = &new_name {
                    self.ensure_account_name_free(n, Some(id))?;
                }
                let account = self.account_mut(id)?;
                if let Some(n) = new_name {
                    account.name = n;
                }
                if let Some(s) = sellable {
                    account.sellable = s;
                }
                if let Some(u) = rounding_unit {
                    account.rounding_unit = u;
                }
            }
            Edit::SetExcluded { id, products, all } => {
                for product in &products {
                    self.product(*product)?;
                }
                let mut products = products;
                products.sort();
                products.dedup();
                let account = self.account_mut(id)?;
                if all {
                    account.excluded_all = products;
                } else {
                    account.excluded = products;
                }
            }
            Edit::SetStrategy { id, strategy } => {
                self.account(id)?;
                let strategy = self.checked_strategy(strategy)?;
                self.account_mut(id)?.strategy = strategy;
            }
            Edit::MoveAccount { id, up } => {
                self.account(id)?;
                swap_with_neighbor(&mut self.accounts, |a| a.id == id, up);
            }
            Edit::RemoveAccount { id } => {
                self.account(id)?;
                self.accounts.retain(|a| a.id != id);
                self.holdings.retain(|h| h.account != id);
                self.recipes.retain(|r| r.account != id);
            }
            Edit::AddAccountType {
                name: new_name,
                sellable,
            } => {
                self.check_capacity(self.custom_types.len())?;
                let new_name = name(&new_name, "account type")?;
                self.ensure_type_name_free(&new_name, None)?;
                let id = format!("custom-{}", self.take_id());
                self.custom_types.push(CustomType {
                    id,
                    name: new_name,
                    sellable,
                });
            }
            Edit::UpdateAccountType {
                id,
                name: new_name,
                sellable,
            } => {
                let new_name = new_name.map(|n| name(&n, "account type")).transpose()?;
                if !self.custom_types.iter().any(|t| t.id == id) {
                    return Err(Error::NotFound("account type"));
                }
                if let Some(n) = &new_name {
                    self.ensure_type_name_free(n, Some(&id))?;
                }
                if let Some(t) = self.custom_types.iter_mut().find(|t| t.id == id) {
                    if let Some(n) = new_name {
                        t.name = n;
                    }
                    if let Some(s) = sellable {
                        t.sellable = s;
                    }
                }
            }
            Edit::RemoveAccountType { id } => {
                if !self.custom_types.iter().any(|t| t.id == id) {
                    return Err(Error::NotFound("account type"));
                }
                self.custom_types.retain(|t| t.id != id);
                for recipe in &mut self.recipes {
                    recipe.type_rules.retain(|r| r.account_type != id);
                }
                for h in std::mem::take(&mut self.holdings) {
                    let mut h = h;
                    if h.account_type.as_deref() == Some(id.as_str()) {
                        h.account_type = None;
                        // Where an untyped record of the product is there already, the typed
                        // record is dropped (nothing is added up: the page sets the value).
                        if self.holdings.iter().any(|o| {
                            o.account == h.account
                                && o.product == h.product
                                && o.account_type.is_none()
                        }) {
                            continue;
                        }
                    }
                    self.holdings.push(h);
                }
            }
            Edit::SetRecordType {
                account,
                product,
                from,
                to,
            } => {
                self.check_type(to.as_deref())?;
                let at = |h: &Holding, t: &Option<String>| {
                    h.account == account && h.product == product && &h.account_type == t
                };
                if from != to && self.holdings.iter().any(|h| at(h, &to)) {
                    return Err(Error::Conflict(
                        "the account already has a record of the product with that type".to_owned(),
                    ));
                }
                let holding = self
                    .holdings
                    .iter_mut()
                    .find(|h| {
                        h.account == account && h.product == product && h.account_type == from
                    })
                    .ok_or(Error::NotFound("record"))?;
                holding.account_type = to;
            }
            Edit::SetHolding {
                account,
                product,
                account_type,
                value,
                quantity,
            } => {
                self.check_type(account_type.as_deref())?;
                let value = non_negative(value, "holding value")?;
                let quantity = quantity.map(|q| non_negative(q, "quantity")).transpose()?;
                self.account(account)?;
                self.product(product)?;
                match self.holdings.iter_mut().find(|h| {
                    h.account == account && h.product == product && h.account_type == account_type
                }) {
                    Some(holding) => {
                        holding.value = value;
                        holding.quantity = quantity;
                        // A value typed by hand no longer follows from an imported price.
                        holding.price = None;
                        holding.price_unit = 1;
                    }
                    None => {
                        self.check_capacity(self.holdings.len())?;
                        self.holdings.push(Holding {
                            account,
                            product,
                            value,
                            quantity,
                            price: None,
                            price_unit: 1,
                            account_type,
                        });
                    }
                }
            }
            Edit::SetProductShare {
                mode,
                product,
                weight,
            } => {
                self.product(product)?;
                let weight = positive(weight, "weight")?;
                if weight > MAX_PRODUCT_WEIGHT {
                    return Err(Error::Invalid("the weight is too large".to_owned()));
                }
                self.product_shares
                    .retain(|e| !(e.mode == mode && e.product == product));
                if weight != Decimal::ONE {
                    self.check_capacity(self.product_shares.len())?;
                    self.product_shares.push(ProductShareEntry {
                        mode,
                        product,
                        weight,
                    });
                    self.product_shares.sort_by_key(|e| (e.mode, e.product));
                }
            }
            Edit::RemoveHolding {
                account,
                product,
                account_type,
            } => {
                self.holdings.retain(|h| {
                    !(h.account == account
                        && h.product == product
                        && h.account_type == account_type)
                });
            }
            Edit::ReplaceHoldings {
                account,
                holdings,
                types,
            } => {
                self.account(account)?;
                let in_scope = |t: &Option<String>| types.as_ref().is_none_or(|ts| ts.contains(t));
                for t in types.iter().flatten() {
                    self.check_type(t.as_deref())?;
                }
                let mut seen = std::collections::BTreeSet::new();
                let mut next = Vec::with_capacity(holdings.len());
                for input in holdings {
                    self.product(input.product)?;
                    self.check_type(input.account_type.as_deref())?;
                    // A record the page does not give a type for keeps the type of the existing
                    // record of the product in the account (when it is the only one).
                    let account_type = input.account_type.clone().or_else(|| {
                        if types.is_some() {
                            return None;
                        }
                        let mut existing = self
                            .holdings
                            .iter()
                            .filter(|h| h.account == account && h.product == input.product);
                        match (existing.next(), existing.next()) {
                            (Some(h), None) => h.account_type.clone(),
                            _ => None,
                        }
                    });
                    if !in_scope(&account_type) {
                        return Err(Error::Invalid(
                            "an imported record has a type the import does not replace".to_owned(),
                        ));
                    }
                    if !seen.insert((input.product, account_type.clone())) {
                        return Err(Error::Invalid(
                            "a product appears twice in the import".to_owned(),
                        ));
                    }
                    if !(1..=MAX_PRICE_UNIT).contains(&input.price_unit) {
                        return Err(Error::Invalid("the price unit is out of range".to_owned()));
                    }
                    next.push(Holding {
                        account,
                        product: input.product,
                        account_type: account_type.clone(),
                        value: non_negative(input.value, "holding value")?,
                        quantity: input
                            .quantity
                            .map(|q| non_negative(q, "quantity"))
                            .transpose()?,
                        price: input.price.map(|p| non_negative(p, "price")).transpose()?,
                        price_unit: input.price_unit,
                    });
                }
                let others = self
                    .holdings
                    .iter()
                    .filter(|h| h.account != account || !in_scope(&h.account_type))
                    .count();
                if others + next.len() > MAX_ITEMS {
                    return Err(Error::Invalid("too many items".to_owned()));
                }
                self.holdings
                    .retain(|h| h.account != account || !in_scope(&h.account_type));
                self.holdings.extend(next);
            }
            Edit::SetImportMapping {
                source,
                name_column,
                value_column,
                price_column,
                price_unit,
                header_rows,
                rows_per_record,
                name_row,
                value_row,
                name_part,
                value_part,
                price_part,
                evaluation_column,
                evaluation_part,
                loose_numbers,
                quantity_column,
                quantity_part,
                max_records,
                kind_column,
                kind_rules,
            } => {
                let mapping = ImportMapping {
                    source,
                    name_column,
                    value_column,
                    price_column,
                    price_unit,
                    header_rows,
                    rows_per_record,
                    name_row,
                    value_row,
                    name_part,
                    value_part,
                    price_part,
                    evaluation_column,
                    evaluation_part,
                    loose_numbers,
                    quantity_column,
                    quantity_part,
                    max_records,
                    kind_column,
                    kind_rules,
                }
                .validated()?;
                match self
                    .import_mappings
                    .iter_mut()
                    .find(|m| m.source == mapping.source)
                {
                    Some(existing) => *existing = mapping,
                    None => {
                        self.check_capacity(self.import_mappings.len())?;
                        self.import_mappings.push(mapping);
                    }
                }
            }
            Edit::SetRecipe { recipe } => {
                let recipe = recipe.validated(self)?;
                // Every recipe says the site it reads (so the right tab is found and its login
                // page can be opened). Only a recipe stored by an older version may lack one; it
                // stays loadable, but cannot be saved again. (A recipe of a preset takes its site from the
                // preset.)
                if recipe.preset.is_none() && recipe.start_site.is_empty() {
                    return Err(Error::Invalid(
                        "the site the recipe reads is required (for example example.co.jp)"
                            .to_owned(),
                    ));
                }
                match self.recipes.iter_mut().find(|r| r.name == recipe.name) {
                    Some(existing) => *existing = recipe,
                    None => {
                        if self.recipes.len() >= MAX_RECIPES {
                            return Err(Error::Invalid("too many recipes".to_owned()));
                        }
                        self.recipes.push(recipe);
                    }
                }
            }
            Edit::AddPresetRecipe { preset, account } => {
                let preset = Preset::find(preset.trim())
                    .ok_or_else(|| Error::Invalid("that preset does not exist".to_owned()))?;
                let account_name = self.account(account)?.name.clone();
                let mut recipe = preset.recipe(account);
                recipe.preset = Some(preset.id.to_owned());
                if self
                    .recipes
                    .iter()
                    .any(|r| r.account == account && r.preset.as_deref() == Some(preset.id))
                {
                    return Ok(());
                }
                // A copy made by an older version is found by what it does, not by its name (the
                // name may have been suffixed or changed): it becomes the reference in place, so
                // the account never gets the same preset twice (the rows would add up twice).
                let copy = self.recipes.iter().position(|r| {
                    r.preset.is_none()
                        && r.account == account
                        && r.table_starts_with == recipe.table_starts_with
                        && r.cards == recipe.cards
                        && r.mapping.source == recipe.mapping.source
                        && r.links
                            .iter()
                            .map(|l| &l.text)
                            .eq(recipe.links.iter().map(|l| &l.text))
                });
                if let Some(index) = copy {
                    recipe.name = self.recipes[index].name.clone();
                    self.recipes[index] = recipe.validated(self)?;
                } else {
                    if self.recipes.iter().any(|r| r.name == recipe.name) {
                        recipe.name = format!("{} ({})", recipe.name, account_name);
                    }
                    let recipe = recipe.validated(self)?;
                    if self.recipes.iter().any(|r| r.name == recipe.name) {
                        return Err(Error::Invalid(
                            "a recipe with that name exists; rename or remove it first".to_owned(),
                        ));
                    }
                    if self.recipes.len() >= MAX_RECIPES {
                        return Err(Error::Invalid("too many recipes".to_owned()));
                    }
                    self.recipes.push(recipe);
                }
            }
            Edit::DetachPresetRecipe { name } => {
                let name = name.trim();
                let index = self
                    .recipes
                    .iter()
                    .position(|r| r.name == name && r.preset.is_some())
                    .ok_or(Error::NotFound("recipe of a preset"))?;
                let mut recipe = self.recipes[index]
                    .effective()
                    .map_err(|_| Error::Invalid("that preset does not exist".to_owned()))?;
                recipe.preset = None;
                self.recipes[index] = recipe.validated(self)?;
            }
            Edit::SetRecipeAccount { name, account } => {
                let name = name.trim();
                self.account(account)?;
                let index = self
                    .recipes
                    .iter()
                    .position(|r| r.name == name)
                    .ok_or(Error::NotFound("recipe"))?;
                if let Some(preset) = &self.recipes[index].preset
                    && self.recipes.iter().enumerate().any(|(i, r)| {
                        i != index && r.account == account && r.preset.as_ref() == Some(preset)
                    })
                {
                    return Err(Error::Invalid(
                        "that account already has this recipe".to_owned(),
                    ));
                }
                let mut recipe = self.recipes[index].clone();
                recipe.account = account;
                self.recipes[index] = recipe.validated(self)?;
            }
            Edit::RemoveRecipe { name } => {
                let name = name.trim();
                self.recipes.retain(|r| r.name != name);
            }
            Edit::RemoveImportMapping { source } => {
                let source = source.trim();
                self.import_mappings.retain(|m| m.source != source);
            }
            Edit::SetExchangeRate { usd_jpy } => {
                self.settings.usd_jpy = usd_jpy
                    .map(|rate| positive(rate, "exchange rate"))
                    .transpose()?;
            }
            Edit::SetTolerances { category, product } => {
                let category = fraction(category, "category tolerance")?;
                let product = fraction(product, "product tolerance")?;
                self.settings.category_tolerance = category;
                self.settings.product_tolerance = product;
            }
        }
        self.revision = self.revision.saturating_add(1);
        Ok(())
    }

    fn take_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn check_capacity(&self, current: usize) -> Result<(), Error> {
        if current >= MAX_ITEMS {
            Err(Error::Invalid("too many items".to_owned()))
        } else {
            Ok(())
        }
    }

    fn group(&self, id: GroupId) -> Result<&Group, Error> {
        self.groups
            .iter()
            .find(|g| g.id == id)
            .ok_or(Error::NotFound("group"))
    }

    fn ensure_group_name_free(&self, name: &str, except: Option<GroupId>) -> Result<(), Error> {
        if self
            .groups
            .iter()
            .any(|g| g.name == name && Some(g.id) != except)
        {
            return Err(Error::Conflict("a group with this name exists".to_owned()));
        }
        Ok(())
    }

    fn category(&self, id: CategoryId) -> Result<&Category, Error> {
        self.categories
            .iter()
            .find(|c| c.id == id)
            .ok_or(Error::NotFound("category"))
    }

    fn category_mut(&mut self, id: CategoryId) -> Result<&mut Category, Error> {
        self.categories
            .iter_mut()
            .find(|c| c.id == id)
            .ok_or(Error::NotFound("category"))
    }

    fn product(&self, id: ProductId) -> Result<&Product, Error> {
        self.products
            .iter()
            .find(|p| p.id == id)
            .ok_or(Error::NotFound("product"))
    }

    fn product_mut(&mut self, id: ProductId) -> Result<&mut Product, Error> {
        self.products
            .iter_mut()
            .find(|p| p.id == id)
            .ok_or(Error::NotFound("product"))
    }

    fn account(&self, id: AccountId) -> Result<&Account, Error> {
        self.accounts
            .iter()
            .find(|a| a.id == id)
            .ok_or(Error::NotFound("account"))
    }

    fn account_mut(&mut self, id: AccountId) -> Result<&mut Account, Error> {
        self.accounts
            .iter_mut()
            .find(|a| a.id == id)
            .ok_or(Error::NotFound("account"))
    }

    fn ensure_category_name_free(
        &self,
        name: &str,
        except: Option<CategoryId>,
    ) -> Result<(), Error> {
        if self
            .categories
            .iter()
            .any(|c| c.name == name && Some(c.id) != except)
        {
            return Err(Error::Conflict(
                "a category with this name exists".to_owned(),
            ));
        }
        Ok(())
    }

    fn ensure_product_name_free(&self, name: &str, except: Option<ProductId>) -> Result<(), Error> {
        if self
            .products
            .iter()
            .any(|p| p.name == name && Some(p.id) != except)
        {
            return Err(Error::Conflict(
                "a product with this name exists".to_owned(),
            ));
        }
        Ok(())
    }

    fn check_type(&self, id: Option<&str>) -> Result<(), Error> {
        match id {
            Some(id) if !self.has_account_type(id) => Err(Error::NotFound("account type")),
            _ => Ok(()),
        }
    }

    fn ensure_type_name_free(&self, name: &str, except: Option<&str>) -> Result<(), Error> {
        if self
            .custom_types
            .iter()
            .any(|t| t.name == name && Some(t.id.as_str()) != except)
        {
            return Err(Error::Conflict(
                "an account type with this name exists".to_owned(),
            ));
        }
        Ok(())
    }

    fn ensure_account_name_free(&self, name: &str, except: Option<AccountId>) -> Result<(), Error> {
        if self
            .accounts
            .iter()
            .any(|a| a.name == name && Some(a.id) != except)
        {
            return Err(Error::Conflict(
                "an account with this name exists".to_owned(),
            ));
        }
        Ok(())
    }
}

/// Moves the first item matching `is` so that it ends up at `index`; the others keep their order.
fn move_to<T>(list: &mut Vec<T>, is: impl Fn(&T) -> bool, index: usize) -> Result<(), Error> {
    if index >= list.len() {
        return Err(Error::Invalid("the position is out of range".to_owned()));
    }
    if let Some(from) = list.iter().position(is) {
        let item = list.remove(from);
        list.insert(index, item);
    }
    Ok(())
}

/// Swaps the first item matching `is` with its neighbor above (`up`) or below; at an end of the
/// list nothing changes.
fn swap_with_neighbor<T>(list: &mut [T], is: impl Fn(&T) -> bool, up: bool) {
    let Some(index) = list.iter().position(is) else {
        return;
    };
    let other = if up {
        index.checked_sub(1)
    } else {
        Some(index + 1)
    };
    if let Some(other) = other.filter(|o| *o < list.len()) {
        list.swap(index, other);
    }
}
