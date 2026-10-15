use rust_decimal::Decimal;

use crate::{Error, Portfolio};

impl Portfolio {
    /// Converts an amount in `currency` to JPY.
    ///
    /// Only JPY and USD are supported; the USD rate is the single number the
    /// user enters in the settings. A currency that cannot be converted is an
    /// error rather than a guess, so a wrong total is never shown.
    pub fn value_in_jpy(&self, currency: &str, value: Decimal) -> Result<Decimal, Error> {
        match currency {
            "JPY" => Ok(value),
            "USD" => {
                let rate = self.settings.usd_jpy.ok_or(Error::MissingExchangeRate)?;
                value.checked_mul(rate).ok_or(Error::Overflow)
            }
            other => Err(Error::UnsupportedCurrency(other.to_owned())),
        }
    }
}

#[cfg(test)]
mod tests {
    use rust_decimal::dec;

    use super::*;
    use crate::Edit;

    #[test]
    fn jpy_is_unchanged_and_usd_uses_the_rate() {
        let mut p = Portfolio::default();

        assert_eq!(p.value_in_jpy("JPY", dec!(1234.5)), Ok(dec!(1234.5)));
        assert_eq!(
            p.value_in_jpy("USD", dec!(10)),
            Err(Error::MissingExchangeRate)
        );
        p.apply(Edit::SetExchangeRate {
            usd_jpy: Some(dec!(150.5)),
        })
        .unwrap();
        assert_eq!(p.value_in_jpy("USD", dec!(10)), Ok(dec!(1505)));
    }

    #[test]
    fn other_currencies_and_overflow_are_errors() {
        let mut p = Portfolio::default();
        p.apply(Edit::SetExchangeRate {
            usd_jpy: Some(dec!(150)),
        })
        .unwrap();

        assert_eq!(
            p.value_in_jpy("EUR", dec!(1)),
            Err(Error::UnsupportedCurrency("EUR".to_owned()))
        );
        assert_eq!(p.value_in_jpy("USD", Decimal::MAX), Err(Error::Overflow));
    }
}
