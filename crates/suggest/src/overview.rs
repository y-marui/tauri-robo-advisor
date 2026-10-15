use std::collections::BTreeMap;

use portfolio::{AccountId, CategoryId, Portfolio, ProductId};
use rust_decimal::Decimal;
use serde::Serialize;

use crate::Error;
use crate::engine::{MAX_AMOUNT, MAX_WEIGHT, to_jpy};

/// Shares are shown to four decimal places of a fraction (0.01 percentage points).
const SHARE_PLACES: u32 = 4;

/// One category's value and share of the whole, next to its target.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CategoryOverview {
    pub category: CategoryId,
    /// The category's value in JPY.
    pub current: Decimal,
    /// `current` divided by the total, as a fraction.
    pub current_share: Decimal,
    pub target_share: Decimal,
    /// `current_share` minus `target_share`: positive when over target.
    pub difference: Decimal,
}

/// One holding record valued in JPY.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct HoldingOverview {
    pub account: AccountId,
    pub product: ProductId,
    /// The holding's value in JPY.
    pub value: Decimal,
    /// `value` divided by the portfolio total, as a fraction.
    pub share: Decimal,
}

/// The whole portfolio valued in JPY, for the balance charts.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Overview {
    pub total: Decimal,
    pub categories: Vec<CategoryOverview>,
    pub holdings: Vec<HoldingOverview>,
}

/// Values every category in JPY. Unlike a suggestion this works while the
/// targets are incomplete, so the charts can show a half-entered portfolio.
pub fn overview(portfolio: &Portfolio) -> Result<Overview, Error> {
    portfolio
        .validate_integrity()
        .map_err(|_| Error::Inconsistent)?;
    if portfolio
        .products
        .iter()
        .any(|p| p.weight.is_some_and(|w| w > MAX_WEIGHT))
    {
        return Err(Error::TooLarge);
    }

    let mut values: BTreeMap<CategoryId, Decimal> = BTreeMap::new();
    let mut rows: Vec<(AccountId, ProductId, Decimal)> = Vec::new();
    for holding in &portfolio.holdings {
        let product = portfolio
            .products
            .iter()
            .find(|p| p.id == holding.product)
            .ok_or(Error::Inconsistent)?;
        let value = to_jpy(portfolio, &product.currency, holding.value)?;
        *values.entry(product.category).or_default() += value;
        rows.push((holding.account, product.id, value));
    }
    let total: Decimal = values.values().copied().sum();
    if total > MAX_AMOUNT {
        return Err(Error::TooLarge);
    }

    let share_of_total = |value: Decimal| {
        if total.is_zero() {
            Decimal::ZERO
        } else {
            (value / total).round_dp(SHARE_PLACES)
        }
    };

    let categories = portfolio
        .categories
        .iter()
        .map(|category| {
            let current = values.get(&category.id).copied().unwrap_or_default();
            let current_share = share_of_total(current);
            CategoryOverview {
                category: category.id,
                current,
                current_share,
                target_share: category.target_weight,
                difference: current_share - category.target_weight,
            }
        })
        .collect();
    let holdings = rows
        .into_iter()
        .map(|(account, product, value)| HoldingOverview {
            account,
            product,
            value,
            share: share_of_total(value),
        })
        .collect();
    Ok(Overview {
        total,
        categories,
        holdings,
    })
}
