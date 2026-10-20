//! Rounding an amount for display to the smallest unit of its currency.
//!
//! Display only: the portfolio, the calculations and the exports keep the exact
//! decimals. A value of an investment trust (quantity x price) has fractions
//! below a yen, which would be noise on screen.

use rust_decimal::{Decimal, RoundingStrategy};
use serde::Serialize;

use crate::{AccountId, Portfolio, ProductId};

/// The number of decimal places of the currency's smallest unit (ISO 4217 minor
/// units), or `None` for a currency this table does not know.
pub fn minor_units(currency: &str) -> Option<u32> {
    match currency {
        "JPY" | "KRW" | "VND" | "CLP" | "ISK" | "UGX" | "PYG" | "XAF" | "XOF" | "XPF" | "BIF"
        | "DJF" | "GNF" | "KMF" | "RWF" | "VUV" => Some(0),
        "USD" | "EUR" | "GBP" | "AUD" | "CAD" | "CHF" | "CNY" | "HKD" | "NZD" | "SGD" | "SEK"
        | "NOK" | "DKK" | "MXN" | "BRL" | "INR" | "ZAR" | "TWD" | "THB" | "PLN" | "CZK" | "HUF"
        | "ILS" | "AED" | "SAR" | "TRY" | "RUB" | "MYR" | "PHP" | "IDR" | "CZK " | "RON"
        | "BGN" | "HRK" | "ARS" | "COP" | "PEN" | "EGP" | "PKR" | "BDT" | "NGN" | "KES" | "QAR"
        | "UAH" => Some(2),
        "BHD" | "IQD" | "JOD" | "KWD" | "LYD" | "OMR" | "TND" => Some(3),
        "CLF" | "UYW" => Some(4),
        _ => None,
    }
}

/// `amount` rounded half away from zero to the smallest unit of `currency`, with
/// exactly that many decimal places (`10.5` USD is `10.50`). A currency without a
/// known unit is returned as it is.
pub fn round_for_display(currency: &str, amount: Decimal) -> Decimal {
    let Some(places) = minor_units(currency) else {
        return amount;
    };
    let mut rounded = amount.round_dp_with_strategy(places, RoundingStrategy::MidpointAwayFromZero);
    // A small negative amount rounds to a zero that must not be shown with a sign.
    if rounded.is_zero() {
        rounded = Decimal::ZERO;
    }
    // Pads the zeros so 10.5 USD reads 10.50 (a rounded value is always within the scale).
    rounded.rescale(places);
    rounded
}

/// A holding's value as the screen shows it: rounded to the smallest unit of the
/// product's own currency. The exact value stays in the holding.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ShownValue {
    pub account: AccountId,
    pub product: ProductId,
    pub account_type: Option<String>,
    pub value: Decimal,
}

impl Portfolio {
    /// Every holding's value rounded for display, in the order of the holdings.
    /// A holding of an unknown product is skipped (the integrity check reports it).
    pub fn shown_values(&self) -> Vec<ShownValue> {
        self.holdings
            .iter()
            .filter_map(|holding| {
                let product = self.products.iter().find(|p| p.id == holding.product)?;
                Some(ShownValue {
                    account: holding.account,
                    product: holding.product,
                    account_type: holding.account_type.clone(),
                    value: round_for_display(&product.currency, holding.value),
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use rust_decimal::dec;

    use super::*;

    #[test]
    fn yen_has_no_decimals() {
        assert_eq!(round_for_display("JPY", dec!(1234.5678)), dec!(1235));
        assert_eq!(round_for_display("JPY", dec!(1234)).to_string(), "1234");
    }

    #[test]
    fn dollars_have_two_decimals_kept_even_when_zero() {
        assert_eq!(round_for_display("USD", dec!(10.456)).to_string(), "10.46");
        assert_eq!(round_for_display("USD", dec!(10.5)).to_string(), "10.50");
        assert_eq!(round_for_display("USD", dec!(10)).to_string(), "10.00");
    }

    #[test]
    fn a_three_decimal_currency_keeps_three_places() {
        assert_eq!(round_for_display("KWD", dec!(1.23456)).to_string(), "1.235");
    }

    #[test]
    fn an_unknown_currency_is_shown_as_it_is() {
        assert_eq!(
            round_for_display("XYZ", dec!(1.23456789)).to_string(),
            "1.23456789"
        );
    }

    #[test]
    fn halves_go_away_from_zero_not_to_even() {
        assert_eq!(round_for_display("JPY", dec!(0.5)), dec!(1));
        assert_eq!(round_for_display("JPY", dec!(2.5)), dec!(3));
        assert_eq!(round_for_display("JPY", dec!(-0.5)), dec!(-1));
        assert_eq!(round_for_display("JPY", dec!(-2.5)), dec!(-3));
        assert_eq!(round_for_display("USD", dec!(-1.005)).to_string(), "-1.01");
        assert_eq!(round_for_display("JPY", dec!(-0.4)).to_string(), "0");
    }

    #[test]
    fn shown_values_round_by_the_products_currency_and_leave_the_holdings_exact() {
        use crate::Edit;

        let mut p = Portfolio::default();
        let apply = |p: &mut Portfolio, e: Edit| p.apply(e).unwrap();
        apply(
            &mut p,
            Edit::AddCategory {
                name: "Stocks".to_owned(),
                target_weight: dec!(1),
            },
        );
        let category = p.categories[0].id;
        for (name, currency) in [("Yen Fund", "JPY"), ("Dollar Fund", "USD")] {
            apply(
                &mut p,
                Edit::AddProduct {
                    name: name.to_owned(),
                    category,
                    currency: currency.to_owned(),
                },
            );
        }
        apply(
            &mut p,
            Edit::AddAccount {
                name: "Main".to_owned(),
            },
        );
        let (account, yen, dollar) = (p.accounts[0].id, p.products[0].id, p.products[1].id);
        for (product, value) in [(yen, dec!(1234.5678)), (dollar, dec!(10.456))] {
            apply(
                &mut p,
                Edit::SetHolding {
                    account_type: None,
                    account,
                    product,
                    value,
                    quantity: None,
                },
            );
        }

        let shown: Vec<String> = p
            .shown_values()
            .iter()
            .map(|s| s.value.to_string())
            .collect();

        assert_eq!(shown, ["1235", "10.46"]);
        assert_eq!(p.holdings[0].value, dec!(1234.5678));
        assert_eq!(p.holdings[1].value, dec!(10.456));
    }
}
