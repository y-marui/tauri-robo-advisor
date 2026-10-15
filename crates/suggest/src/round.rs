//! Rounding amounts to a unit (for example 1,000 yen) without changing their sum.

use rust_decimal::Decimal;

/// Rounds each value to a multiple of `unit` so the results add up to exactly
/// `total`, which must itself be a multiple of `unit`.
///
/// Every value is rounded down first; the units left over go, one each, to the
/// values with the largest fractional parts (the largest-remainder method), so
/// the rounding error is spread fairly and the sum is exact. Ties go to the
/// earlier value, which keeps the result deterministic. A value with a cap never
/// receives a unit that would push it above the cap; if the units cannot all be
/// placed under the caps, there is no valid rounding and `None` is returned.
pub fn round_to_units(
    values: &[Decimal],
    unit: Decimal,
    total: Decimal,
    caps: &[Option<Decimal>],
) -> Option<Vec<Decimal>> {
    let wanted = (total / unit).round();
    let scaled: Vec<Decimal> = values.iter().map(|v| *v / unit).collect();
    let mut units: Vec<Decimal> = scaled.iter().map(|s| s.floor()).collect();
    let mut remainder = wanted - units.iter().copied().sum::<Decimal>();

    let mut order: Vec<usize> = (0..values.len()).collect();
    order.sort_by(|&a, &b| {
        let fraction = |i: usize| scaled[i] - units[i];
        fraction(b).cmp(&fraction(a)).then(a.cmp(&b))
    });

    let has_room = |units: Decimal, index: usize| {
        caps.get(index)
            .copied()
            .flatten()
            .is_none_or(|cap| (units + Decimal::ONE) * unit <= cap)
    };

    // Give out the missing units, largest fraction first. Repeating the pass
    // lets values that were skipped for lack of room be reconsidered.
    let mut progressed = true;
    while remainder > Decimal::ZERO && progressed {
        progressed = false;
        for &index in &order {
            if remainder <= Decimal::ZERO {
                break;
            }
            if has_room(units[index], index) {
                units[index] += Decimal::ONE;
                remainder -= Decimal::ONE;
                progressed = true;
            }
        }
    }
    if remainder > Decimal::ZERO {
        return None;
    }
    // Rounding noise can leave one unit too many; take it from the smallest
    // fraction that still has a unit to give.
    for &index in order.iter().rev().cycle().take(values.len() * 2) {
        if remainder >= Decimal::ZERO {
            break;
        }
        if units[index] > Decimal::ZERO {
            units[index] -= Decimal::ONE;
            remainder += Decimal::ONE;
        }
    }
    Some(units.into_iter().map(|u| u * unit).collect())
}

/// The largest multiple of `unit` that does not exceed `amount`.
pub fn floor_to_unit(amount: Decimal, unit: Decimal) -> Decimal {
    (amount / unit).floor() * unit
}

/// The smallest multiple of `unit` that is at least `amount`.
pub fn ceil_to_unit(amount: Decimal, unit: Decimal) -> Decimal {
    (amount / unit).ceil() * unit
}

#[cfg(test)]
mod tests {
    use rust_decimal::dec;

    use super::*;

    #[test]
    fn the_rounded_values_add_up_to_the_total() {
        let values = [dec!(3333.33), dec!(3333.33), dec!(3333.34)];

        let rounded = round_to_units(&values, dec!(1000), dec!(10000), &[None; 3]).unwrap();

        assert_eq!(rounded.iter().copied().sum::<Decimal>(), dec!(10000));
        assert!(rounded.iter().all(|r| (*r % dec!(1000)).is_zero()));
    }

    #[test]
    fn the_largest_fraction_gets_the_extra_unit() {
        // 10_000 in units of 1000: floors are 1, 2, 6 (sum 9); fractions are
        // .9, .1, .9 -> the first and third are tied; the earlier one wins.
        let values = [dec!(1900), dec!(2100), dec!(6000)];

        let rounded = round_to_units(&values, dec!(1000), dec!(10000), &[None; 3]).unwrap();

        assert_eq!(rounded, vec![dec!(2000), dec!(2000), dec!(6000)]);
    }

    #[test]
    fn a_sum_that_is_already_exact_is_unchanged() {
        let values = [dec!(4000), dec!(6000)];

        assert_eq!(
            round_to_units(&values, dec!(1000), dec!(10000), &[]).unwrap(),
            vec![dec!(4000), dec!(6000)]
        );
    }

    #[test]
    fn tiny_values_can_round_to_zero() {
        let values = [dec!(10), dec!(9990)];

        let rounded = round_to_units(&values, dec!(1000), dec!(10000), &[None; 3]).unwrap();

        assert_eq!(rounded, vec![dec!(0), dec!(10000)]);
    }

    #[test]
    fn noise_above_the_total_does_not_add_a_unit() {
        let values = [dec!(5000.0000000000000000000000001), dec!(5000)];

        let rounded = round_to_units(&values, dec!(1000), dec!(10000), &[None; 3]).unwrap();

        assert_eq!(rounded.iter().copied().sum::<Decimal>(), dec!(10000));
    }

    #[test]
    fn a_capped_value_never_gets_a_unit_above_its_cap() {
        // The first value has the larger fraction but no room for another unit.
        let values = [dec!(1600), dec!(400)];

        let rounded =
            round_to_units(&values, dec!(1000), dec!(2000), &[Some(dec!(1500)), None]).unwrap();

        assert_eq!(rounded, vec![dec!(1000), dec!(1000)]);
    }

    #[test]
    fn rounding_is_impossible_when_the_caps_leave_no_room() {
        let values = [dec!(1500)];

        assert_eq!(
            round_to_units(&values, dec!(1000), dec!(2000), &[Some(dec!(1500))]),
            None
        );
    }

    #[test]
    fn floor_and_ceil_to_a_unit() {
        assert_eq!(floor_to_unit(dec!(12999), dec!(1000)), dec!(12000));
        assert_eq!(floor_to_unit(dec!(12000), dec!(1000)), dec!(12000));
        assert_eq!(ceil_to_unit(dec!(12001), dec!(1000)), dec!(13000));
        assert_eq!(ceil_to_unit(dec!(12000), dec!(1000)), dec!(12000));
        assert_eq!(floor_to_unit(dec!(999), dec!(1000)), dec!(0));
    }
}
