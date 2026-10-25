use std::collections::BTreeMap;

use portfolio::{AccountId, CategoryId, Portfolio, ProductId, SuggestionMode};
use rust_decimal::Decimal;
use serde::Serialize;

use crate::Error;
use crate::engine::{MAX_AMOUNT, to_jpy};

use crate::split::{TypeValue, type_values};
use crate::strategy::{StrategyBalance, balance};

/// One product's value in the account.
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
    /// For a product held in several types: the sum of the weights of the account's records per
    /// mode on the account's own tab. Empty otherwise.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub own_tab_weights: BTreeMap<SuggestionMode, Decimal>,
    /// The same on the All tab.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub all_tab_weights: BTreeMap<SuggestionMode, Decimal>,
}

/// The sums of an account's record weights of a product per mode on the given tab; empty unless
/// the account holds the product in several types.
pub(crate) fn weight_sums(
    portfolio: &Portfolio,
    account: AccountId,
    product: ProductId,
    tab: Option<AccountId>,
    types: &[TypeValue],
) -> BTreeMap<SuggestionMode, Decimal> {
    if types.len() < 2 {
        return BTreeMap::new();
    }
    SuggestionMode::ALL
        .iter()
        .filter_map(|mode| {
            portfolio
                .account_product_weight(*mode, product, tab, account)
                .map(|sum| (*mode, sum))
        })
        .collect()
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

/// Lists the products an account holds, category by category, with their values and the
/// per-type split. Only the products the account holds are listed.
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
            let products = members
                .iter()
                .map(|product| {
                    let current = held.get(&product.id).copied().unwrap_or_default();
                    let own = types.remove(&(account.id, product.id)).unwrap_or_default();
                    ProductBalance {
                        product: product.id,
                        current,
                        own_tab_weights: weight_sums(
                            portfolio,
                            account.id,
                            product.id,
                            Some(account.id),
                            &own,
                        ),
                        all_tab_weights: weight_sums(portfolio, account.id, product.id, None, &own),
                        types: own,
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
