//! Splits a sale of a product in an account over the record types that hold it.
//!
//! The allocation decides how much to sell per (account, product); this last step decides which
//! records give it: the types that may be sold, in the order of `Portfolio::sell_rank` (NISA
//! family last), each one until it is exhausted.

use std::collections::BTreeMap;

use portfolio::{AccountId, Portfolio, ProductId};
use rust_decimal::Decimal;
use serde::Serialize;

use crate::ProductLine;
use crate::engine::to_jpy;
use crate::round::floor_to_unit;

/// What an account holds of one product in the records of one type, in JPY.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TypeValue {
    /// The type id; `None` for the records without a type.
    pub account_type: Option<String>,
    #[serde(serialize_with = "crate::display::yen")]
    pub current: Decimal,
}

/// The value per (account, product) and record type, in sale order (see the module comment).
/// Records of the same type add up.
pub(crate) fn type_values(
    portfolio: &Portfolio,
) -> BTreeMap<(AccountId, ProductId), Vec<TypeValue>> {
    let mut out: BTreeMap<(AccountId, ProductId), Vec<TypeValue>> = BTreeMap::new();
    for holding in &portfolio.holdings {
        let Some(product) = portfolio.products.iter().find(|p| p.id == holding.product) else {
            continue;
        };
        let Ok(value) = to_jpy(portfolio, &product.currency, holding.value) else {
            continue;
        };
        let groups = out.entry((holding.account, holding.product)).or_default();
        match groups
            .iter_mut()
            .find(|g| g.account_type == holding.account_type)
        {
            Some(group) => group.current += value,
            None => groups.push(TypeValue {
                account_type: holding.account_type.clone(),
                current: value,
            }),
        }
    }
    for groups in out.values_mut() {
        groups.sort_by_key(|g| portfolio.sell_rank(g.account_type.as_deref()));
    }
    out
}

/// Takes `amount` from the groups in order: first whole rounding units of each group, then, when a
/// remainder is left (the units cannot be made from one group alone), what each still holds. The
/// parts add up to `amount`; no group gives more than it holds.
pub(crate) fn split_sale(groups: &[&TypeValue], amount: Decimal, unit: Decimal) -> Vec<Decimal> {
    let mut taken = vec![Decimal::ZERO; groups.len()];
    let mut remaining = amount;
    for (i, group) in groups.iter().enumerate() {
        let take = remaining.min(floor_to_unit(group.current, unit));
        taken[i] = take;
        remaining -= take;
    }
    for (i, group) in groups.iter().enumerate() {
        let take = remaining.min(group.current - taken[i]);
        taken[i] += take;
        remaining -= take;
    }
    if remaining > Decimal::ZERO && !taken.is_empty() {
        // Not reachable for amounts the allocation capped; keep the total exact regardless.
        let last = taken.len() - 1;
        taken[last] += remaining;
    }
    taken
}

/// Replaces each sale line (negative `delta`) with one line per record type it takes from. A buy
/// stays one line. `after` of a sale line is the value of its type group once the sale is made.
pub(crate) fn split_sales(portfolio: &Portfolio, lines: Vec<ProductLine>) -> Vec<ProductLine> {
    let values = type_values(portfolio);
    let mut out = Vec::new();
    for line in lines {
        if line.delta >= Decimal::ZERO {
            out.push(line);
            continue;
        }
        let unit = portfolio
            .accounts
            .iter()
            .find(|a| a.id == line.account)
            .map_or(Decimal::ONE, |a| a.rounding_unit);
        let groups: Vec<&TypeValue> = values
            .get(&(line.account, line.product))
            .map(|all| {
                all.iter()
                    .filter(|g| portfolio.type_sellable(g.account_type.as_deref()))
                    .collect()
            })
            .unwrap_or_default();
        if groups.is_empty() {
            out.push(line);
            continue;
        }
        let taken = split_sale(&groups, -line.delta, unit);
        for (group, take) in groups.iter().zip(taken) {
            if take.is_zero() {
                continue;
            }
            out.push(ProductLine {
                account: line.account,
                product: line.product,
                account_type: group.account_type.clone(),
                by_type: true,
                delta: -take,
                after: group.current - take,
            });
        }
    }
    out
}
