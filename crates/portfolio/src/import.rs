use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::{Portfolio, ProductId};

/// One line of imported data: a name the source uses, and its value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImportRow {
    pub name: String,
    pub value: Decimal,
}

/// A product with the imported rows linked to it through aliases the user
/// confirmed. When several names are linked to one product, their values are
/// added up, so the product appears once.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MatchedRow {
    pub names: Vec<String>,
    pub product: ProductId,
    pub value: Decimal,
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
    /// it is reported instead of matched.
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
                Some(p) if p.currency != currency => plan.wrong_currency.push(row.clone()),
                Some(p) => match plan.matched.iter_mut().find(|m| m.product == p.id) {
                    Some(existing) => {
                        existing.names.push(name.to_owned());
                        existing.value += row.value;
                    }
                    None => plan.matched.push(MatchedRow {
                        names: vec![name.to_owned()],
                        product: p.id,
                        value: row.value,
                    }),
                },
            }
        }
        plan
    }
}
