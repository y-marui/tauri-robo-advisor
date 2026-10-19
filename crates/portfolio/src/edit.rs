use rust_decimal::Decimal;
use serde::Deserialize;

use crate::presets::Preset;
use crate::validate::{currency, fraction, name, non_negative, positive};
use crate::{
    Account, AccountId, AccountKind, Alias, Category, CategoryId, Error, FetchLink, Holding,
    ImportMapping, Portfolio, Product, ProductId, ProductTargeting, Recipe,
};

/// One user action. The frontend sends the intent and Rust validates it, assigns
/// ids, and applies it, so no rule lives in the UI.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
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
    AddCategory {
        name: String,
        target_weight: Decimal,
    },
    UpdateCategory {
        id: CategoryId,
        name: Option<String>,
        target_weight: Option<Decimal>,
        targeting: Option<ProductTargeting>,
    },
    RemoveCategory {
        id: CategoryId,
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
        kind: AccountKind,
    },
    UpdateAccount {
        id: AccountId,
        name: Option<String>,
        kind: Option<AccountKind>,
        sellable: Option<bool>,
        rounding_unit: Option<Decimal>,
    },
    SetTradable {
        id: AccountId,
        /// `None` means the account can buy any product.
        products: Option<Vec<ProductId>>,
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
    SetHolding {
        account: AccountId,
        product: ProductId,
        value: Decimal,
        quantity: Option<Decimal>,
    },
    RemoveHolding {
        account: AccountId,
        product: ProductId,
    },
    /// Sets (or clears) the relative weight of the record of a product in an account. A record
    /// that does not exist yet is made with a value of zero, so a product can be given a weight
    /// before anything is bought.
    SetHoldingWeight {
        account: AccountId,
        product: ProductId,
        weight: Option<Decimal>,
    },
    /// Makes the account hold exactly these products, as an import does. The
    /// values are in each product's own currency.
    ReplaceHoldings {
        account: AccountId,
        holdings: Vec<HoldingInput>,
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
        loose_numbers: bool,
        #[serde(default)]
        quantity_column: Option<usize>,
        #[serde(default)]
        quantity_part: Option<usize>,
        #[serde(default)]
        max_records: Option<usize>,
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
    SetExchangeRate {
        usd_jpy: Option<Decimal>,
    },
    SetTolerances {
        category: Decimal,
        product: Decimal,
    },
    SetTargetAccount {
        account: Option<AccountId>,
    },
}

/// One product's value in a [`Edit::ReplaceHoldings`].
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct HoldingInput {
    pub product: ProductId,
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
    pub(crate) fn validated(self) -> Result<Self, Error> {
        let source = name(&self.source, "import source")?;
        let invalid = |what: &str| Error::Invalid(format!("{what} is out of range"));
        if self.name_column >= MAX_COLUMN
            || self.value_column >= MAX_COLUMN
            || self.price_column.is_some_and(|c| c >= MAX_COLUMN)
            || self.quantity_column.is_some_and(|c| c >= MAX_COLUMN)
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
        let table_starts_with = self.table_starts_with.trim().to_owned();
        if table_starts_with.is_empty()
            || table_starts_with.chars().count() > MAX_LINK_TEXT_CHARS
            || table_starts_with.chars().any(char::is_control)
        {
            return Err(Error::Invalid(
                "the table title is not acceptable".to_owned(),
            ));
        }
        portfolio.account(self.account)?;
        let allowed_sites = allowed_sites(&self.allowed_sites)?;
        let start_site = start_site(&self.start_site)?;
        Ok(Self {
            name: recipe_name,
            links,
            table_starts_with,
            allowed_sites,
            start_site,
            // Empty: the currency of each product the rows are linked to.
            currency: if self.currency.trim().is_empty() {
                String::new()
            } else {
                currency(&self.currency)?
            },
            mapping: self.mapping.validated()?,
            ..self
        })
    }
}

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
                    target_weight,
                    targeting: ProductTargeting::default(),
                });
            }
            Edit::UpdateCategory {
                id,
                name: new_name,
                target_weight,
                targeting,
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
                if let Some(t) = targeting {
                    category.targeting = t;
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
            Edit::RemoveProduct { id } => {
                self.product(id)?;
                self.products.retain(|p| p.id != id);
                self.holdings.retain(|h| h.product != id);
                for account in &mut self.accounts {
                    if let Some(tradable) = &mut account.tradable {
                        tradable.retain(|p| *p != id);
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
            Edit::AddAccount {
                name: new_name,
                kind,
            } => {
                self.check_capacity(self.accounts.len())?;
                let new_name = name(&new_name, "account")?;
                self.ensure_account_name_free(&new_name, None)?;
                let id = AccountId(self.take_id());
                self.accounts.push(Account {
                    id,
                    name: new_name,
                    kind,
                    // NISA accounts default to no selling: selling would
                    // forfeit tax-free room.
                    sellable: kind != AccountKind::Nisa,
                    tradable: None,
                    rounding_unit: Decimal::ONE,
                });
            }
            Edit::UpdateAccount {
                id,
                name: new_name,
                kind,
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
                if let Some(k) = kind {
                    account.kind = k;
                }
                if let Some(s) = sellable {
                    account.sellable = s;
                }
                if let Some(u) = rounding_unit {
                    account.rounding_unit = u;
                }
            }
            Edit::SetTradable { id, products } => {
                self.account(id)?;
                if let Some(list) = &products {
                    for product in list {
                        self.product(*product)?;
                    }
                }
                let products = products.map(|mut list| {
                    list.sort();
                    list.dedup();
                    list
                });
                self.account_mut(id)?.tradable = products;
            }
            Edit::MoveAccount { id, up } => {
                self.account(id)?;
                let index = self.accounts.iter().position(|a| a.id == id).unwrap_or(0);
                let other = if up {
                    index.checked_sub(1)
                } else {
                    Some(index + 1)
                };
                if let Some(other) = other.filter(|o| *o < self.accounts.len()) {
                    self.accounts.swap(index, other);
                }
            }
            Edit::RemoveAccount { id } => {
                self.account(id)?;
                self.accounts.retain(|a| a.id != id);
                self.holdings.retain(|h| h.account != id);
                self.recipes.retain(|r| r.account != id);
                if self.settings.target_account == Some(id) {
                    self.settings.target_account = None;
                }
            }
            Edit::SetHolding {
                account,
                product,
                value,
                quantity,
            } => {
                let value = non_negative(value, "holding value")?;
                let quantity = quantity.map(|q| non_negative(q, "quantity")).transpose()?;
                self.account(account)?;
                self.product(product)?;
                match self
                    .holdings
                    .iter_mut()
                    .find(|h| h.account == account && h.product == product)
                {
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
                            weight: None,
                        });
                    }
                }
            }
            Edit::SetHoldingWeight {
                account,
                product,
                weight,
            } => {
                let weight = weight.map(|w| positive(w, "weight")).transpose()?;
                self.account(account)?;
                self.product(product)?;
                match self
                    .holdings
                    .iter_mut()
                    .find(|h| h.account == account && h.product == product)
                {
                    Some(holding) => holding.weight = weight,
                    None if weight.is_none() => {}
                    None => {
                        self.check_capacity(self.holdings.len())?;
                        self.holdings.push(Holding {
                            account,
                            product,
                            value: Decimal::ZERO,
                            quantity: None,
                            price: None,
                            price_unit: 1,
                            weight,
                        });
                    }
                }
            }
            Edit::RemoveHolding { account, product } => {
                self.holdings
                    .retain(|h| !(h.account == account && h.product == product));
            }
            Edit::ReplaceHoldings { account, holdings } => {
                self.account(account)?;
                let mut seen = std::collections::BTreeSet::new();
                let mut next = Vec::with_capacity(holdings.len());
                for input in holdings {
                    self.product(input.product)?;
                    if !seen.insert(input.product) {
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
                        value: non_negative(input.value, "holding value")?,
                        quantity: input
                            .quantity
                            .map(|q| non_negative(q, "quantity"))
                            .transpose()?,
                        price: input.price.map(|p| non_negative(p, "price")).transpose()?,
                        price_unit: input.price_unit,
                        // The weight is the user's wish, not part of what the page says.
                        weight: self
                            .holdings
                            .iter()
                            .find(|h| h.account == account && h.product == input.product)
                            .and_then(|h| h.weight),
                    });
                }
                // A record that has a weight stays when the page no longer lists the product
                // (sold out), with nothing held, so its weight is not lost.
                let kept: Vec<Holding> = self
                    .holdings
                    .iter()
                    .filter(|h| h.account == account && h.weight.is_some())
                    .filter(|h| !seen.contains(&h.product))
                    .map(|h| Holding {
                        value: Decimal::ZERO,
                        quantity: None,
                        price: None,
                        price_unit: 1,
                        ..h.clone()
                    })
                    .collect();
                next.extend(kept);
                let others = self
                    .holdings
                    .iter()
                    .filter(|h| h.account != account)
                    .count();
                if others + next.len() > MAX_ITEMS {
                    return Err(Error::Invalid("too many items".to_owned()));
                }
                self.holdings.retain(|h| h.account != account);
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
                loose_numbers,
                quantity_column,
                quantity_part,
                max_records,
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
                    loose_numbers,
                    quantity_column,
                    quantity_part,
                    max_records,
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
            Edit::SetTargetAccount { account } => {
                if let Some(account) = account {
                    self.account(account)?;
                }
                self.settings.target_account = account;
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
