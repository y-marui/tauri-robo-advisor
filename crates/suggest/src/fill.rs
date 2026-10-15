//! Water-filling: spreading an amount over items so the most underweight ones
//! are raised first.
//!
//! Each item `i` receives `clamp(weight_i * L - current_i, 0, cap_i)` for a
//! common level `L`, chosen so the amounts add up to the requested total. At the
//! start only the item with the lowest `current / weight` receives money; as the
//! amount grows the level rises and more items join, until (with no caps) every
//! item ends at the same ratio, which is exactly its target share. Nothing is
//! ever taken away from an item, so this is the buy-only rule.
//!
//! Selling uses the same function with the sign flipped: an item with value `V`
//! gives `clamp(V - weight * L, 0, cap)`, which equals the buy formula with
//! `current = -V` and the level negated.

use rust_decimal::Decimal;

#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub weight: Decimal,
    pub current: Decimal,
    /// The most this item can receive, if limited.
    pub cap: Option<Decimal>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillError {
    /// The amount is positive but no item can receive anything.
    NothingToFill,
    /// The amount is larger than all items can take together.
    ExceedsCapacity,
}

impl Item {
    fn eligible(&self) -> bool {
        self.weight > Decimal::ZERO && self.cap.is_none_or(|cap| cap > Decimal::ZERO)
    }

    fn allocation(&self, level: Decimal) -> Decimal {
        let raw = (self.weight * level - self.current).max(Decimal::ZERO);
        match self.cap {
            Some(cap) => raw.min(cap),
            None => raw,
        }
    }
}

fn total_at(items: &[Item], level: Decimal) -> Decimal {
    items
        .iter()
        .filter(|item| item.eligible())
        .map(|item| item.allocation(level))
        .sum()
}

/// Splits `amount` over `items`; the result has one amount per item, in order.
pub fn fill_up(items: &[Item], amount: Decimal) -> Result<Vec<Decimal>, FillError> {
    if amount.is_zero() {
        return Ok(vec![Decimal::ZERO; items.len()]);
    }
    if !items.iter().any(Item::eligible) {
        return Err(FillError::NothingToFill);
    }

    // Every place the slope of the total changes: where an item starts
    // receiving money, and where it reaches its cap.
    let mut breakpoints: Vec<Decimal> = Vec::new();
    for item in items.iter().filter(|item| item.eligible()) {
        breakpoints.push(item.current / item.weight);
        if let Some(cap) = item.cap {
            breakpoints.push((item.current + cap) / item.weight);
        }
    }
    breakpoints.sort_unstable();
    breakpoints.dedup();

    let level = locate_level(items, &breakpoints, amount)?;
    Ok(items
        .iter()
        .map(|item| {
            if item.eligible() {
                item.allocation(level)
            } else {
                Decimal::ZERO
            }
        })
        .collect())
}

fn locate_level(
    items: &[Item],
    breakpoints: &[Decimal],
    amount: Decimal,
) -> Result<Decimal, FillError> {
    let mut previous = breakpoints[0];
    for &point in &breakpoints[1..] {
        if total_at(items, point) >= amount {
            return Ok(solve_in_segment(items, previous, point, amount));
        }
        previous = point;
    }
    // Past the last breakpoint only uncapped items still grow.
    let slope: Decimal = items
        .iter()
        .filter(|item| item.eligible() && item.cap.is_none())
        .map(|item| item.weight)
        .sum();
    if slope.is_zero() {
        return Err(FillError::ExceedsCapacity);
    }
    Ok(previous + (amount - total_at(items, previous)) / slope)
}

/// Inside one segment the total is a straight line, so the level is exact.
fn solve_in_segment(items: &[Item], from: Decimal, to: Decimal, amount: Decimal) -> Decimal {
    let slope: Decimal = items
        .iter()
        .filter(|item| item.eligible())
        .filter(|item| {
            let starts = item.current / item.weight;
            let ends = item.cap.map(|cap| (item.current + cap) / item.weight);
            starts <= from && ends.is_none_or(|end| end >= to)
        })
        .map(|item| item.weight)
        .sum();
    if slope.is_zero() {
        return to;
    }
    from + (amount - total_at(items, from)) / slope
}

#[cfg(test)]
mod tests {
    use rust_decimal::dec;

    use super::*;

    fn item(weight: Decimal, current: Decimal) -> Item {
        Item {
            weight,
            current,
            cap: None,
        }
    }

    fn capped(weight: Decimal, current: Decimal, cap: Decimal) -> Item {
        Item {
            weight,
            current,
            cap: Some(cap),
        }
    }

    fn close(a: Decimal, b: Decimal) -> bool {
        (a - b).abs() < dec!(0.000000001)
    }

    #[test]
    fn the_most_underweight_item_is_filled_first() {
        // Ratios are 100/0.5 = 200 and 100/0.5 = 200 -> equal; use unequal ones.
        let items = [item(dec!(0.5), dec!(100)), item(dec!(0.5), dec!(300))];

        let result = fill_up(&items, dec!(100)).unwrap();

        assert!(close(result[0], dec!(100)));
        assert!(close(result[1], dec!(0)));
    }

    #[test]
    fn once_every_item_joins_the_split_follows_the_weights() {
        let items = [item(dec!(0.5), dec!(100)), item(dec!(0.5), dec!(300))];

        let result = fill_up(&items, dec!(400)).unwrap();

        // Total after = 800, so each ends at 400.
        assert!(close(result[0], dec!(300)));
        assert!(close(result[1], dec!(100)));
    }

    #[test]
    fn the_result_adds_up_to_the_amount() {
        let items = [
            item(dec!(0.3), dec!(50)),
            item(dec!(0.2), dec!(10)),
            item(dec!(0.5), dec!(400)),
        ];

        let result = fill_up(&items, dec!(123.45)).unwrap();

        assert!(close(result.iter().copied().sum(), dec!(123.45)));
        assert!(result.iter().all(|r| *r >= Decimal::ZERO));
    }

    #[test]
    fn nothing_is_ever_taken_from_an_item_above_its_share() {
        let items = [item(dec!(0.5), dec!(0)), item(dec!(0.5), dec!(1000))];

        let result = fill_up(&items, dec!(200)).unwrap();

        assert!(close(result[0], dec!(200)));
        assert_eq!(result[1], dec!(0));
    }

    #[test]
    fn caps_limit_an_item_and_the_rest_flows_to_the_others() {
        let items = [
            capped(dec!(0.5), dec!(0), dec!(50)),
            item(dec!(0.5), dec!(0)),
        ];

        let result = fill_up(&items, dec!(200)).unwrap();

        assert!(close(result[0], dec!(50)));
        assert!(close(result[1], dec!(150)));
    }

    #[test]
    fn an_amount_above_the_total_capacity_is_an_error() {
        let items = [
            capped(dec!(1), dec!(0), dec!(50)),
            capped(dec!(1), dec!(0), dec!(30)),
        ];

        assert_eq!(fill_up(&items, dec!(81)), Err(FillError::ExceedsCapacity));
        assert!(fill_up(&items, dec!(80)).is_ok());
    }

    #[test]
    fn items_without_weight_or_capacity_receive_nothing() {
        let items = [
            item(dec!(0), dec!(0)),
            capped(dec!(1), dec!(0), dec!(0)),
            item(dec!(1), dec!(0)),
        ];

        let result = fill_up(&items, dec!(10)).unwrap();

        assert_eq!(result[0], dec!(0));
        assert_eq!(result[1], dec!(0));
        assert!(close(result[2], dec!(10)));
    }

    #[test]
    fn no_eligible_item_is_an_error_unless_the_amount_is_zero() {
        let items = [item(dec!(0), dec!(5))];

        assert_eq!(fill_up(&items, dec!(1)), Err(FillError::NothingToFill));
        assert_eq!(fill_up(&items, dec!(0)).unwrap(), vec![dec!(0)]);
        assert_eq!(fill_up(&[], dec!(0)).unwrap(), Vec::<Decimal>::new());
    }

    #[test]
    fn equal_ratios_split_in_proportion_to_the_weights() {
        let items = [item(dec!(1), dec!(0)), item(dec!(3), dec!(0))];

        let result = fill_up(&items, dec!(400)).unwrap();

        assert!(close(result[0], dec!(100)));
        assert!(close(result[1], dec!(300)));
    }

    #[test]
    fn selling_is_filling_with_negated_current_values() {
        // Two items worth 600 and 400 with equal weights; sell 200. The larger
        // one is above its share, so it gives first: 600 -> 400, 400 stays.
        let items = [item(dec!(0.5), dec!(-600)), item(dec!(0.5), dec!(-400))];

        let result = fill_up(&items, dec!(200)).unwrap();

        assert!(close(result[0], dec!(200)));
        assert!(close(result[1], dec!(0)));
    }

    #[test]
    fn selling_more_levels_both_items_down_equally() {
        let items = [item(dec!(0.5), dec!(-600)), item(dec!(0.5), dec!(-400))];

        let result = fill_up(&items, dec!(500)).unwrap();

        // 1000 - 500 = 500 left, 250 each: sell 350 and 150.
        assert!(close(result[0], dec!(350)));
        assert!(close(result[1], dec!(150)));
    }
}
