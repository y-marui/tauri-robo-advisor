//! Portfolio rebalancing proposals.
//!
//! The crate is pure calculation: it knows nothing about Tauri, brokerages, or
//! storage, so it can be tested in isolation and extracted into its own
//! repository later (see `docs/architecture.md`). All amounts use exact decimal
//! arithmetic and are never floating point.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Largest allowed distance between the sum of target weights and 1.
///
/// Weights are typically typed by hand as percentages, so a tiny tolerance
/// absorbs rounding such as 33.33% + 33.33% + 33.34%.
const WEIGHT_SUM_TOLERANCE: Decimal = Decimal::from_parts(1, 0, 0, false, 6);

/// A position currently held, valued in a single common currency.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Holding {
    pub symbol: String,
    pub value: Decimal,
}

/// The desired share of the portfolio for one symbol, as a fraction of 1.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Target {
    pub symbol: String,
    pub weight: Decimal,
}

/// Inputs that shape a proposal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Params {
    /// New money to invest on top of the current holdings.
    pub contribution: Decimal,
    /// Deviation from the target weight, as a fraction of 1, that is tolerated
    /// before a symbol is proposed for trading.
    pub tolerance: Decimal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Buy,
    Sell,
    Hold,
}

/// The proposed change for one symbol.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Proposal {
    pub symbol: String,
    pub current_value: Decimal,
    pub target_value: Decimal,
    /// Positive to buy, negative to sell, zero to hold.
    pub delta: Decimal,
    pub action: Action,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("target weights must be between 0 and 1 and sum to 1 (sum was {0})")]
    InvalidWeights(Decimal),
    #[error("duplicate symbol: {0}")]
    DuplicateSymbol(String),
    #[error("holding values and the contribution must not be negative")]
    NegativeAmount,
    #[error("tolerance must not be negative")]
    NegativeTolerance,
    #[error("the total value after contribution must be positive")]
    EmptyPortfolio,
    #[error("an amount is too large to calculate exactly")]
    Overflow,
}

/// Computes what to trade to move the holdings toward the targets.
///
/// Symbols held but absent from the targets are treated as having a target
/// weight of 0, so they are proposed for sale. A symbol within `tolerance` of
/// its target weight (measured against the total after the contribution) is
/// held. The result is sorted by symbol.
pub fn propose(
    holdings: &[Holding],
    targets: &[Target],
    params: &Params,
) -> Result<Vec<Proposal>, Error> {
    validate(holdings, targets, params)?;

    // Inputs come across the IPC boundary, so every operation is checked: an
    // overflow panic would abort the whole app in release builds.
    let total = holdings
        .iter()
        .try_fold(params.contribution, |acc, h| acc.checked_add(h.value))
        .ok_or(Error::Overflow)?;
    if total <= Decimal::ZERO {
        return Err(Error::EmptyPortfolio);
    }

    let mut symbols: Vec<&str> = holdings
        .iter()
        .map(|h| h.symbol.as_str())
        .chain(targets.iter().map(|t| t.symbol.as_str()))
        .collect();
    symbols.sort_unstable();
    symbols.dedup();

    symbols
        .into_iter()
        .map(|symbol| {
            let current_value = holdings
                .iter()
                .find(|h| h.symbol == symbol)
                .map_or(Decimal::ZERO, |h| h.value);
            let weight = targets
                .iter()
                .find(|t| t.symbol == symbol)
                .map_or(Decimal::ZERO, |t| t.weight);
            let target_value = weight.checked_mul(total).ok_or(Error::Overflow)?;
            let delta = target_value
                .checked_sub(current_value)
                .ok_or(Error::Overflow)?;
            let deviation = current_value
                .checked_div(total)
                .and_then(|share| share.checked_sub(weight))
                .ok_or(Error::Overflow)?
                .abs();
            let action = if deviation <= params.tolerance || delta.is_zero() {
                Action::Hold
            } else if delta.is_sign_positive() {
                Action::Buy
            } else {
                Action::Sell
            };
            Ok(Proposal {
                symbol: symbol.to_owned(),
                current_value,
                target_value,
                delta: if action == Action::Hold {
                    Decimal::ZERO
                } else {
                    delta
                },
                action,
            })
        })
        .collect()
}

fn validate(holdings: &[Holding], targets: &[Target], params: &Params) -> Result<(), Error> {
    if params.tolerance.is_sign_negative() {
        return Err(Error::NegativeTolerance);
    }
    if params.contribution.is_sign_negative() || holdings.iter().any(|h| h.value.is_sign_negative())
    {
        return Err(Error::NegativeAmount);
    }
    if let Some(symbol) = first_duplicate(holdings.iter().map(|h| h.symbol.as_str()))
        .or_else(|| first_duplicate(targets.iter().map(|t| t.symbol.as_str())))
    {
        return Err(Error::DuplicateSymbol(symbol.to_owned()));
    }
    // Range first: once every weight is within 0..=1 the sum cannot overflow.
    let sum = targets
        .iter()
        .try_fold(Decimal::ZERO, |acc, t| acc.checked_add(t.weight));
    let in_range = targets
        .iter()
        .all(|t| t.weight >= Decimal::ZERO && t.weight <= Decimal::ONE);
    match sum {
        Some(sum) if in_range && (sum - Decimal::ONE).abs() <= WEIGHT_SUM_TOLERANCE => Ok(()),
        Some(sum) => Err(Error::InvalidWeights(sum)),
        None => Err(Error::InvalidWeights(Decimal::MAX)),
    }
}

fn first_duplicate<'a>(symbols: impl Iterator<Item = &'a str>) -> Option<&'a str> {
    let mut seen = std::collections::HashSet::new();
    symbols.into_iter().find(|s| !seen.insert(*s))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::dec;

    fn holding(symbol: &str, value: Decimal) -> Holding {
        Holding {
            symbol: symbol.to_owned(),
            value,
        }
    }

    fn target(symbol: &str, weight: Decimal) -> Target {
        Target {
            symbol: symbol.to_owned(),
            weight,
        }
    }

    fn params(contribution: Decimal, tolerance: Decimal) -> Params {
        Params {
            contribution,
            tolerance,
        }
    }

    fn find<'a>(proposals: &'a [Proposal], symbol: &str) -> &'a Proposal {
        proposals.iter().find(|p| p.symbol == symbol).unwrap()
    }

    #[test]
    fn propose_balanced_portfolio_holds_everything() {
        let holdings = [holding("A", dec!(600)), holding("B", dec!(400))];
        let targets = [target("A", dec!(0.6)), target("B", dec!(0.4))];

        let result = propose(&holdings, &targets, &params(dec!(0), dec!(0))).unwrap();

        assert!(result.iter().all(|p| p.action == Action::Hold));
        assert!(result.iter().all(|p| p.delta.is_zero()));
    }

    #[test]
    fn propose_overweight_symbol_is_sold_and_underweight_is_bought() {
        let holdings = [holding("A", dec!(800)), holding("B", dec!(200))];
        let targets = [target("A", dec!(0.5)), target("B", dec!(0.5))];

        let result = propose(&holdings, &targets, &params(dec!(0), dec!(0))).unwrap();

        assert_eq!(find(&result, "A").action, Action::Sell);
        assert_eq!(find(&result, "A").delta, dec!(-300));
        assert_eq!(find(&result, "B").action, Action::Buy);
        assert_eq!(find(&result, "B").delta, dec!(300));
    }

    #[test]
    fn propose_contribution_raises_the_total_used_for_targets() {
        let holdings = [holding("A", dec!(500)), holding("B", dec!(500))];
        let targets = [target("A", dec!(0.5)), target("B", dec!(0.5))];

        let result = propose(&holdings, &targets, &params(dec!(200), dec!(0))).unwrap();

        assert_eq!(find(&result, "A").target_value, dec!(600));
        assert_eq!(find(&result, "A").delta, dec!(100));
        assert_eq!(find(&result, "B").delta, dec!(100));
    }

    #[test]
    fn propose_deviation_within_tolerance_is_held() {
        let holdings = [holding("A", dec!(520)), holding("B", dec!(480))];
        let targets = [target("A", dec!(0.5)), target("B", dec!(0.5))];

        let result = propose(&holdings, &targets, &params(dec!(0), dec!(0.05))).unwrap();

        assert!(result.iter().all(|p| p.action == Action::Hold));
    }

    #[test]
    fn propose_deviation_beyond_tolerance_is_traded() {
        let holdings = [holding("A", dec!(600)), holding("B", dec!(400))];
        let targets = [target("A", dec!(0.5)), target("B", dec!(0.5))];

        let result = propose(&holdings, &targets, &params(dec!(0), dec!(0.05))).unwrap();

        assert_eq!(find(&result, "A").action, Action::Sell);
        assert_eq!(find(&result, "B").action, Action::Buy);
    }

    #[test]
    fn propose_symbol_missing_from_targets_is_sold_in_full() {
        let holdings = [holding("A", dec!(500)), holding("OLD", dec!(500))];
        let targets = [target("A", dec!(1))];

        let result = propose(&holdings, &targets, &params(dec!(0), dec!(0))).unwrap();

        assert_eq!(find(&result, "OLD").action, Action::Sell);
        assert_eq!(find(&result, "OLD").delta, dec!(-500));
    }

    #[test]
    fn propose_symbol_missing_from_holdings_is_bought() {
        let holdings = [holding("A", dec!(1000))];
        let targets = [target("A", dec!(0.5)), target("NEW", dec!(0.5))];

        let result = propose(&holdings, &targets, &params(dec!(0), dec!(0))).unwrap();

        assert_eq!(find(&result, "NEW").current_value, dec!(0));
        assert_eq!(find(&result, "NEW").delta, dec!(500));
    }

    #[test]
    fn propose_amounts_are_exact_decimals() {
        let holdings = [holding("A", dec!(0.1)), holding("B", dec!(0.2))];
        let targets = [target("A", dec!(0.5)), target("B", dec!(0.5))];

        let result = propose(&holdings, &targets, &params(dec!(0), dec!(0))).unwrap();

        assert_eq!(find(&result, "A").delta, dec!(0.05));
        assert_eq!(find(&result, "B").delta, dec!(-0.05));
    }

    #[test]
    fn propose_result_is_sorted_by_symbol() {
        let holdings = [holding("B", dec!(1)), holding("A", dec!(1))];
        let targets = [target("B", dec!(0.5)), target("A", dec!(0.5))];

        let result = propose(&holdings, &targets, &params(dec!(0), dec!(0))).unwrap();

        let symbols: Vec<_> = result.iter().map(|p| p.symbol.as_str()).collect();
        assert_eq!(symbols, ["A", "B"]);
    }

    #[test]
    fn propose_rejects_weights_that_do_not_sum_to_one() {
        let holdings = [holding("A", dec!(100))];
        let targets = [target("A", dec!(0.9))];

        let result = propose(&holdings, &targets, &params(dec!(0), dec!(0)));

        assert_eq!(result, Err(Error::InvalidWeights(dec!(0.9))));
    }

    #[test]
    fn propose_accepts_weights_with_small_rounding_error() {
        let holdings = [holding("A", dec!(100))];
        let targets = [
            target("A", dec!(0.3333)),
            target("B", dec!(0.3333)),
            target("C", dec!(0.3334)),
        ];

        assert!(propose(&holdings, &targets, &params(dec!(0), dec!(0))).is_ok());
    }

    #[test]
    fn propose_rejects_out_of_range_weight() {
        let targets = [target("A", dec!(1.5)), target("B", dec!(-0.5))];

        let result = propose(&[], &targets, &params(dec!(100), dec!(0)));

        assert_eq!(result, Err(Error::InvalidWeights(dec!(1.0))));
    }

    #[test]
    fn propose_rejects_duplicate_symbols() {
        let holdings = [holding("A", dec!(1)), holding("A", dec!(2))];
        let targets = [target("A", dec!(1))];

        let result = propose(&holdings, &targets, &params(dec!(0), dec!(0)));

        assert_eq!(result, Err(Error::DuplicateSymbol("A".to_owned())));
    }

    #[test]
    fn propose_rejects_negative_amounts_and_tolerance() {
        let targets = [target("A", dec!(1))];

        assert_eq!(
            propose(
                &[holding("A", dec!(-1))],
                &targets,
                &params(dec!(0), dec!(0))
            ),
            Err(Error::NegativeAmount)
        );
        assert_eq!(
            propose(&[], &targets, &params(dec!(-1), dec!(0))),
            Err(Error::NegativeAmount)
        );
        assert_eq!(
            propose(&[], &targets, &params(dec!(1), dec!(-0.1))),
            Err(Error::NegativeTolerance)
        );
    }

    #[test]
    fn propose_returns_overflow_instead_of_panicking_on_huge_amounts() {
        let holdings = [holding("A", Decimal::MAX), holding("B", Decimal::MAX)];
        let targets = [target("A", dec!(0.5)), target("B", dec!(0.5))];

        let result = propose(&holdings, &targets, &params(dec!(0), dec!(0)));

        assert_eq!(result, Err(Error::Overflow));
    }

    #[test]
    fn propose_returns_overflow_when_contribution_overflows_the_total() {
        let holdings = [holding("A", Decimal::MAX)];
        let targets = [target("A", dec!(1))];

        let result = propose(&holdings, &targets, &params(Decimal::MAX, dec!(0)));

        assert_eq!(result, Err(Error::Overflow));
    }

    #[test]
    fn propose_rejects_huge_weights_without_overflowing_the_sum() {
        let targets = [target("A", Decimal::MAX), target("B", Decimal::MAX)];

        let result = propose(&[], &targets, &params(dec!(1), dec!(0)));

        assert_eq!(result, Err(Error::InvalidWeights(Decimal::MAX)));
    }

    #[test]
    fn propose_rejects_empty_portfolio() {
        let targets = [target("A", dec!(1))];

        let result = propose(&[], &targets, &params(dec!(0), dec!(0)));

        assert_eq!(result, Err(Error::EmptyPortfolio));
    }
}
