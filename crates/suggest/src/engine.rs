use std::collections::BTreeMap;

use portfolio::{Account, Category, CategoryId, Portfolio, Product, ProductId, ProductTargeting};
use rust_decimal::Decimal;

use crate::fill::{Item, fill_up};
use crate::round::{ceil_to_unit, floor_to_unit, round_to_units};
use crate::{CategoryLine, Error, Mode, ModeName, Note, ProductLine, Request, Suggestion};

/// Amounts above this (one quadrillion yen) are refused, so none of the
/// arithmetic below can overflow a `Decimal`.
pub(crate) const MAX_AMOUNT: Decimal = Decimal::from_parts(2_764_472_320, 232_830, 0, false, 0);
/// Product weights are relative, so a huge one is a typing mistake.
pub(crate) const MAX_WEIGHT: Decimal = Decimal::from_parts(1_000_000_000, 0, 0, false, 0);
/// A requirement below this is rounding noise, not a real need.
const NOISE: Decimal = Decimal::from_parts(1, 0, 0, false, 9);

/// Computes a suggestion for one account.
pub fn suggest(portfolio: &Portfolio, request: &Request) -> Result<Suggestion, Error> {
    portfolio
        .validate_integrity()
        .map_err(|_| Error::Inconsistent)?;
    let issues = portfolio.issues();
    if !issues.is_empty() {
        return Err(Error::Blocked(issues));
    }
    let book = Book::new(portfolio, request)?;
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

/// The portfolio valued in JPY, seen from one account.
struct Book<'a> {
    portfolio: &'a Portfolio,
    account: &'a Account,
    /// This account's holdings per product.
    account_value: BTreeMap<ProductId, Decimal>,
    /// Holdings per product across all accounts.
    product_value: BTreeMap<ProductId, Decimal>,
    category_value: BTreeMap<CategoryId, Decimal>,
    total: Decimal,
}

impl<'a> Book<'a> {
    fn new(portfolio: &'a Portfolio, request: &Request) -> Result<Self, Error> {
        let account = portfolio
            .accounts
            .iter()
            .find(|a| a.id == request.account)
            .ok_or(Error::UnknownAccount)?;
        if portfolio
            .holdings
            .iter()
            .any(|h| h.weight.is_some_and(|w| w > MAX_WEIGHT))
        {
            return Err(Error::TooLarge);
        }

        let mut account_value = BTreeMap::new();
        let mut product_value: BTreeMap<ProductId, Decimal> = BTreeMap::new();
        for holding in &portfolio.holdings {
            let product = portfolio
                .products
                .iter()
                .find(|p| p.id == holding.product)
                .ok_or(Error::Inconsistent)?;
            let value = to_jpy(portfolio, &product.currency, holding.value)?;
            *product_value.entry(product.id).or_default() += value;
            if holding.account == account.id {
                *account_value.entry(product.id).or_default() += value;
            }
        }

        let mut category_value: BTreeMap<CategoryId, Decimal> = BTreeMap::new();
        for product in &portfolio.products {
            let value = product_value.get(&product.id).copied().unwrap_or_default();
            *category_value.entry(product.category).or_default() += value;
        }
        let total: Decimal = category_value.values().copied().sum();
        if total > MAX_AMOUNT {
            return Err(Error::TooLarge);
        }
        if total <= Decimal::ZERO || portfolio.categories.is_empty() {
            return Err(Error::EmptyPortfolio);
        }
        Ok(Self {
            portfolio,
            account,
            account_value,
            product_value,
            category_value,
            total,
        })
    }

    fn unit(&self) -> Decimal {
        self.account.rounding_unit
    }

    fn value_of(&self, category: &Category) -> Decimal {
        self.category_value
            .get(&category.id)
            .copied()
            .unwrap_or_default()
    }

    fn global_value(&self, product: &Product) -> Decimal {
        self.product_value
            .get(&product.id)
            .copied()
            .unwrap_or_default()
    }

    fn category(&self, id: CategoryId) -> Option<&'a Category> {
        self.portfolio.categories.iter().find(|c| c.id == id)
    }

    /// Products of the category this account is allowed to buy.
    fn candidates(&self, category: CategoryId) -> Vec<&'a Product> {
        self.portfolio
            .products
            .iter()
            .filter(|p| p.category == category)
            .filter(|p| {
                self.account
                    .tradable
                    .as_ref()
                    .is_none_or(|list| list.contains(&p.id))
            })
            .collect()
    }

    /// Products of the category this account holds, with the value held here.
    fn held(&self, category: CategoryId) -> Vec<(&'a Product, Decimal)> {
        self.portfolio
            .products
            .iter()
            .filter(|p| p.category == category)
            .filter_map(|p| {
                let value = self.account_value.get(&p.id).copied().unwrap_or_default();
                (value > Decimal::ZERO).then_some((p, value))
            })
            .collect()
    }

    fn held_total(&self, category: CategoryId) -> Decimal {
        self.held(category).iter().map(|(_, value)| *value).sum()
    }

    // ---- splitting a category's amount over its products -------------------

    fn split_buy(
        &self,
        category: &Category,
        amount: Decimal,
    ) -> Result<Vec<(ProductId, Decimal)>, Error> {
        let candidates = self.candidates(category.id);
        let all_zero = candidates
            .iter()
            .all(|p| self.global_value(p) <= Decimal::ZERO);
        let items: Vec<Item> = candidates
            .iter()
            .map(|p| {
                let value = self.global_value(p);
                match category.targeting {
                    ProductTargeting::Free => Item {
                        weight: Decimal::ONE,
                        current: Decimal::ZERO,
                        cap: None,
                    },
                    ProductTargeting::Equal => Item {
                        weight: Decimal::ONE,
                        current: value,
                        cap: None,
                    },
                    ProductTargeting::Weighted => Item {
                        weight: self.portfolio.product_weight(p.id).unwrap_or_default(),
                        current: value,
                        cap: None,
                    },
                    ProductTargeting::KeepCurrent if all_zero => Item {
                        weight: Decimal::ONE,
                        current: Decimal::ZERO,
                        cap: None,
                    },
                    ProductTargeting::KeepCurrent => Item {
                        weight: value,
                        current: value,
                        cap: None,
                    },
                }
            })
            .collect();
        let shares = fill_up(&items, amount).map_err(|_| Error::NothingToBuy)?;
        Ok(candidates.iter().map(|p| p.id).zip(shares).collect())
    }

    fn split_sell(&self, category: &Category, amount: Decimal) -> Vec<(ProductId, Decimal)> {
        let held = self.held(category.id);
        let items: Vec<Item> = held
            .iter()
            .map(|(p, here)| {
                let value = self.global_value(p);
                let (weight, current) = match category.targeting {
                    ProductTargeting::Free => (*here, -*here),
                    ProductTargeting::Equal => (Decimal::ONE, -value),
                    ProductTargeting::Weighted => (
                        self.portfolio.product_weight(p.id).unwrap_or_default(),
                        -value,
                    ),
                    ProductTargeting::KeepCurrent => (value, -value),
                };
                Item {
                    weight,
                    current,
                    cap: Some(*here),
                }
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
        for (id, share) in self.fill_categories_buy(spend)? {
            if share > Decimal::ZERO
                && let Some(category) = self.category(id)
            {
                flat.extend(self.split_buy(category, share)?);
            }
        }
        let values: Vec<Decimal> = flat.iter().map(|(_, value)| *value).collect();
        let rounded =
            round_to_units(&values, self.unit(), spend, &[]).ok_or(Error::NothingToBuy)?;
        let lines = flat
            .iter()
            .zip(rounded)
            .map(|((product, _), delta)| ProductLine {
                product: *product,
                delta,
            })
            .collect();

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
        if !self.account.sellable {
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
            .map(|c| (c.id, self.held_total(c.id)))
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
        let target = self
            .portfolio
            .settings
            .target_account
            .ok_or(Error::NoTargetAccount)?;
        if target != self.account.id {
            return Err(Error::NotTargetAccount);
        }
        let tolerance = self.portfolio.settings.category_tolerance;

        // How far each category is from its target, ignoring small deviations.
        let delta_of = |category: &Category| {
            let value = self.value_of(category);
            let off = (value / self.total - category.target_weight).abs();
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
            if delta < Decimal::ZERO && self.account.sellable {
                let can_sell = (-delta).min(self.held_total(category.id));
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
            let values: Vec<Decimal> = flat.iter().map(|(_, v)| *v).collect();
            let rounded =
                round_to_units(&values, self.unit(), trade, &[]).ok_or(Error::NothingToBuy)?;
            lines.extend(
                flat.iter()
                    .zip(rounded)
                    .map(|((product, _), delta)| ProductLine {
                        product: *product,
                        delta,
                    }),
            );
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
        let values: Vec<Decimal> = flat.iter().map(|(_, v)| *v).collect();
        let caps: Vec<Option<Decimal>> = flat
            .iter()
            .map(|(product, _)| self.account_value.get(product).copied())
            .collect();
        let rounded =
            round_to_units(&values, self.unit(), total, &caps).ok_or(Error::NotEnoughToSell)?;
        Ok(flat
            .iter()
            .zip(rounded)
            .map(|((product, _), amount)| ProductLine {
                product: *product,
                delta: -amount,
            })
            .collect())
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
        mut lines: Vec<ProductLine>,
        notes: Vec<Note>,
    ) -> Suggestion {
        lines.retain(|line| !line.delta.is_zero());
        lines.sort_by_key(|line| line.product);
        let deltas = self.category_deltas(&lines);
        let total_after = self.total + lines.iter().map(|l| l.delta).sum::<Decimal>();
        let categories = self
            .portfolio
            .categories
            .iter()
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
            account: self.account.id,
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
