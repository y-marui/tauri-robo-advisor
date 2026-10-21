use std::collections::BTreeMap;

use portfolio::{AccountId, CategoryId, GroupId, Portfolio, ProductId};
use rust_decimal::Decimal;
use serde::Serialize;

use crate::Error;
use crate::engine::{MAX_AMOUNT, to_jpy};

/// Shares are shown to four decimal places of a fraction (0.01 percentage points).
const SHARE_PLACES: u32 = 4;

/// One category's value and share of the whole, next to its target.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CategoryOverview {
    pub category: CategoryId,
    /// The category's value in JPY.
    #[serde(serialize_with = "crate::display::yen")]
    pub current: Decimal,
    /// `current` divided by the total, as a fraction.
    pub current_share: Decimal,
    pub target_share: Decimal,
    /// `current_share` minus `target_share`: positive when over target.
    pub difference: Decimal,
}

/// One group's value and share of the whole, next to its target. A group has no target of its
/// own: its target and its current value are the sums of its categories'.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GroupOverview {
    /// `None`: the categories that belong to no group, shown together as "Ungrouped".
    pub group: Option<GroupId>,
    /// The value of the group's categories in JPY.
    #[serde(serialize_with = "crate::display::yen")]
    pub current: Decimal,
    /// `current` divided by the total, as a fraction.
    pub current_share: Decimal,
    /// The sum of the targets of the group's categories.
    pub target_share: Decimal,
    /// `current_share` minus `target_share`: positive when over target.
    pub difference: Decimal,
}

/// One holding record valued in JPY.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct HoldingOverview {
    pub account: AccountId,
    pub product: ProductId,
    /// The type of the record (`None`: not set).
    pub account_type: Option<String>,
    /// The holding's value in JPY.
    #[serde(serialize_with = "crate::display::yen")]
    pub value: Decimal,
    /// `value` divided by the portfolio total, as a fraction.
    pub share: Decimal,
}

/// One account's holdings valued in JPY (every account, also one without holdings).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AccountOverview {
    pub account: AccountId,
    /// The value of the account's holdings in JPY.
    #[serde(serialize_with = "crate::display::yen")]
    pub value: Decimal,
    /// `value` divided by the portfolio total, as a fraction.
    pub share: Decimal,
}

/// The records of one type valued in JPY (only types that have records).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TypeOverview {
    /// `None`: the records without a type.
    pub account_type: Option<String>,
    #[serde(serialize_with = "crate::display::yen")]
    pub value: Decimal,
    /// `value` divided by the portfolio total, as a fraction.
    pub share: Decimal,
}

/// The whole portfolio valued in JPY, for the balance charts.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Overview {
    #[serde(serialize_with = "crate::display::yen")]
    pub total: Decimal,
    /// Empty when no group exists. Otherwise one entry per group in order, then the ungrouped
    /// categories (only if there are any).
    pub groups: Vec<GroupOverview>,
    pub categories: Vec<CategoryOverview>,
    pub holdings: Vec<HoldingOverview>,
    pub accounts: Vec<AccountOverview>,
    /// Ordered by type id, the records without a type first.
    pub types: Vec<TypeOverview>,
}

/// Values every category in JPY. Unlike a suggestion this works while the
/// targets are incomplete, so the charts can show a half-entered portfolio.
pub fn overview(portfolio: &Portfolio) -> Result<Overview, Error> {
    portfolio
        .validate_integrity()
        .map_err(|_| Error::Inconsistent)?;

    let mut values: BTreeMap<CategoryId, Decimal> = BTreeMap::new();
    let mut rows: Vec<(AccountId, ProductId, Option<String>, Decimal)> = Vec::new();
    for holding in &portfolio.holdings {
        let product = portfolio
            .products
            .iter()
            .find(|p| p.id == holding.product)
            .ok_or(Error::Inconsistent)?;
        let value = to_jpy(portfolio, &product.currency, holding.value)?;
        *values.entry(product.category).or_default() += value;
        rows.push((
            holding.account,
            product.id,
            holding.account_type.clone(),
            value,
        ));
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
    let groups = group_overviews(portfolio, &values, &share_of_total);
    let accounts = portfolio
        .accounts
        .iter()
        .map(|account| {
            let value: Decimal = rows
                .iter()
                .filter(|(id, _, _, _)| *id == account.id)
                .map(|(_, _, _, v)| *v)
                .sum();
            AccountOverview {
                account: account.id,
                value,
                share: share_of_total(value),
            }
        })
        .collect();
    let mut by_type: BTreeMap<Option<String>, Decimal> = BTreeMap::new();
    for (_, _, account_type, value) in &rows {
        *by_type.entry(account_type.clone()).or_default() += *value;
    }
    let types = by_type
        .into_iter()
        .map(|(account_type, value)| TypeOverview {
            account_type,
            value,
            share: share_of_total(value),
        })
        .collect();
    let holdings = rows
        .into_iter()
        .map(|(account, product, account_type, value)| HoldingOverview {
            account,
            product,
            account_type,
            value,
            share: share_of_total(value),
        })
        .collect();
    Ok(Overview {
        total,
        groups,
        categories,
        holdings,
        accounts,
        types,
    })
}

/// Adds up the categories of each group. The sums are exact; only the share is rounded, from the
/// summed value, so the groups' shares do not drift from the categories'.
fn group_overviews(
    portfolio: &Portfolio,
    values: &BTreeMap<CategoryId, Decimal>,
    share_of_total: &impl Fn(Decimal) -> Decimal,
) -> Vec<GroupOverview> {
    if portfolio.groups.is_empty() {
        return Vec::new();
    }
    let entry = |group: Option<GroupId>| {
        let mut current = Decimal::ZERO;
        let mut target_share = Decimal::ZERO;
        let mut any = false;
        for category in portfolio.categories.iter().filter(|c| c.group == group) {
            any = true;
            current += values.get(&category.id).copied().unwrap_or_default();
            target_share += category.target_weight;
        }
        let current_share = share_of_total(current);
        (
            any,
            GroupOverview {
                group,
                current,
                current_share,
                target_share,
                difference: current_share - target_share,
            },
        )
    };
    portfolio
        .groups
        .iter()
        .map(|g| Some(g.id))
        .chain(std::iter::once(None))
        .map(entry)
        // A group without categories is still listed (it is the user's); only an empty
        // "Ungrouped" entry is left out.
        .filter(|(any, overview)| *any || overview.group.is_some())
        .map(|(_, overview)| overview)
        .collect()
}
