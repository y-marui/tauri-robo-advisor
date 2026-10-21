use std::collections::BTreeMap;

use portfolio::{AccountId, CategoryId, Portfolio, ProductId, ProductTargeting};
use rust_decimal::Decimal;
use serde::Serialize;

use crate::Error;
use crate::engine::{MAX_AMOUNT, MAX_WEIGHT, to_jpy};
use crate::round::round_to_units;
use crate::strategy::{StrategyBalance, balance};

/// One product's value in the account against the share of the account's
/// holdings in its category that the product weights ask for.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ProductBalance {
    pub product: ProductId,
    /// What the account holds of the product, in JPY.
    #[serde(serialize_with = "crate::display::yen")]
    pub current: Decimal,
    /// What the weights ask for, in whole yen: an equal share for free and equal categories, the
    /// current value for keep-current. `None` only when the weights are unusable (all zero).
    #[serde(serialize_with = "crate::display::yen_option")]
    pub target: Option<Decimal>,
    /// `target` minus `current`, in whole yen: positive when the product is
    /// short, negative when there is too much. `None` when there is no target.
    #[serde(serialize_with = "crate::display::yen_option")]
    pub shortfall: Option<Decimal>,
}

/// The account's holdings in one category.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CategoryBalance {
    pub category: CategoryId,
    /// What the account holds in the category, in JPY.
    #[serde(serialize_with = "crate::display::yen")]
    pub total: Decimal,
    pub products: Vec<ProductBalance>,
}

/// How one account's holdings are spread over products, category by category.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AccountBalance {
    pub account: AccountId,
    pub categories: Vec<CategoryBalance>,
    /// For an account with a product strategy: its products against their ratios (the categories
    /// are then empty). `None` for an account that follows the portfolio.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strategy: Option<StrategyBalance>,
}

/// Compares the products inside one account with the product weights. The
/// split is only over the products the account holds, and the money
/// that is already in a category stays there; moving it between categories is
/// the suggestion's job, not this view's.
pub fn account_balance(portfolio: &Portfolio, account: AccountId) -> Result<AccountBalance, Error> {
    portfolio
        .validate_integrity()
        .map_err(|_| Error::Inconsistent)?;
    let account = portfolio
        .accounts
        .iter()
        .find(|a| a.id == account)
        .ok_or(Error::UnknownAccount)?;
    if !account.strategy.is_portfolio() {
        return Ok(AccountBalance {
            account: account.id,
            categories: Vec::new(),
            strategy: Some(balance(portfolio, account)?),
        });
    }
    if portfolio
        .holdings
        .iter()
        .any(|h| h.weight.is_some_and(|w| w > MAX_WEIGHT))
    {
        return Err(Error::TooLarge);
    }

    let mut held: BTreeMap<ProductId, Decimal> = BTreeMap::new();
    for holding in portfolio
        .holdings
        .iter()
        .filter(|h| h.account == account.id)
    {
        let product = portfolio
            .products
            .iter()
            .find(|p| p.id == holding.product)
            .ok_or(Error::Inconsistent)?;
        let value = to_jpy(portfolio, &product.currency, holding.value)?;
        *held.entry(product.id).or_default() += value;
    }
    if held.values().copied().sum::<Decimal>() > MAX_AMOUNT {
        return Err(Error::TooLarge);
    }

    let categories = portfolio
        .categories
        .iter()
        .map(|category| {
            let members: Vec<_> = portfolio
                .products
                .iter()
                .filter(|p| p.category == category.id)
                // A product the account does not hold is out of scope here: not shown, and it
                // gets no share of the category.
                .filter(|p| held.get(&p.id).is_some_and(|v| *v > Decimal::ZERO))
                .collect();
            let total: Decimal = members
                .iter()
                .map(|p| held.get(&p.id).copied().unwrap_or_default())
                .sum();
            let weights: Vec<Option<Decimal>> = members
                .iter()
                .map(|p| match category.targeting {
                    // Suggestions treat a free category as an equal split, so the table does too.
                    ProductTargeting::Equal | ProductTargeting::Free => Some(Decimal::ONE),
                    ProductTargeting::Weighted => Some(
                        portfolio
                            .record_weight(account.id, p.id)
                            .unwrap_or_default(),
                    ),
                    // Keep-current asks for the split it already has: the weights are the values.
                    ProductTargeting::KeepCurrent => {
                        Some(held.get(&p.id).copied().unwrap_or_default())
                    }
                })
                .collect();
            let weight_sum: Decimal = weights.iter().flatten().copied().sum();
            // Whole yen that add up to the category's total, so the shortfalls
            // never suggest money is missing or left over.
            let targets: Vec<Option<Decimal>> =
                if weight_sum > Decimal::ZERO && weights.iter().all(Option::is_some) {
                    let exact: Vec<Decimal> = weights
                        .iter()
                        .flatten()
                        .map(|w| total * *w / weight_sum)
                        .collect();
                    let caps = vec![None; exact.len()];
                    match round_to_units(&exact, Decimal::ONE, total.round_dp(0), &caps) {
                        Some(whole) => whole.into_iter().map(Some).collect(),
                        None => vec![None; exact.len()],
                    }
                } else {
                    vec![None; members.len()]
                };
            let products = members
                .iter()
                .zip(&targets)
                .map(|(product, target)| {
                    let current = held.get(&product.id).copied().unwrap_or_default();
                    ProductBalance {
                        product: product.id,
                        current,
                        target: *target,
                        shortfall: target.map(|t| (t - current).round_dp(0)),
                    }
                })
                .collect();
            CategoryBalance {
                category: category.id,
                total,
                products,
            }
        })
        .collect();
    Ok(AccountBalance {
        account: account.id,
        categories,
        strategy: None,
    })
}
