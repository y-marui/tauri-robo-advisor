use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::{Portfolio, ProductId};

/// One line of imported data: a name the source uses, and its value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImportRow {
    pub name: String,
    pub value: Decimal,
    /// What the value was worked out from, when the table gave it.
    #[serde(default)]
    pub quantity: Option<Decimal>,
    #[serde(default)]
    pub price: Option<Decimal>,
    #[serde(default = "one")]
    pub price_unit: u64,
}

fn one() -> u64 {
    1
}

/// A product with the imported rows linked to it through aliases the user
/// confirmed. When several names are linked to one product, their values are
/// added up, so the product appears once.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MatchedRow {
    pub names: Vec<String>,
    pub product: ProductId,
    pub value: Decimal,
    /// The quantities added up, when every row had one.
    pub quantity: Option<Decimal>,
    /// The price, when every row had the same one (rows at different prices have no single price).
    pub price: Option<Decimal>,
    pub price_unit: u64,
}

/// What an import would do. Nothing here changes the portfolio.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ImportPlan {
    pub matched: Vec<MatchedRow>,
    /// Names with no alias yet; the user links each to a product, or leaves it out.
    pub unmatched: Vec<ImportRow>,
    /// Names linked to a product whose currency differs from the values'.
    pub wrong_currency: Vec<ImportRow>,
}

impl Portfolio {
    /// Links imported rows to products by alias. Names are matched exactly (after
    /// trimming); a name is never guessed. `currency` is the currency of the
    /// rows' values: a product in another currency would be valued wrongly, so
    /// it is reported instead of matched. An empty `currency` means "the product's own":
    /// the values are taken as they are in the currency of the product they are linked to.
    pub fn plan_import(&self, source: &str, currency: &str, rows: &[ImportRow]) -> ImportPlan {
        let mut plan = ImportPlan {
            matched: Vec::new(),
            unmatched: Vec::new(),
            wrong_currency: Vec::new(),
        };
        for row in rows {
            let name = row.name.trim();
            let product = self.products.iter().find(|p| {
                p.aliases
                    .iter()
                    .any(|a| a.source == source && a.name == name)
            });
            match product {
                None => plan.unmatched.push(row.clone()),
                Some(p) if !currency.is_empty() && p.currency != currency => {
                    plan.wrong_currency.push(row.clone());
                }
                Some(p) => match plan.matched.iter_mut().find(|m| m.product == p.id) {
                    Some(existing) => {
                        existing.names.push(name.to_owned());
                        existing.value += row.value;
                        existing.quantity = match (existing.quantity, row.quantity) {
                            (Some(a), Some(b)) => a.checked_add(b),
                            _ => None,
                        };
                        if existing.price != row.price || existing.price_unit != row.price_unit {
                            existing.price = None;
                            existing.price_unit = 1;
                        }
                    }
                    None => plan.matched.push(MatchedRow {
                        names: vec![name.to_owned()],
                        product: p.id,
                        value: row.value,
                        quantity: row.quantity,
                        price: row.price,
                        price_unit: row.price_unit,
                    }),
                },
            }
        }
        plan
    }
}
