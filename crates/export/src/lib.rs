//! A full-record export of a portfolio, so it can be analysed in other software.
//!
//! The result is a set of files, each as bytes: CSV tables for holdings,
//! categories, products, aliases, accounts, and settings, a JSON file with the
//! complete data, and a README describing them. The crate only builds the
//! contents; writing them to disk is the app's job.
//!
//! Spreadsheets run cells that start with `=`, `+`, `-`, or `@` as formulas, so
//! every name the user typed is written as plain text (see [`csv::text_cell`]).

mod csv;

use portfolio::{Account, Category, Portfolio, Product};
use rust_decimal::Decimal;

use csv::{Table, number, optional_number, plain_cell, text_cell};

/// One file of the export.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportFile {
    pub name: &'static str,
    pub content: Vec<u8>,
}

/// Facts about the export that go into the README.
pub struct Info<'a> {
    pub app_version: &'a str,
    /// When the export was made, as an ISO 8601 UTC time such as `2026-10-15T01:23:45Z`.
    pub exported_at: &'a str,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("could not encode the data")]
    Encode,
}

/// Builds every file of the export.
pub fn build(portfolio: &Portfolio, info: &Info) -> Result<Vec<ExportFile>, Error> {
    let values = Values::new(portfolio);
    let json = serde_json::to_vec_pretty(portfolio).map_err(|_| Error::Encode)?;
    let mut json = json;
    json.push(b'\n');

    Ok(vec![
        file("holdings.csv", holdings(portfolio, &values)),
        file("categories.csv", categories(portfolio, &values)),
        file("products.csv", products(portfolio, &values)),
        file("aliases.csv", aliases(portfolio)),
        file("accounts.csv", accounts(portfolio, &values)),
        file("settings.csv", settings(portfolio)),
        ExportFile {
            name: "portfolio.json",
            content: json,
        },
        ExportFile {
            name: "README.txt",
            content: readme(portfolio, info).into_bytes(),
        },
    ])
}

fn file(name: &'static str, table: Table) -> ExportFile {
    ExportFile {
        name,
        content: table.to_bytes(),
    }
}

/// Every holding's value in JPY, and the totals the shares are based on.
struct Values {
    /// Indexed like `portfolio.holdings`; `None` when the value cannot be
    /// converted (no exchange rate, or an unsupported currency).
    jpy: Vec<Option<Decimal>>,
    /// The sum of all values; `None` when any value cannot be converted or the
    /// sum would overflow.
    total: Option<Decimal>,
}

impl Values {
    fn new(portfolio: &Portfolio) -> Self {
        let jpy: Vec<Option<Decimal>> = portfolio
            .holdings
            .iter()
            .map(|holding| {
                let product = portfolio
                    .products
                    .iter()
                    .find(|p| p.id == holding.product)?;
                portfolio
                    .value_in_jpy(&product.currency, holding.value)
                    .ok()
            })
            .collect();
        // A share against a partial total would be wrong, so the total exists
        // only when every holding could be converted.
        let total = jpy
            .iter()
            .try_fold(Decimal::ZERO, |sum, value| sum.checked_add((*value)?));
        Self { jpy, total }
    }

    /// The converted value of the holdings selected by `keep`, or `None` when
    /// any of them cannot be converted (a partial sum would mislead).
    fn sum(
        &self,
        portfolio: &Portfolio,
        keep: impl Fn(&portfolio::Holding) -> bool,
    ) -> Option<Decimal> {
        let mut sum = Decimal::ZERO;
        for (holding, value) in portfolio.holdings.iter().zip(&self.jpy) {
            if keep(holding) {
                sum = sum.checked_add((*value)?)?;
            }
        }
        Some(sum)
    }

    /// `value` as a fraction of the total, to six decimal places.
    fn share(&self, value: Option<Decimal>) -> Option<Decimal> {
        let total = self.total?;
        if total.is_zero() {
            return None;
        }
        Some(value?.checked_div(total)?.round_dp(6))
    }
}

fn holdings(portfolio: &Portfolio, values: &Values) -> Table {
    let mut table = Table::new(&[
        "account_id",
        "account",
        "account_kind",
        "product_id",
        "product",
        "category_id",
        "category",
        "currency",
        "value",
        "quantity",
        "value_jpy",
        "share_of_total",
    ]);
    for (holding, jpy) in portfolio.holdings.iter().zip(&values.jpy) {
        let account = portfolio.accounts.iter().find(|a| a.id == holding.account);
        let product = portfolio.products.iter().find(|p| p.id == holding.product);
        let category =
            product.and_then(|p| portfolio.categories.iter().find(|c| c.id == p.category));
        table.push(vec![
            plain_cell(&holding.account.0.to_string()),
            text_cell(account.map_or("", |a| &a.name)),
            plain_cell(&account.map(|a| keyword(&a.kind)).unwrap_or_default()),
            plain_cell(&holding.product.0.to_string()),
            text_cell(product.map_or("", |p| &p.name)),
            plain_cell(&category.map_or(String::new(), |c| c.id.0.to_string())),
            text_cell(category.map_or("", |c| &c.name)),
            plain_cell(product.map_or("", |p| &p.currency)),
            number(holding.value),
            optional_number(holding.quantity),
            optional_number(*jpy),
            optional_number(values.share(*jpy)),
        ]);
    }
    table
}

fn categories(portfolio: &Portfolio, values: &Values) -> Table {
    let mut table = Table::new(&[
        "category_id",
        "category",
        "target_weight",
        "product_targeting",
        "value_jpy",
        "current_share",
    ]);
    for category in &portfolio.categories {
        let value = values.sum(portfolio, |h| {
            product_of(portfolio, h.product).is_some_and(|p| p.category == category.id)
        });
        table.push(vec![
            plain_cell(&category.id.0.to_string()),
            text_cell(&category.name),
            number(category.target_weight),
            plain_cell(&keyword(&category.targeting)),
            optional_number(value),
            optional_number(values.share(value)),
        ]);
    }
    table
}

fn products(portfolio: &Portfolio, values: &Values) -> Table {
    let mut table = Table::new(&[
        "product_id",
        "product",
        "category_id",
        "category",
        "currency",
        "relative_weight",
        "value_jpy",
    ]);
    for product in &portfolio.products {
        let category: Option<&Category> = portfolio
            .categories
            .iter()
            .find(|c| c.id == product.category);
        let value = values.sum(portfolio, |h| h.product == product.id);
        table.push(vec![
            plain_cell(&product.id.0.to_string()),
            text_cell(&product.name),
            plain_cell(&product.category.0.to_string()),
            text_cell(category.map_or("", |c| &c.name)),
            plain_cell(&product.currency),
            optional_number(product.weight),
            optional_number(value),
        ]);
    }
    table
}

fn aliases(portfolio: &Portfolio) -> Table {
    let mut table = Table::new(&["product_id", "product", "source", "alias"]);
    for product in &portfolio.products {
        for alias in &product.aliases {
            table.push(vec![
                plain_cell(&product.id.0.to_string()),
                text_cell(&product.name),
                text_cell(&alias.source),
                text_cell(&alias.name),
            ]);
        }
    }
    table
}

fn accounts(portfolio: &Portfolio, values: &Values) -> Table {
    let mut table = Table::new(&[
        "account_id",
        "account",
        "kind",
        "sellable",
        "funding",
        "rounding_unit",
        "tradable_product_ids",
        "value_jpy",
    ]);
    for account in &portfolio.accounts {
        let value = values.sum(portfolio, |h| h.account == account.id);
        table.push(vec![
            plain_cell(&account.id.0.to_string()),
            text_cell(&account.name),
            plain_cell(&keyword(&account.kind)),
            plain_cell(if account.sellable { "true" } else { "false" }),
            plain_cell(&keyword(&account.funding)),
            number(account.rounding_unit),
            plain_cell(&tradable(account)),
            optional_number(value),
        ]);
    }
    table
}

fn settings(portfolio: &Portfolio) -> Table {
    let s = &portfolio.settings;
    let target = s
        .target_account
        .and_then(|id| portfolio.accounts.iter().find(|a| a.id == id));
    let mut table = Table::new(&["setting", "value"]);
    let mut row = |key: &str, value: String| table.push(vec![plain_cell(key), value]);
    row("schema_version", portfolio.schema_version.to_string());
    row("usd_jpy", optional_number(s.usd_jpy));
    row("category_tolerance", number(s.category_tolerance));
    row("product_tolerance", number(s.product_tolerance));
    row(
        "rebalancing_account_id",
        target.map_or(String::new(), |a| a.id.0.to_string()),
    );
    row(
        "rebalancing_account",
        target.map_or(String::new(), |a| text_cell(&a.name)),
    );
    table
}

fn tradable(account: &Account) -> String {
    match &account.tradable {
        None => "all".to_owned(),
        Some(ids) => ids
            .iter()
            .map(|id| id.0.to_string())
            .collect::<Vec<_>>()
            .join(";"),
    }
}

fn product_of(portfolio: &Portfolio, id: portfolio::ProductId) -> Option<&Product> {
    portfolio.products.iter().find(|p| p.id == id)
}

/// The snake_case keyword an enum is stored with, such as `nisa`.
fn keyword<T: serde::Serialize>(value: &T) -> String {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::String(text)) => text,
        _ => String::new(),
    }
}

fn readme(portfolio: &Portfolio, info: &Info) -> String {
    format!(
        "tauri-robo-advisor full export\n\
         ================================\n\
         \n\
         App version:    {version}\n\
         Exported at:    {at} (UTC)\n\
         Schema version: {schema}\n\
         \n\
         WARNING: these files contain your holdings and your portfolio targets.\n\
         Keep them private and delete them when you no longer need them.\n\
         \n\
         Files\n\
         -----\n\
         holdings.csv     One row per product held in an account (the amounts you entered).\n\
         categories.csv   Categories with their target weight and current value.\n\
         products.csv     Products with category, currency, and relative weight.\n\
         aliases.csv      Names that data sources use for a product.\n\
         accounts.csv     Accounts with their rules (sellable, tradable products, rounding unit).\n\
         settings.csv     Exchange rate, tolerances, and the rebalancing account.\n\
         portfolio.json   The complete data in the app's own format (all of the above).\n\
         \n\
         Notes\n\
         -----\n\
         * CSV files are UTF-8 with a byte order mark and CRLF line ends, so Excel opens them correctly.\n\
         * `value` is in the product's own currency. `value_jpy` converts JPY as is and USD with the\n\
           USD/JPY rate in settings.csv; it is empty when a value cannot be converted (no rate set,\n\
           or another currency). Totals are empty when any value they include is empty.\n\
         * Weights, shares, and tolerances are fractions of 1 (0.25 means 25%). `share_of_total` and\n\
           `current_share` are rounded to six decimal places.\n\
         * Names are written as text. A name that starts with = + - or @ is prefixed with an\n\
           apostrophe so a spreadsheet does not run it as a formula; remove the apostrophe if you\n\
           need the original name.\n\
         * `tradable_product_ids` is `all` or a list of product ids separated by semicolons.\n\
         * Ids are stable numbers and are never reused.\n\
         \n\
         This export has {categories} categories, {products} products, {accounts} accounts, and {holdings} holdings.\n",
        version = info.app_version,
        at = info.exported_at,
        schema = portfolio.schema_version,
        categories = portfolio.categories.len(),
        products = portfolio.products.len(),
        accounts = portfolio.accounts.len(),
        holdings = portfolio.holdings.len(),
    )
}

#[cfg(test)]
mod tests;
