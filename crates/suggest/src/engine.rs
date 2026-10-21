use std::collections::{BTreeMap, BTreeSet};

use portfolio::{
    Account, AccountId, Category, CategoryId, Portfolio, Product, ProductId, SuggestionMode,
};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

use crate::fill::{Item, fill_up};
use crate::round::{ceil_to_unit, floor_to_unit, round_to_units};
use crate::{CategoryLine, Error, Method, Mode, ModeName, Note, ProductLine, Request, Suggestion};

/// Amounts above this (one quadrillion yen) are refused, so none of the
/// arithmetic below can overflow a `Decimal`.
pub(crate) const MAX_AMOUNT: Decimal = Decimal::from_parts(2_764_472_320, 232_830, 0, false, 0);
/// Product weights are relative, so a huge one is a typing mistake.
pub(crate) const MAX_WEIGHT: Decimal = Decimal::from_parts(1_000_000_000, 0, 0, false, 0);
/// A requirement below this is rounding noise, not a real need.
const NOISE: Decimal = Decimal::from_parts(1, 0, 0, false, 9);

/// Computes a suggestion for one account, or across every account when the request names none.
pub fn suggest(portfolio: &Portfolio, request: &Request) -> Result<Suggestion, Error> {
    portfolio
        .validate_integrity()
        .map_err(|_| Error::Inconsistent)?;
    // An account with a product strategy does not use the category targets, so problems with them
    // do not block it.
    if let Some(id) = request.account
        && let Some(account) = crate::strategy::account_of(portfolio, id)
    {
        return crate::strategy::suggest(portfolio, account, &request.mode);
    }
    // The category targets are not used when allocating by product weights, so problems with them
    // (weights that do not add up) do not block it.
    if request.method == Method::FollowPortfolio {
        let issues = portfolio.issues();
        if !issues.is_empty() {
            return Err(Error::Blocked(issues));
        }
    }
    let book = Book::new(portfolio, request)?;
    if request.method == Method::ByProductWeights {
        return book.by_product_weights(&request.mode);
    }
    match &request.mode {
        Mode::FixedAdd { amount } => {
            check_amount(*amount)?;
            book.fixed_add(*amount, ModeName::FixedAdd, Some(*amount), None)
        }
        Mode::BuyOnlyToTarget => book.buy_only_to_target(),
        Mode::FullRebalance => book.full_rebalance(),
        Mode::FixedSell { amount } => {
            check_amount(*amount)?;
            book.fixed_sell(*amount)
        }
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

/// An (account, product) pair: the unit a suggestion trades in.
type Slot = (AccountId, ProductId);

/// The portfolio valued in JPY, seen from the accounts a suggestion trades in.
struct Book<'a> {
    portfolio: &'a Portfolio,
    /// The accounts the suggestion trades in: the requested one, or all of them.
    scope: Vec<&'a Account>,
    request_account: Option<AccountId>,
    /// Whether the money is allocated across all products by their weights (no categories).
    by_product: bool,
    /// The mode being run: the product settings of this mode apply.
    mode: SuggestionMode,
    /// The step amounts are a multiple of: the rounding unit of the accounts in scope (the least
    /// common multiple when they differ, so every account's own unit divides it).
    unit: Decimal,
    /// What each account holds per product in records it may sell.
    account_value: BTreeMap<Slot, Decimal>,
    /// What each account holds per product in any record (selling allowed or not).
    held_here: BTreeMap<Slot, Decimal>,
    /// Pairs where the account holds the product, when only those may be bought; `None` when every
    /// product that takes part may be bought.
    held_only: Option<BTreeSet<Slot>>,
    /// Holdings per product the leveling goes by: across all accounts for All, the requested
    /// account's own on its tab.
    product_value: BTreeMap<ProductId, Decimal>,
    category_value: BTreeMap<CategoryId, Decimal>,
    total: Decimal,
}

impl<'a> Book<'a> {
    fn new(portfolio: &'a Portfolio, request: &Request) -> Result<Self, Error> {
        let scope: Vec<&Account> = match request.account {
            Some(id) => vec![
                portfolio
                    .accounts
                    .iter()
                    .find(|a| a.id == id)
                    .ok_or(Error::UnknownAccount)?,
            ],
            // An account with a product strategy takes no part in the shared suggestion: its
            // strategy applies in its own tab. Its holdings still count toward the values.
            None => portfolio
                .accounts
                .iter()
                .filter(|a| a.strategy.is_portfolio())
                .collect(),
        };

        let mut account_value = BTreeMap::new();
        let mut held_here: BTreeMap<Slot, Decimal> = BTreeMap::new();
        let mut product_value: BTreeMap<ProductId, Decimal> = BTreeMap::new();
        for holding in &portfolio.holdings {
            let product = portfolio
                .products
                .iter()
                .find(|p| p.id == holding.product)
                .ok_or(Error::Inconsistent)?;
            let value = to_jpy(portfolio, &product.currency, holding.value)?;
            *product_value.entry(product.id).or_default() += value;
            *held_here.entry((holding.account, product.id)).or_default() += value;
            // Only what a suggestion may sell counts as held here (the type of a record can forbid
            // selling it, like a NISA record).
            if portfolio.type_sellable(holding.account_type.as_deref()) {
                *account_value
                    .entry((holding.account, product.id))
                    .or_default() += value;
            }
        }

        let portfolio_total: Decimal = product_value.values().copied().sum();
        if portfolio_total > MAX_AMOUNT {
            return Err(Error::TooLarge);
        }
        if portfolio_total <= Decimal::ZERO || portfolio.categories.is_empty() {
            return Err(Error::EmptyPortfolio);
        }
        // An account's tab looks only at that account's holdings (all record types, like its
        // "Current" column); the All tab considers every account's.
        if let Some(id) = request.account {
            product_value = held_here
                .iter()
                .filter(|((account, _), _)| *account == id)
                .map(|((_, product), value)| (*product, *value))
                .collect();
        }
        let mut category_value: BTreeMap<CategoryId, Decimal> = BTreeMap::new();
        for product in &portfolio.products {
            let value = product_value.get(&product.id).copied().unwrap_or_default();
            *category_value.entry(product.category).or_default() += value;
        }
        let total: Decimal = category_value.values().copied().sum();
        let held_only = request.held_only.then(|| {
            held_here
                .iter()
                .filter(|(_, value)| **value > Decimal::ZERO)
                .map(|(slot, _)| *slot)
                .collect()
        });
        let unit = common_unit(scope.iter().map(|a| a.rounding_unit))?;
        Ok(Self {
            portfolio,
            scope,
            request_account: request.account,
            by_product: request.method == Method::ByProductWeights,
            mode: match request.mode {
                Mode::FixedAdd { .. } => SuggestionMode::FixedAdd,
                Mode::BuyOnlyToTarget => SuggestionMode::BuyOnlyToTarget,
                Mode::FullRebalance => SuggestionMode::FullRebalance,
                Mode::FixedSell { .. } => SuggestionMode::FixedSell,
            },
            unit,
            account_value,
            held_here,
            held_only,
            product_value,
            category_value,
            total,
        })
    }

    fn unit(&self) -> Decimal {
        self.unit
    }

    fn value_of(&self, category: &Category) -> Decimal {
        self.category_value
            .get(&category.id)
            .copied()
            .unwrap_or_default()
    }

    /// The weight of a product on the tab being run: the account's own, or the All tab's.
    fn weight(&self, product: ProductId) -> Decimal {
        self.portfolio
            .product_weight(self.mode, product, self.request_account)
    }

    /// What the product holds for the leveling: on an account's tab that account's holding in
    /// records of any type (the "Current" column), on the All tab the holdings of all accounts.
    fn global_value(&self, product: &Product) -> Decimal {
        self.product_value
            .get(&product.id)
            .copied()
            .unwrap_or_default()
    }

    fn category(&self, id: CategoryId) -> Option<&'a Category> {
        self.portfolio.categories.iter().find(|c| c.id == id)
    }

    fn any_sellable(&self) -> bool {
        self.scope.iter().any(|a| a.sellable)
    }

    /// Whether the account leaves the product out of this suggestion: by the account's own list
    /// when it trades alone, by the list for All when every account trades.
    fn is_excluded(&self, account: &Account, product: ProductId) -> bool {
        if self.request_account.is_some() {
            account.excluded.contains(&product)
        } else {
            account.excluded_all.contains(&product)
        }
    }

    /// Whether the account may buy the product.
    fn can_buy(&self, account: &Account, product: ProductId) -> bool {
        !self.is_excluded(account, product)
            && self
                .held_only
                .as_ref()
                .is_none_or(|held| held.contains(&(account.id, product)))
    }

    /// The accounts in scope that may buy the product.
    fn buyers(&self, product: ProductId) -> Vec<&'a Account> {
        self.scope
            .iter()
            .copied()
            .filter(|a| self.can_buy(a, product))
            .collect()
    }

    /// The accounts in scope that may sell the product, with the value each can give.
    fn sellers(&self, product: ProductId) -> Vec<(&'a Account, Decimal)> {
        self.scope
            .iter()
            .copied()
            .filter(|a| a.sellable && !self.is_excluded(a, product))
            .filter_map(|a| {
                let value = self
                    .account_value
                    .get(&(a.id, product))
                    .copied()
                    .unwrap_or_default();
                (value > Decimal::ZERO).then_some((a, value))
            })
            .collect()
    }

    /// Products of the category some account in scope is allowed to buy.
    fn candidates(&self, category: CategoryId) -> Vec<&'a Product> {
        self.portfolio
            .products
            .iter()
            .filter(|p| p.category == category)
            .filter(|p| !self.buyers(p.id).is_empty())
            .collect()
    }

    /// Products of the category the accounts in scope can sell, with the value they hold.
    fn held(&self, category: CategoryId) -> Vec<(&'a Product, Decimal)> {
        self.portfolio
            .products
            .iter()
            .filter(|p| p.category == category)
            .filter_map(|p| {
                let value: Decimal = self.sellers(p.id).iter().map(|(_, v)| *v).sum();
                (value > Decimal::ZERO).then_some((p, value))
            })
            .collect()
    }

    /// What can be sold from the category in whole rounding units: each (account, product) pair
    /// only gives multiples of its account's unit, and an account's total only a multiple of the
    /// common step, so the remainders below them cannot be sold.
    fn sellable_total(&self, category: CategoryId) -> Decimal {
        self.sellable_total_of(|p| p.category == category)
    }

    /// The same for the products the filter picks.
    fn sellable_total_of(&self, pick: impl Fn(&Product) -> bool) -> Decimal {
        let mut per_account: BTreeMap<AccountId, Decimal> = BTreeMap::new();
        for product in self.portfolio.products.iter().filter(|p| pick(p)) {
            for (account, value) in self.sellers(product.id) {
                *per_account.entry(account.id).or_default() +=
                    floor_to_unit(value, account.rounding_unit);
            }
        }
        per_account
            .values()
            .map(|sum| floor_to_unit(*sum, self.unit))
            .sum()
    }

    /// Spreads each product's buy amount over the accounts that may buy it: the account holding
    /// the least of it first (equal weights), like the products inside a category.
    fn place_buy(&self, flat: Vec<(ProductId, Decimal)>) -> Vec<(Slot, Decimal)> {
        let mut out = Vec::new();
        for (product, share) in flat {
            let buyers = self.buyers(product);
            let items: Vec<Item> = buyers
                .iter()
                .map(|a| Item {
                    weight: Decimal::ONE,
                    current: self
                        .held_here
                        .get(&(a.id, product))
                        .copied()
                        .unwrap_or_default(),
                    cap: None,
                })
                .collect();
            let shares =
                fill_up(&items, share).unwrap_or_else(|_| vec![Decimal::ZERO; items.len()]);
            out.extend(buyers.iter().map(|a| (a.id, product)).zip(shares));
        }
        out
    }

    /// Spreads each product's sell amount over the accounts that hold it, in proportion to what
    /// each can give.
    fn place_sell(&self, flat: Vec<(ProductId, Decimal)>) -> Vec<(Slot, Decimal)> {
        let mut out = Vec::new();
        for (product, share) in flat {
            let sellers = self.sellers(product);
            let caps: Vec<Decimal> = sellers
                .iter()
                .map(|(a, v)| floor_to_unit(*v, a.rounding_unit))
                .collect();
            let shares = proportional(&caps, share);
            out.extend(sellers.iter().map(|(a, _)| (a.id, product)).zip(shares));
        }
        out
    }

    /// Rounds the amounts to each account's unit so they add up to exactly `total` (a multiple of
    /// the common unit). The per-account totals are rounded first, to the common unit, then the
    /// amounts inside each account to its own unit. A sale never takes more of a pair than is held.
    fn round_slots(
        &self,
        flat: &[(Slot, Decimal)],
        total: Decimal,
        selling: bool,
    ) -> Option<Vec<Decimal>> {
        let mut groups: BTreeMap<AccountId, Vec<usize>> = BTreeMap::new();
        for (index, ((account, _), _)) in flat.iter().enumerate() {
            groups.entry(*account).or_default().push(index);
        }
        let unit_of = |id: AccountId| {
            self.scope
                .iter()
                .find(|a| a.id == id)
                .map_or(self.unit, |a| a.rounding_unit)
        };
        let cap_of = |index: usize| {
            selling
                .then(|| self.account_value.get(&flat[index].0))
                .flatten()
                .copied()
        };
        let sums: Vec<Decimal> = groups
            .values()
            .map(|ids| ids.iter().map(|&i| flat[i].1).sum())
            .collect();
        let group_caps: Vec<Option<Decimal>> = groups
            .iter()
            .map(|(account, ids)| {
                selling.then(|| {
                    let sum: Decimal = ids
                        .iter()
                        .map(|&i| floor_to_unit(cap_of(i).unwrap_or_default(), unit_of(*account)))
                        .sum();
                    floor_to_unit(sum, self.unit)
                })
            })
            .collect();
        let totals = round_to_units(&sums, self.unit, total, &group_caps)?;
        let mut out = vec![Decimal::ZERO; flat.len()];
        for ((account, ids), group_total) in groups.iter().zip(totals) {
            // The account's share was rounded to the common unit; bring its amounts to that sum
            // before rounding them to the account's own unit.
            let sum: Decimal = ids.iter().map(|&i| flat[i].1).sum();
            let values: Vec<Decimal> = ids
                .iter()
                .map(|&i| {
                    if sum > Decimal::ZERO {
                        flat[i].1 * group_total / sum
                    } else {
                        flat[i].1
                    }
                })
                .collect();
            let caps: Vec<Option<Decimal>> = ids.iter().map(|&i| cap_of(i)).collect();
            let rounded = round_to_units(&values, unit_of(*account), group_total, &caps)?;
            for (&i, amount) in ids.iter().zip(rounded) {
                out[i] = amount;
            }
        }
        Some(out)
    }

    // ---- splitting a category's amount over its products -------------------

    fn split_buy(
        &self,
        category: &Category,
        amount: Decimal,
    ) -> Result<Vec<(ProductId, Decimal)>, Error> {
        self.spread_buy(self.candidates(category.id), amount)
    }

    /// Spreads a purchase over the products by their weights, counting what is held across all
    /// accounts: the product furthest below its share is raised first.
    fn spread_buy(
        &self,
        candidates: Vec<&'a Product>,
        amount: Decimal,
    ) -> Result<Vec<(ProductId, Decimal)>, Error> {
        let items: Vec<Item> = candidates
            .iter()
            .map(|p| Item {
                weight: self.weight(p.id),
                current: self.global_value(p),
                cap: None,
            })
            .collect();
        let shares = fill_up(&items, amount).map_err(|_| Error::NothingToBuy)?;
        Ok(candidates.iter().map(|p| p.id).zip(shares).collect())
    }

    fn split_sell(&self, category: &Category, amount: Decimal) -> Vec<(ProductId, Decimal)> {
        self.spread_sell(self.held(category.id), amount)
    }

    /// Spreads a sale over the products the accounts can sell, the one furthest above its share
    /// first.
    fn spread_sell(
        &self,
        held: Vec<(&'a Product, Decimal)>,
        amount: Decimal,
    ) -> Vec<(ProductId, Decimal)> {
        let items: Vec<Item> = held
            .iter()
            .map(|(p, here)| Item {
                weight: self.weight(p.id),
                current: -self.global_value(p),
                cap: Some(*here),
            })
            .collect();
        let shares = fill_up(&items, amount).unwrap_or_else(|_| {
            // No weights to go by (or rounding noise above the capacity): sell
            // in proportion to what is held here.
            let caps: Vec<Decimal> = held.iter().map(|(_, here)| *here).collect();
            proportional(&caps, amount)
        });
        held.iter().map(|(p, _)| p.id).zip(shares).collect()
    }

    // ---- category level -----------------------------------------------------

    /// Spreads `amount` over the categories that need money and that this
    /// account can buy into.
    fn fill_categories_buy(&self, amount: Decimal) -> Result<Vec<(CategoryId, Decimal)>, Error> {
        let categories: Vec<&Category> = self
            .portfolio
            .categories
            .iter()
            .filter(|c| !self.candidates(c.id).is_empty())
            .collect();
        let items: Vec<Item> = categories
            .iter()
            .map(|c| Item {
                weight: c.target_weight,
                current: self.value_of(c),
                cap: None,
            })
            .collect();
        let shares = fill_up(&items, amount).map_err(|_| Error::NothingToBuy)?;
        Ok(categories.iter().map(|c| c.id).zip(shares).collect())
    }

    /// Takes `amount` out of the given categories, most overweight first.
    /// `caps` limit how much each can give. A category with a target of zero is
    /// emptied before any other.
    fn fill_categories_sell(
        &self,
        caps: &[(CategoryId, Decimal)],
        amount: Decimal,
    ) -> Result<Vec<(CategoryId, Decimal)>, Error> {
        let weight_of = |id: CategoryId| {
            self.category(id)
                .map(|c| c.target_weight)
                .unwrap_or_default()
        };
        let zero_total: Decimal = caps
            .iter()
            .filter(|(id, _)| weight_of(*id).is_zero())
            .map(|(_, cap)| *cap)
            .sum();

        if zero_total >= amount && zero_total > Decimal::ZERO {
            let zero_caps: Vec<Decimal> = caps
                .iter()
                .map(|(id, cap)| {
                    if weight_of(*id).is_zero() {
                        *cap
                    } else {
                        Decimal::ZERO
                    }
                })
                .collect();
            let shares = proportional(&zero_caps, amount);
            return Ok(caps.iter().map(|(id, _)| *id).zip(shares).collect());
        }

        let positive: Vec<&(CategoryId, Decimal)> = caps
            .iter()
            .filter(|(id, _)| !weight_of(*id).is_zero())
            .collect();
        let items: Vec<Item> = positive
            .iter()
            .map(|(id, cap)| Item {
                weight: weight_of(*id),
                current: -self
                    .category(*id)
                    .map(|c| self.value_of(c))
                    .unwrap_or_default(),
                cap: Some(*cap),
            })
            .collect();
        let shares = fill_up(&items, amount - zero_total).map_err(|_| Error::NotEnoughToSell)?;
        let mut by_category: BTreeMap<CategoryId, Decimal> =
            positive.iter().map(|(id, _)| *id).zip(shares).collect();
        for (id, cap) in caps {
            if weight_of(*id).is_zero() {
                by_category.insert(*id, *cap);
            }
        }
        Ok(caps
            .iter()
            .map(|(id, _)| (*id, by_category.get(id).copied().unwrap_or_default()))
            .collect())
    }

    // ---- the four modes -----------------------------------------------------

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
        let mut flat = Vec::new();
        if self.by_product {
            flat = self.spread_buy(self.all_candidates(), spend)?;
        } else {
            for (id, share) in self.fill_categories_buy(spend)? {
                if share > Decimal::ZERO
                    && let Some(category) = self.category(id)
                {
                    flat.extend(self.split_buy(category, share)?);
                }
            }
        }
        let lines = self.buy_lines(flat, spend)?;

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
        let mut level = self.total;
        for category in &self.portfolio.categories {
            let value = self.value_of(category);
            if category.target_weight.is_zero() {
                if value > Decimal::ZERO {
                    return Err(Error::CannotReachWithoutSelling);
                }
            } else {
                level = level.max(value / category.target_weight);
            }
        }
        let required = level - self.total;
        if required <= NOISE {
            let notes = vec![Note::AlreadyBalanced];
            return Ok(self.finish(
                ModeName::BuyOnlyToTarget,
                None,
                Decimal::ZERO,
                Some(Decimal::ZERO),
                Vec::new(),
                notes,
            ));
        }
        if required > MAX_AMOUNT {
            return Err(Error::TooLarge);
        }
        let spend = ceil_to_unit(required, self.unit());
        let mut suggestion =
            self.fixed_add(spend, ModeName::BuyOnlyToTarget, None, Some(required))?;

        // Report the categories this account could not bring to target.
        for category in &self.portfolio.categories {
            let need = category.target_weight * level - self.value_of(category);
            let bought = suggestion
                .categories
                .iter()
                .find(|line| line.category == category.id)
                .map_or(Decimal::ZERO, |line| line.delta);
            if need - bought >= self.unit() {
                suggestion.notes.push(Note::CategoryNotFilled {
                    category: category.id,
                    shortfall: need - bought,
                });
            }
        }
        Ok(suggestion)
    }

    fn fixed_sell(&self, amount: Decimal) -> Result<Suggestion, Error> {
        if !self.any_sellable() {
            return Err(Error::NotSellable);
        }
        let sell = floor_to_unit(amount, self.unit());
        if sell <= Decimal::ZERO {
            return Err(Error::AmountBelowUnit);
        }
        let caps: Vec<(CategoryId, Decimal)> = self
            .portfolio
            .categories
            .iter()
            .map(|c| (c.id, self.sellable_total(c.id)))
            .filter(|(_, cap)| *cap > Decimal::ZERO)
            .collect();
        let capacity: Decimal = caps.iter().map(|(_, cap)| *cap).sum();
        if sell > capacity {
            return Err(Error::NotEnoughToSell);
        }

        let lines = self.sell_lines(&self.fill_categories_sell(&caps, sell)?, sell)?;
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
        let tolerance = self.portfolio.settings.category_tolerance;

        // How far each category is from its target, ignoring small deviations.
        let delta_of = |category: &Category| {
            let value = self.value_of(category);
            let off = if self.total.is_zero() {
                Decimal::ZERO
            } else {
                (value / self.total - category.target_weight).abs()
            };
            if off <= tolerance {
                Decimal::ZERO
            } else {
                category.target_weight * self.total - value
            }
        };
        let mut max_buy: Vec<(CategoryId, Decimal)> = Vec::new();
        let mut max_sell: Vec<(CategoryId, Decimal)> = Vec::new();
        for category in &self.portfolio.categories {
            let delta = delta_of(category);
            if delta > Decimal::ZERO && !self.candidates(category.id).is_empty() {
                max_buy.push((category.id, delta));
            }
            if delta < Decimal::ZERO && self.any_sellable() {
                let can_sell = (-delta).min(self.sellable_total(category.id));
                if can_sell > Decimal::ZERO {
                    max_sell.push((category.id, can_sell));
                }
            }
        }
        let buy_capacity: Decimal = max_buy.iter().map(|(_, v)| *v).sum();
        let sell_capacity: Decimal = max_sell.iter().map(|(_, v)| *v).sum();
        let trade = floor_to_unit(buy_capacity.min(sell_capacity), self.unit());

        let mut lines = Vec::new();
        if trade > Decimal::ZERO {
            let items: Vec<Item> = max_buy
                .iter()
                .filter_map(|(id, cap)| {
                    let category = self.category(*id)?;
                    Some(Item {
                        weight: category.target_weight,
                        current: self.value_of(category),
                        cap: Some(*cap),
                    })
                })
                .collect();
            let shares = fill_up(&items, trade).map_err(|_| Error::NothingToBuy)?;
            let mut flat = Vec::new();
            for ((id, _), share) in max_buy.iter().zip(shares) {
                if share > Decimal::ZERO
                    && let Some(category) = self.category(*id)
                {
                    flat.extend(self.split_buy(category, share)?);
                }
            }
            lines.extend(self.buy_lines(flat, trade)?);
            lines.extend(self.sell_lines(&self.fill_categories_sell(&max_sell, trade)?, trade)?);
        }

        // Anything that was not reached is reported rather than hidden.
        let mut notes = Vec::new();
        let net: BTreeMap<CategoryId, Decimal> = self.category_deltas(&lines);
        for category in &self.portfolio.categories {
            let wanted = delta_of(category);
            let done = net.get(&category.id).copied().unwrap_or_default();
            let missing = (wanted - done).abs();
            if missing >= self.unit() {
                notes.push(Note::CategoryNotFilled {
                    category: category.id,
                    shortfall: wanted - done,
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

    /// A trade of a product in an account, with the value the account holds of it afterwards.
    fn line(&self, account: AccountId, product: ProductId, delta: Decimal) -> ProductLine {
        let held = self
            .held_here
            .get(&(account, product))
            .copied()
            .unwrap_or_default();
        ProductLine {
            account,
            product,
            account_type: None,
            by_type: false,
            delta,
            after: held + delta,
        }
    }

    /// Turns per-product buy amounts into per-account purchases, rounded to the units.
    fn buy_lines(
        &self,
        flat: Vec<(ProductId, Decimal)>,
        total: Decimal,
    ) -> Result<Vec<ProductLine>, Error> {
        let placed = self.place_buy(flat);
        let rounded = self
            .round_slots(&placed, total, false)
            .ok_or(Error::NothingToBuy)?;
        Ok(placed
            .iter()
            .zip(rounded)
            .map(|(((account, product), _), delta)| self.line(*account, *product, delta))
            .collect())
    }

    /// Turns per-category sale amounts into per-product sales (negative lines),
    /// rounded to the unit without selling more of a product than is held here.
    fn sell_lines(
        &self,
        per_category: &[(CategoryId, Decimal)],
        total: Decimal,
    ) -> Result<Vec<ProductLine>, Error> {
        let mut flat: Vec<(ProductId, Decimal)> = Vec::new();
        for (id, share) in per_category {
            if *share > Decimal::ZERO
                && let Some(category) = self.category(*id)
            {
                flat.extend(self.split_sell(category, *share));
            }
        }
        self.sell_lines_flat(flat, total)
    }

    /// Turns per-product sale amounts into sales of the accounts (negative lines), rounded to the
    /// units.
    fn sell_lines_flat(
        &self,
        flat: Vec<(ProductId, Decimal)>,
        total: Decimal,
    ) -> Result<Vec<ProductLine>, Error> {
        let placed = self.place_sell(flat);
        let rounded = self
            .round_slots(&placed, total, true)
            .ok_or(Error::NotEnoughToSell)?;
        Ok(placed
            .iter()
            .zip(rounded)
            .map(|(((account, product), _), amount)| self.line(*account, *product, -amount))
            .collect())
    }

    // ---- allocating by product weights ---------------------------------------

    /// Every product some account in scope is allowed to buy.
    fn all_candidates(&self) -> Vec<&'a Product> {
        self.portfolio
            .products
            .iter()
            .filter(|p| !self.buyers(p.id).is_empty())
            .collect()
    }

    /// Every product the accounts in scope can sell, with the value they hold.
    fn all_held(&self) -> Vec<(&'a Product, Decimal)> {
        self.portfolio
            .products
            .iter()
            .filter_map(|p| {
                let value: Decimal = self.sellers(p.id).iter().map(|(_, v)| *v).sum();
                (value > Decimal::ZERO).then_some((p, value))
            })
            .collect()
    }

    fn by_product_weights(&self, mode: &Mode) -> Result<Suggestion, Error> {
        match mode {
            Mode::FixedAdd { amount } => {
                check_amount(*amount)?;
                self.fixed_add(*amount, ModeName::FixedAdd, Some(*amount), None)
            }
            Mode::BuyOnlyToTarget => self.product_buy_only_to_target(),
            Mode::FullRebalance => self.product_full_rebalance(),
            Mode::FixedSell { amount } => {
                check_amount(*amount)?;
                self.product_fixed_sell(*amount)
            }
        }
    }

    /// The products that take part, with the share of their total each aims at (the shares add up
    /// to 1) and what is held of them in all accounts.
    fn product_shares(&self) -> Vec<(&'a Product, Decimal, Decimal)> {
        let candidates = self.all_candidates();
        let weights: Vec<Decimal> = candidates.iter().map(|p| self.weight(p.id)).collect();
        let sum: Decimal = weights.iter().copied().sum();
        candidates
            .into_iter()
            .zip(weights)
            .map(|(p, w)| (p, w / sum, self.global_value(p)))
            .collect()
    }

    fn product_buy_only_to_target(&self) -> Result<Suggestion, Error> {
        let rows = self.product_shares();
        let pool: Decimal = rows.iter().map(|(_, _, held)| *held).sum();
        let level = rows
            .iter()
            .map(|(_, share, held)| *held / *share)
            .fold(pool, Decimal::max);
        let required = level - pool;
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
        for (product, share, held) in &rows {
            let need = *share * level - *held;
            let bought: Decimal = suggestion
                .products
                .iter()
                .filter(|l| l.product == product.id)
                .map(|l| l.delta)
                .sum();
            if need - bought >= self.unit() {
                suggestion.notes.push(Note::ProductNotFilled {
                    product: product.id,
                    shortfall: need - bought,
                });
            }
        }
        Ok(suggestion)
    }

    fn product_fixed_sell(&self, amount: Decimal) -> Result<Suggestion, Error> {
        if !self.any_sellable() {
            return Err(Error::NotSellable);
        }
        let sell = floor_to_unit(amount, self.unit());
        if sell <= Decimal::ZERO {
            return Err(Error::AmountBelowUnit);
        }
        if sell > self.sellable_total_of(|_| true) {
            return Err(Error::NotEnoughToSell);
        }
        let flat = self.spread_sell(self.all_held(), sell);
        let lines = self.sell_lines_flat(flat, sell)?;
        let mut notes = Vec::new();
        if sell != amount {
            notes.push(Note::AmountRounded {
                requested: amount,
                applied: sell,
            });
        }
        Ok(self.finish(ModeName::FixedSell, Some(amount), sell, None, lines, notes))
    }

    fn product_full_rebalance(&self) -> Result<Suggestion, Error> {
        let tolerance = self.portfolio.settings.product_tolerance;
        let rows = self.product_shares();
        let pool: Decimal = rows.iter().map(|(_, _, held)| *held).sum();
        // How far each product is from its share of the pool, ignoring small deviations.
        let wanted: Vec<Decimal> = rows
            .iter()
            .map(|(_, share, held)| {
                if pool <= Decimal::ZERO || (*held / pool - *share).abs() <= tolerance {
                    Decimal::ZERO
                } else {
                    *share * pool - *held
                }
            })
            .collect();
        let buy_capacity: Decimal = wanted.iter().filter(|w| **w > Decimal::ZERO).sum();
        let sell_wanted: Decimal = rows
            .iter()
            .zip(&wanted)
            .filter(|(_, w)| **w < Decimal::ZERO)
            .map(|((p, _, _), w)| (-*w).min(self.sellable_total_of(|q| q.id == p.id)))
            .sum();
        let sell_capacity = if self.any_sellable() {
            let picked: BTreeSet<ProductId> = rows
                .iter()
                .zip(&wanted)
                .filter(|(_, w)| **w < Decimal::ZERO)
                .map(|((p, _, _), _)| p.id)
                .collect();
            sell_wanted.min(self.sellable_total_of(|p| picked.contains(&p.id)))
        } else {
            Decimal::ZERO
        };
        let trade = floor_to_unit(buy_capacity.min(sell_capacity), self.unit());

        let mut lines = Vec::new();
        if trade > Decimal::ZERO {
            let buying: Vec<&Product> = rows
                .iter()
                .zip(&wanted)
                .filter(|(_, w)| **w > Decimal::ZERO)
                .map(|((p, _, _), _)| *p)
                .collect();
            let items: Vec<Item> = rows
                .iter()
                .zip(&wanted)
                .filter(|(_, w)| **w > Decimal::ZERO)
                .map(|((p, _, held), w)| Item {
                    weight: self.weight(p.id),
                    current: *held,
                    cap: Some(*w),
                })
                .collect();
            let shares = fill_up(&items, trade).map_err(|_| Error::NothingToBuy)?;
            let flat: Vec<(ProductId, Decimal)> = buying.iter().map(|p| p.id).zip(shares).collect();
            lines.extend(self.buy_lines(flat, trade)?);

            let selling: Vec<(&Product, Decimal)> = rows
                .iter()
                .zip(&wanted)
                .filter(|(_, w)| **w < Decimal::ZERO)
                .filter_map(|((p, _, _), w)| {
                    let here: Decimal = self.sellers(p.id).iter().map(|(_, v)| *v).sum();
                    let cap = (-*w).min(here);
                    (cap > Decimal::ZERO).then_some((*p, cap))
                })
                .collect();
            let items: Vec<Item> = selling
                .iter()
                .map(|(p, cap)| Item {
                    weight: self.weight(p.id),
                    current: -self.global_value(p),
                    cap: Some(*cap),
                })
                .collect();
            let shares = fill_up(&items, trade).map_err(|_| Error::NotEnoughToSell)?;
            let flat: Vec<(ProductId, Decimal)> =
                selling.iter().map(|(p, _)| p.id).zip(shares).collect();
            lines.extend(self.sell_lines_flat(flat, trade)?);
        }

        let mut notes = Vec::new();
        for ((product, _, _), wanted) in rows.iter().zip(&wanted) {
            let done: Decimal = lines
                .iter()
                .filter(|l| l.product == product.id)
                .map(|l| l.delta)
                .sum();
            if (*wanted - done).abs() >= self.unit() {
                notes.push(Note::ProductNotFilled {
                    product: product.id,
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

    // ---- assembling the result ---------------------------------------------

    fn category_deltas(&self, lines: &[ProductLine]) -> BTreeMap<CategoryId, Decimal> {
        let mut deltas: BTreeMap<CategoryId, Decimal> = BTreeMap::new();
        for line in lines {
            if let Some(product) = self
                .portfolio
                .products
                .iter()
                .find(|p| p.id == line.product)
            {
                *deltas.entry(product.category).or_default() += line.delta;
            }
        }
        deltas
    }

    fn finish(
        &self,
        mode: ModeName,
        requested: Option<Decimal>,
        applied: Decimal,
        required_total: Option<Decimal>,
        lines: Vec<ProductLine>,
        notes: Vec<Note>,
    ) -> Suggestion {
        // One line per account and product: trades in the same slot add up, and `after` follows.
        let mut merged: BTreeMap<Slot, ProductLine> = BTreeMap::new();
        for line in lines {
            merged
                .entry((line.account, line.product))
                .and_modify(|m| {
                    m.delta += line.delta;
                    m.after += line.delta;
                })
                .or_insert(line);
        }
        let mut lines: Vec<ProductLine> = merged.into_values().collect();
        lines.retain(|line| !line.delta.is_zero());
        // The last step: a sale is taken from the record types in order (NISA last).
        let mut lines = crate::split::split_sales(self.portfolio, lines);
        lines.sort_by_key(|line| (line.account, line.product));
        let deltas = self.category_deltas(&lines);
        let total_after = self.total + lines.iter().map(|l| l.delta).sum::<Decimal>();
        // Allocating by product weights has no category targets to report against.
        let categories = self
            .portfolio
            .categories
            .iter()
            .filter(|_| !self.by_product)
            .map(|category| {
                let current = self.value_of(category);
                let delta = deltas.get(&category.id).copied().unwrap_or_default();
                CategoryLine {
                    category: category.id,
                    current,
                    delta,
                    after: current + delta,
                    target_after: category.target_weight * total_after,
                }
            })
            .collect();
        Suggestion {
            mode,
            account: self.request_account,
            requested,
            applied,
            required_total,
            total_after,
            categories,
            products: lines,
            notes,
        }
    }
}

/// Converts a holding's value to JPY and refuses amounts that could overflow
/// the arithmetic below.
pub(crate) fn to_jpy(
    portfolio: &Portfolio,
    currency: &str,
    value: Decimal,
) -> Result<Decimal, Error> {
    let jpy = portfolio
        .value_in_jpy(currency, value)
        .map_err(|error| match error {
            portfolio::Error::UnsupportedCurrency(code) => Error::UnsupportedCurrency(code),
            portfolio::Error::MissingExchangeRate => Error::MissingExchangeRate,
            _ => Error::TooLarge,
        })?;
    if jpy > MAX_AMOUNT {
        return Err(Error::TooLarge);
    }
    Ok(jpy)
}

/// The step suggested amounts are a multiple of: the least common multiple of the units, worked
/// out exactly on the decimals. A step too large to calculate with is refused.
fn common_unit(units: impl Iterator<Item = Decimal>) -> Result<Decimal, Error> {
    fn gcd(a: u128, b: u128) -> u128 {
        if b == 0 { a } else { gcd(b, a % b) }
    }
    let units: Vec<Decimal> = units.collect();
    let scale = units.iter().map(Decimal::scale).max().unwrap_or(0);
    let factor = 10u128.pow(scale);
    let mut lcm: u128 = 1;
    for unit in &units {
        // The unit as a whole number of 10^-scale.
        let whole = (*unit * Decimal::from(factor))
            .to_u128()
            .filter(|w| *w > 0)
            .ok_or(Error::IncompatibleUnits)?;
        lcm = (lcm / gcd(lcm, whole))
            .checked_mul(whole)
            .filter(|next| *next <= 1_000_000_000_000_000_000)
            .ok_or(Error::IncompatibleUnits)?;
    }
    let step = Decimal::from_i128_with_scale(
        i128::try_from(lcm).map_err(|_| Error::IncompatibleUnits)?,
        scale,
    );
    Ok(step)
}

/// Splits `amount` in proportion to `caps`, never giving an item more than its cap.
fn proportional(caps: &[Decimal], amount: Decimal) -> Vec<Decimal> {
    let total: Decimal = caps.iter().copied().sum();
    if total <= Decimal::ZERO {
        return vec![Decimal::ZERO; caps.len()];
    }
    caps.iter()
        .map(|cap| (amount * *cap / total).min(*cap))
        .collect()
}
