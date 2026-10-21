//! Suggestions and the balance table for an account whose strategy is "specified products with
//! ratios" (`portfolio::Strategy::Products`).
//!
//! Such an account aims at its own list of products and ratios. The category targets are not
//! used, and a product outside the list is neither bought nor sold. Within the list the same
//! water-filling as everywhere else spreads the money: the product furthest below its share of the
//! account's strategy products is raised first, and a sale starts with the one furthest above.

use std::collections::BTreeMap;

use portfolio::{Account, Portfolio, ProductId};
use rust_decimal::Decimal;
use serde::Serialize;

use crate::Error;
use crate::account_balance::ProductBalance;
use crate::engine::{MAX_AMOUNT, MAX_WEIGHT, to_jpy};
use crate::fill::{Item, fill_up};
use crate::round::{ceil_to_unit, floor_to_unit, round_to_units};
use crate::{Mode, ModeName, Note, ProductLine, Suggestion};

/// A requirement below this is rounding noise, not a real need.
const NOISE: Decimal = Decimal::from_parts(1, 0, 0, false, 9);

/// The strategy products of an account with what it holds of them.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StrategyBalance {
    /// What the account holds of the strategy's products, in JPY.
    #[serde(serialize_with = "crate::display::yen")]
    pub total: Decimal,
    /// One line per product of the strategy, in the order of the strategy.
    pub products: Vec<ProductBalance>,
}

struct Row {
    product: ProductId,
    /// The share of the strategy's total this product aims at (the shares add up to 1).
    share: Decimal,
    held: Decimal,
    /// What may be sold, in whole rounding units (0 when the account or the product cannot sell).
    sell_cap: Decimal,
    buyable: bool,
}

struct Plan<'a> {
    portfolio: &'a Portfolio,
    account: &'a Account,
    rows: Vec<Row>,
    /// What the account holds of the strategy's products.
    total: Decimal,
    /// The value of everything in every account.
    portfolio_total: Decimal,
}

pub(crate) fn account_of(portfolio: &Portfolio, id: portfolio::AccountId) -> Option<&Account> {
    portfolio
        .accounts
        .iter()
        .find(|a| a.id == id && !a.strategy.is_portfolio())
}

impl<'a> Plan<'a> {
    fn new(portfolio: &'a Portfolio, account: &'a Account) -> Result<Self, Error> {
        let items = account.strategy.items();
        if items.is_empty() {
            return Err(Error::Inconsistent);
        }
        if items.iter().any(|i| i.weight() > MAX_WEIGHT) {
            return Err(Error::TooLarge);
        }
        let mut held: BTreeMap<ProductId, Decimal> = BTreeMap::new();
        let mut sellable: BTreeMap<ProductId, Decimal> = BTreeMap::new();
        let mut portfolio_total = Decimal::ZERO;
        for holding in &portfolio.holdings {
            let product = portfolio
                .products
                .iter()
                .find(|p| p.id == holding.product)
                .ok_or(Error::Inconsistent)?;
            let value = to_jpy(portfolio, &product.currency, holding.value)?;
            portfolio_total += value;
            if holding.account == account.id {
                *held.entry(product.id).or_default() += value;
                if portfolio.type_sellable(holding.account_type.as_deref()) {
                    *sellable.entry(product.id).or_default() += value;
                }
            }
        }
        if portfolio_total > MAX_AMOUNT {
            return Err(Error::TooLarge);
        }
        let weight_sum: Decimal = items.iter().map(|i| i.weight()).sum();
        let rows: Vec<Row> = items
            .iter()
            .map(|item| {
                let product = item.product;
                let excluded = account.excluded.contains(&product);
                let sell_cap = if account.sellable && !excluded {
                    floor_to_unit(
                        sellable.get(&product).copied().unwrap_or_default(),
                        account.rounding_unit,
                    )
                } else {
                    Decimal::ZERO
                };
                Row {
                    product,
                    share: item.weight() / weight_sum,
                    held: held.get(&product).copied().unwrap_or_default(),
                    sell_cap,
                    buyable: !excluded,
                }
            })
            .collect();
        let total = rows.iter().map(|r| r.held).sum();
        Ok(Self {
            portfolio,
            account,
            rows,
            total,
            portfolio_total,
        })
    }

    fn unit(&self) -> Decimal {
        self.account.rounding_unit
    }

    fn buy_items(&self, caps: &[Option<Decimal>]) -> (Vec<usize>, Vec<Item>) {
        let index: Vec<usize> = (0..self.rows.len())
            .filter(|&i| self.rows[i].buyable)
            .collect();
        let items = index
            .iter()
            .map(|&i| Item {
                weight: self.rows[i].share,
                current: self.rows[i].held,
                cap: caps[i],
            })
            .collect();
        (index, items)
    }

    fn sell_items(&self, caps: &[Option<Decimal>]) -> (Vec<usize>, Vec<Item>) {
        let index: Vec<usize> = (0..self.rows.len())
            .filter(|&i| {
                self.rows[i].sell_cap > Decimal::ZERO && caps[i].is_none_or(|c| c > Decimal::ZERO)
            })
            .collect();
        let items = index
            .iter()
            .map(|&i| Item {
                weight: self.rows[i].share,
                current: -self.rows[i].held,
                cap: Some(caps[i].map_or(self.rows[i].sell_cap, |c| c.min(self.rows[i].sell_cap))),
            })
            .collect();
        (index, items)
    }

    /// Spreads `amount` (a multiple of the unit) over the products to buy, rounded to the unit.
    fn buy_lines(
        &self,
        amount: Decimal,
        caps: &[Option<Decimal>],
    ) -> Result<Vec<ProductLine>, Error> {
        let (index, items) = self.buy_items(caps);
        let shares = fill_up(&items, amount).map_err(|_| Error::NothingToBuy)?;
        let caps: Vec<Option<Decimal>> = index.iter().map(|&i| caps[i]).collect();
        let rounded =
            round_to_units(&shares, self.unit(), amount, &caps).ok_or(Error::NothingToBuy)?;
        Ok(self.lines(&index, rounded, Decimal::ONE))
    }

    /// Spreads a sale of `amount` (a multiple of the unit) over the products held above their share.
    fn sell_lines(
        &self,
        amount: Decimal,
        caps: &[Option<Decimal>],
    ) -> Result<Vec<ProductLine>, Error> {
        let (index, items) = self.sell_items(caps);
        let shares = fill_up(&items, amount).map_err(|_| Error::NotEnoughToSell)?;
        let caps: Vec<Option<Decimal>> = items.iter().map(|i| i.cap).collect();
        let rounded =
            round_to_units(&shares, self.unit(), amount, &caps).ok_or(Error::NotEnoughToSell)?;
        Ok(self.lines(&index, rounded, -Decimal::ONE))
    }

    fn lines(&self, index: &[usize], amounts: Vec<Decimal>, sign: Decimal) -> Vec<ProductLine> {
        index
            .iter()
            .zip(amounts)
            .map(|(&i, amount)| ProductLine {
                account: self.account.id,
                product: self.rows[i].product,
                account_type: None,
                by_type: false,
                delta: amount * sign,
                after: self.rows[i].held + amount * sign,
            })
            .collect()
    }

    fn finish(
        &self,
        mode: ModeName,
        requested: Option<Decimal>,
        applied: Decimal,
        required_total: Option<Decimal>,
        mut lines: Vec<ProductLine>,
        notes: Vec<Note>,
    ) -> Suggestion {
        lines.retain(|line| !line.delta.is_zero());
        let mut lines = crate::split::split_sales(self.portfolio, lines);
        lines.sort_by_key(|line| line.product);
        let total_after = self.portfolio_total + lines.iter().map(|l| l.delta).sum::<Decimal>();
        Suggestion {
            mode,
            account: Some(self.account.id),
            requested,
            applied,
            required_total,
            total_after,
            categories: Vec::new(),
            products: lines,
            notes,
        }
    }

    fn fixed_add(
        &self,
        amount: Decimal,
        mode: ModeName,
        requested: Option<Decimal>,
        required_total: Option<Decimal>,
    ) -> Result<Suggestion, Error> {
        let spend = floor_to_unit(amount, self.unit());
        if spend <= Decimal::ZERO {
            return Err(Error::AmountBelowUnit);
        }
        let lines = self.buy_lines(spend, &vec![None; self.rows.len()])?;
        let mut notes = Vec::new();
        if mode == ModeName::FixedAdd && spend != amount {
            notes.push(Note::AmountRounded {
                requested: amount,
                applied: spend,
            });
        }
        Ok(self.finish(mode, requested, spend, required_total, lines, notes))
    }

    fn buy_only_to_target(&self) -> Result<Suggestion, Error> {
        let level = self
            .rows
            .iter()
            .map(|r| r.held / r.share)
            .fold(self.total, Decimal::max);
        let required = level - self.total;
        if required <= NOISE {
            return Ok(self.finish(
                ModeName::BuyOnlyToTarget,
                None,
                Decimal::ZERO,
                Some(Decimal::ZERO),
                Vec::new(),
                vec![Note::AlreadyBalanced],
            ));
        }
        if required > MAX_AMOUNT {
            return Err(Error::TooLarge);
        }
        let spend = ceil_to_unit(required, self.unit());
        let mut suggestion =
            self.fixed_add(spend, ModeName::BuyOnlyToTarget, None, Some(required))?;
        for row in &self.rows {
            let need = row.share * level - row.held;
            let bought = suggestion
                .products
                .iter()
                .find(|l| l.product == row.product)
                .map_or(Decimal::ZERO, |l| l.delta);
            if need - bought >= self.unit() {
                suggestion.notes.push(Note::ProductNotFilled {
                    product: row.product,
                    shortfall: need - bought,
                });
            }
        }
        Ok(suggestion)
    }

    fn fixed_sell(&self, amount: Decimal) -> Result<Suggestion, Error> {
        if !self.account.sellable {
            return Err(Error::NotSellable);
        }
        let sell = floor_to_unit(amount, self.unit());
        if sell <= Decimal::ZERO {
            return Err(Error::AmountBelowUnit);
        }
        let capacity: Decimal = self.rows.iter().map(|r| r.sell_cap).sum();
        if sell > capacity {
            return Err(Error::NotEnoughToSell);
        }
        let lines = self.sell_lines(sell, &vec![None; self.rows.len()])?;
        let mut notes = Vec::new();
        if sell != amount {
            notes.push(Note::AmountRounded {
                requested: amount,
                applied: sell,
            });
        }
        Ok(self.finish(ModeName::FixedSell, Some(amount), sell, None, lines, notes))
    }

    fn full_rebalance(&self) -> Result<Suggestion, Error> {
        let tolerance = self.portfolio.settings.product_tolerance;
        // How far each product is from its share of the strategy's total, ignoring small deviations.
        let wanted: Vec<Decimal> = self
            .rows
            .iter()
            .map(|r| {
                if self.total <= Decimal::ZERO || (r.held / self.total - r.share).abs() <= tolerance
                {
                    Decimal::ZERO
                } else {
                    r.share * self.total - r.held
                }
            })
            .collect();
        let buy_caps: Vec<Option<Decimal>> = self
            .rows
            .iter()
            .zip(&wanted)
            .map(|(r, w)| {
                Some(if r.buyable && *w > Decimal::ZERO {
                    *w
                } else {
                    Decimal::ZERO
                })
            })
            .collect();
        let sell_caps: Vec<Option<Decimal>> = wanted
            .iter()
            .map(|w| {
                Some(if *w < Decimal::ZERO {
                    -*w
                } else {
                    Decimal::ZERO
                })
            })
            .collect();
        let buy_capacity: Decimal = buy_caps.iter().flatten().sum();
        let sell_capacity: Decimal = self
            .rows
            .iter()
            .zip(&sell_caps)
            .map(|(r, c)| r.sell_cap.min(c.unwrap_or_default()))
            .sum();
        let trade = floor_to_unit(buy_capacity.min(sell_capacity), self.unit());

        let mut lines = Vec::new();
        if trade > Decimal::ZERO {
            lines.extend(self.buy_lines(trade, &buy_caps)?);
            lines.extend(self.sell_lines(trade, &sell_caps)?);
        }
        let mut notes = Vec::new();
        for (row, wanted) in self.rows.iter().zip(&wanted) {
            let done: Decimal = lines
                .iter()
                .filter(|l| l.product == row.product)
                .map(|l| l.delta)
                .sum();
            if (*wanted - done).abs() >= self.unit() {
                notes.push(Note::ProductNotFilled {
                    product: row.product,
                    shortfall: *wanted - done,
                });
            }
        }
        if lines.is_empty() && notes.is_empty() {
            notes.push(Note::AlreadyBalanced);
        }
        let applied = lines
            .iter()
            .filter(|l| l.delta > Decimal::ZERO)
            .map(|l| l.delta)
            .sum();
        Ok(self.finish(ModeName::FullRebalance, None, applied, None, lines, notes))
    }
}

fn check_amount(amount: Decimal) -> Result<(), Error> {
    if amount <= Decimal::ZERO {
        Err(Error::InvalidAmount)
    } else if amount > MAX_AMOUNT {
        Err(Error::TooLarge)
    } else {
        Ok(())
    }
}

pub(crate) fn suggest(
    portfolio: &Portfolio,
    account: &Account,
    mode: &Mode,
) -> Result<Suggestion, Error> {
    let plan = Plan::new(portfolio, account)?;
    match mode {
        Mode::FixedAdd { amount } => {
            check_amount(*amount)?;
            plan.fixed_add(*amount, ModeName::FixedAdd, Some(*amount), None)
        }
        Mode::BuyOnlyToTarget => plan.buy_only_to_target(),
        Mode::FullRebalance => plan.full_rebalance(),
        Mode::FixedSell { amount } => {
            check_amount(*amount)?;
            plan.fixed_sell(*amount)
        }
    }
}

/// The strategy's products with what the account holds of each, and their total.
pub(crate) fn balance(portfolio: &Portfolio, account: &Account) -> Result<StrategyBalance, Error> {
    let plan = Plan::new(portfolio, account)?;
    let mut types = crate::split::type_values(portfolio);
    let products = plan
        .rows
        .iter()
        .map(|row| ProductBalance {
            product: row.product,
            current: row.held,
            types: types.remove(&(account.id, row.product)).unwrap_or_default(),
        })
        .collect();
    Ok(StrategyBalance {
        total: plan.total,
        products,
    })
}
