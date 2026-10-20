use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::{Portfolio, ProductId, minor_units};

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
    /// When the value is an evaluation that the table showed next to a quantity and a price:
    /// quantity x price / unit, to check the value against.
    #[serde(default)]
    pub computed: Option<Decimal>,
    /// The type the table title said the record has (see `Recipe::type_rules`); `None`: not read.
    #[serde(default)]
    pub account_type: Option<String>,
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
    /// The type the rows were read with; rows of one product but of two types are two matches.
    pub account_type: Option<String>,
}

/// What an import would do. Nothing here changes the portfolio.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ImportPlan {
    pub matched: Vec<MatchedRow>,
    /// Names with no alias yet; the user links each to a product, or leaves it out.
    pub unmatched: Vec<ImportRow>,
    /// Names linked to a product whose currency differs from the values'.
    pub wrong_currency: Vec<ImportRow>,
    /// Rows whose evaluation differs from quantity x price by more than the smallest unit of the
    /// currency. The evaluation stays the value; the person looks before applying.
    pub differences: Vec<ValueDifference>,
    /// For unmatched names that have a product of the same or a similar name: a hint for the
    /// link step. Only a suggestion; nothing is linked by it.
    pub suggestions: Vec<LinkSuggestion>,
}

/// An existing product that an unmatched name may belong to (see `name_match`).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LinkSuggestion {
    pub name: String,
    /// A product with the same name (trimmed, ignoring case).
    pub exact: Option<ProductId>,
    /// Else a product whose name is similar; offered first, never linked on its own.
    pub similar: Option<ProductId>,
}

/// Letters and digits only, lowercased: "Poly-gon" and "poly gon" both become "polygon".
fn normalized(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// How a name relates to a product name. `Exact`: equal after trimming, ignoring case.
/// `Similar`: equal after dropping everything but letters and digits, or one contains the other
/// (the shorter one needs two characters, so a single letter hints nothing). Deliberately
/// simple: it is a hint for a person, not a decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum NameMatch {
    Containing,
    Normalized,
    Exact,
}

fn name_match(name: &str, product: &str) -> Option<NameMatch> {
    let (name, product) = (name.trim(), product.trim());
    if name.to_lowercase() == product.to_lowercase() {
        return Some(NameMatch::Exact);
    }
    let (a, b) = (normalized(name), normalized(product));
    if a.is_empty() || b.is_empty() {
        return None;
    }
    if a == b {
        return Some(NameMatch::Normalized);
    }
    let (short, long) = if a.len() <= b.len() {
        (&a, &b)
    } else {
        (&b, &a)
    };
    (short.chars().count() >= 2 && long.contains(short.as_str())).then_some(NameMatch::Containing)
}

/// A row whose read value and computed value (quantity x price / unit) disagree.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ValueDifference {
    pub name: String,
    pub value: Decimal,
    pub computed: Decimal,
}

impl Portfolio {
    /// The products an unmatched name may belong to: only products that could be linked
    /// (those in `currency`, when it is set). The best relation wins, the first product on a tie.
    fn suggest(&self, name: &str, currency: &str) -> Option<LinkSuggestion> {
        let mut best: Option<(NameMatch, ProductId)> = None;
        for p in &self.products {
            if !currency.is_empty() && p.currency != currency {
                continue;
            }
            if let Some(m) = name_match(name, &p.name)
                && best.is_none_or(|(b, _)| m > b)
            {
                best = Some((m, p.id));
            }
        }
        let (m, id) = best?;
        Some(LinkSuggestion {
            name: name.to_owned(),
            exact: (m == NameMatch::Exact).then_some(id),
            similar: (m != NameMatch::Exact).then_some(id),
        })
    }

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
            differences: Vec::new(),
            suggestions: Vec::new(),
        };
        for row in rows {
            let name = row.name.trim();
            let product = self.products.iter().find(|p| {
                p.aliases
                    .iter()
                    .any(|a| a.source == source && a.name == name)
            });
            // The currency of the values: the recipe's, else the linked product's, else a whole
            // unit (the loosest check) when nothing says.
            let unit_currency = if currency.is_empty() {
                product.map_or("", |p| p.currency.as_str())
            } else {
                currency
            };
            if let Some(computed) = row.computed {
                let places = minor_units(unit_currency).unwrap_or(0);
                if (row.value - computed).abs() > Decimal::new(1, places) {
                    plan.differences.push(ValueDifference {
                        name: name.to_owned(),
                        value: row.value,
                        computed,
                    });
                }
            }
            match product {
                None => {
                    plan.suggestions.extend(self.suggest(name, currency));
                    plan.unmatched.push(row.clone());
                }
                Some(p) if !currency.is_empty() && p.currency != currency => {
                    plan.wrong_currency.push(row.clone());
                }
                Some(p) => match plan
                    .matched
                    .iter_mut()
                    .find(|m| m.product == p.id && m.account_type == row.account_type)
                {
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
                        account_type: row.account_type.clone(),
                    }),
                },
            }
        }
        plan
    }
}
