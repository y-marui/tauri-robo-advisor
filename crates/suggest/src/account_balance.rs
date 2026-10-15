use std::collections::BTreeMap;

use portfolio::{AccountId, CategoryId, Portfolio, ProductId, ProductTargeting};
use rust_decimal::Decimal;
use serde::Serialize;

use crate::Error;
use crate::engine::{MAX_AMOUNT, MAX_WEIGHT, to_jpy};

/// One product's value in the account against the share of the account's
/// holdings in its category that the product weights ask for.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ProductBalance {
    pub product: ProductId,
    /// What the account holds of the product, in JPY.
    pub current: Decimal,
    /// What the weights ask for, in whole yen. `None` when the category does
    /// not ask for a split (free, keep-current) or has no usable weights.
    pub target: Option<Decimal>,
    /// `target` minus `current`, in whole yen: positive when the product is
    /// short, negative when there is too much. `None` when there is no target.
    pub shortfall: Option<Decimal>,
}

/// The account's holdings in one category.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CategoryBalance {
    pub category: CategoryId,
    /// What the account holds in the category, in JPY.
    pub total: Decimal,
    pub products: Vec<ProductBalance>,
}

/// How one account's holdings are spread over products, category by category.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AccountBalance {
    pub account: AccountId,
    pub categories: Vec<CategoryBalance>,
}

/// Compares the products inside one account with the product weights. The
/// split is only over the products the account holds or can buy, and the money
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
    if portfolio
        .products
        .iter()
        .any(|p| p.weight.is_some_and(|w| w > MAX_WEIGHT))
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
                .filter(|p| {
                    held.get(&p.id).is_some_and(|v| *v > Decimal::ZERO)
                        || account
                            .tradable
                            .as_ref()
                            .is_none_or(|list| list.contains(&p.id))
                })
                .collect();
            let total: Decimal = members
                .iter()
                .map(|p| held.get(&p.id).copied().unwrap_or_default())
                .sum();
            let weights: Vec<Option<Decimal>> = members
                .iter()
                .map(|p| match category.targeting {
                    ProductTargeting::Equal => Some(Decimal::ONE),
                    ProductTargeting::Weighted => Some(p.weight.unwrap_or_default()),
                    ProductTargeting::Free | ProductTargeting::KeepCurrent => None,
                })
                .collect();
            let weight_sum: Decimal = weights.iter().flatten().copied().sum();
            let products = members
                .iter()
                .zip(&weights)
                .map(|(product, weight)| {
                    let current = held.get(&product.id).copied().unwrap_or_default();
                    let target = weight
                        .filter(|_| weight_sum > Decimal::ZERO)
                        .map(|w| (total * w / weight_sum).round_dp(0));
                    ProductBalance {
                        product: product.id,
                        current,
                        target,
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
    })
}
