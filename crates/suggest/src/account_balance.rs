use std::collections::BTreeMap;

use portfolio::{AccountId, CategoryId, Portfolio, ProductId, SuggestionMode};
use rust_decimal::Decimal;
use serde::Serialize;

use crate::Error;
use crate::engine::{MAX_AMOUNT, to_jpy};

/// The product settings the balance table follows: the fixed-add mode's.
const BALANCE_MODE: SuggestionMode = SuggestionMode::FixedAdd;
use crate::round::round_to_units;
use crate::split::{TypeValue, type_values};
use crate::strategy::{StrategyBalance, balance};

/// One product's value in the account against the share of the account's
/// holdings in its category that the product weights ask for.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ProductBalance {
    pub product: ProductId,
    /// What the account holds of the product, in JPY.
    #[serde(serialize_with = "crate::display::yen")]
    pub current: Decimal,
    /// `current` split by the type of the records, in the order a sale takes them (records
    /// without a type first, the NISA family last). Empty when the account does not hold it.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub types: Vec<TypeValue>,
    /// What the weights ask for, in whole yen: the category's holdings of the account split by
    /// the product weights. `None` only when the weights are unusable (all zero).
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

    let mut types = type_values(portfolio);
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
            let weights: Vec<Decimal> = members
                .iter()
                .map(|p| portfolio.product_weight(BALANCE_MODE, p.id))
                .collect();
            let weight_sum: Decimal = weights.iter().copied().sum();
            // Whole yen that add up to the total, so the shortfalls never suggest money is missing
            // or left over.
            let targets: Vec<Option<Decimal>> = if weight_sum > Decimal::ZERO {
                let exact: Vec<Decimal> = weights.iter().map(|w| total * *w / weight_sum).collect();
                let caps = vec![None; exact.len()];
                match round_to_units(&exact, Decimal::ONE, total.round_dp(0), &caps) {
                    Some(whole) => whole.into_iter().map(Some).collect(),
                    None => vec![None; members.len()],
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
                        types: types.remove(&(account.id, product.id)).unwrap_or_default(),
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
