use rust_decimal::Decimal;
use serde::Deserialize;

use crate::presets::Preset;
use crate::validate::{currency, fraction, name, non_negative, positive};
use crate::{
    Account, AccountId, AccountKind, Alias, Category, CategoryId, Error, FetchLink, Funding,
    Holding, ImportMapping, Portfolio, Product, ProductId, ProductTargeting, Recipe,
};

/// One user action. The frontend sends the intent and Rust validates it, assigns
/// ids, and applies it, so no rule lives in the UI.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Edit {
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
    SetProductWeight {
        id: ProductId,
        weight: Option<Decimal>,
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
        funding: Option<Funding>,
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
    /// Adds a recipe that ships with the app (see `presets`) for an account. The recipe is an
    /// ordinary one afterwards: the user can read it, change it, or remove it. A recipe of the
    /// same name for the same account is replaced; for another account, the account's name is
    /// added to the recipe's name.
    AddPresetRecipe {
        preset: String,
        account: AccountId,
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

/// Sites that are shared by many unrelated owners, so a recipe may not be allowed to follow
/// links across all of them. (The list is short on purpose: the site must also contain the host
/// the recipe starts on, which is checked when the recipe runs.)
const SHARED_SUFFIXES: &[&str] = &[
    "co.jp",
    "or.jp",
    "ne.jp",
    "ac.jp",
    "go.jp",
    "ed.jp",
    "gr.jp",
    "lg.jp",
    "ad.jp",
    "co.uk",
    "org.uk",
    "ac.uk",
    "gov.uk",
    "com.au",
    "net.au",
    "org.au",
    "co.nz",
    "com.cn",
    "com.hk",
    "com.tw",
    "co.kr",
    "co.in",
    "com.br",
    "com.sg",
    "github.io",
    "herokuapp.com",
    "cloudfront.net",
    "amazonaws.com",
    "azurewebsites.net",
    "appspot.com",
    "pages.dev",
    "vercel.app",
    "netlify.app",
];
const MAX_ALLOWED_SITES: usize = 3;

/// Checks the allowed sites of a recipe: plain lower-case host names with at least two labels
/// (no scheme, path, port, or wildcard), not a suffix shared by many owners. Duplicates go.
fn allowed_sites(sites: &[String]) -> Result<Vec<String>, Error> {
    let bad = || Error::Invalid("an allowed site is not acceptable".to_owned());
    if sites.len() > MAX_ALLOWED_SITES {
        return Err(Error::Invalid(
            "a recipe has too many allowed sites".to_owned(),
        ));
    }
    let mut out: Vec<String> = Vec::new();
    for site in sites {
        let site = site.trim().to_lowercase();
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
        if !plain || SHARED_SUFFIXES.contains(&site.as_str()) {
            return Err(bad());
        }
        if !out.contains(&site) {
            out.push(site);
        }
    }
    Ok(out)
}

impl Recipe {
    /// Checks a recipe and trims its names.
    pub(crate) fn validated(self, portfolio: &Portfolio) -> Result<Self, Error> {
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
            links.push(FetchLink { text, reference });
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
        Ok(Self {
            name: recipe_name,
            links,
            table_starts_with,
            allowed_sites,
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
                self.check_capacity(self.products.len())?;
                let new_name = name(&new_name, "product")?;
                let code = currency(&code)?;
                self.category(category)?;
                self.ensure_product_name_free(&new_name, None)?;
                let id = ProductId(self.take_id());
                self.products.push(Product {
                    id,
                    name: new_name,
                    category,
                    currency: code,
                    weight: None,
                    aliases: Vec::new(),
                });
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
            Edit::SetProductWeight { id, weight } => {
                let weight = weight.map(|w| positive(w, "product weight")).transpose()?;
                self.product_mut(id)?.weight = weight;
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
                let alias = Alias {
                    source: name(&source, "alias source")?,
                    name: name(&alias_name, "alias")?,
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
                    funding: if kind == AccountKind::Points {
                        Funding::Points
                    } else {
                        Funding::Cash
                    },
                    rounding_unit: Decimal::new(1000, 0),
                });
            }
            Edit::UpdateAccount {
                id,
                name: new_name,
                kind,
                sellable,
                funding,
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
                if let Some(f) = funding {
                    account.funding = f;
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
                    });
                }
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
                let mut recipe = preset.recipe(account);
                let account_name = self.account(account)?.name.clone();
                // The same preset for the same account is found by what it does, not by its
                // name (the name may have been suffixed, or changed by the user): it is replaced
                // in place, so the account never gets two copies that would add up twice.
                let same = self.recipes.iter().position(|r| {
                    r.account == account
                        && r.table_starts_with == recipe.table_starts_with
                        && r.mapping.source == recipe.mapping.source
                        && r.links
                            .iter()
                            .map(|l| &l.text)
                            .eq(recipe.links.iter().map(|l| &l.text))
                });
                if let Some(index) = same {
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
