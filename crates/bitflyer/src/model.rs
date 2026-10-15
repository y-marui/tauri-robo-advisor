use std::str::FromStr;

use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json::value::RawValue;

use crate::Error;

/// One row of `GET /v1/me/getbalance`.
#[derive(Debug, Clone, PartialEq)]
pub struct Balance {
    pub currency_code: String,
    /// Total amount, including funds locked in open orders.
    pub amount: Decimal,
    /// Amount that is free to trade.
    pub available: Decimal,
}

// bitFlyer sends amounts as JSON numbers. Reading them as `f64` would round, so
// the raw text is kept and parsed into a `Decimal` instead.
#[derive(Deserialize)]
struct RawBalance<'a> {
    currency_code: String,
    #[serde(borrow)]
    amount: &'a RawValue,
    #[serde(borrow)]
    available: &'a RawValue,
}

#[derive(Deserialize)]
struct RawTicker<'a> {
    #[serde(borrow)]
    ltp: &'a RawValue,
}

#[derive(Deserialize)]
struct RawApiError {
    error_message: Option<String>,
}

pub(crate) fn parse_balances(body: &str) -> Result<Vec<Balance>, Error> {
    let rows: Vec<RawBalance> =
        serde_json::from_str(body).map_err(|e| Error::Format(e.to_string()))?;
    rows.into_iter()
        .map(|row| {
            validate_currency_code(&row.currency_code)?;
            Ok(Balance {
                amount: non_negative(parse_decimal(row.amount)?)?,
                available: non_negative(parse_decimal(row.available)?)?,
                currency_code: row.currency_code,
            })
        })
        .collect()
}

pub(crate) fn parse_last_price(body: &str) -> Result<Decimal, Error> {
    let ticker: RawTicker = serde_json::from_str(body).map_err(|e| Error::Format(e.to_string()))?;
    let price = parse_decimal(ticker.ltp)?;
    if price <= Decimal::ZERO {
        return Err(Error::Format("the last price must be positive".to_owned()));
    }
    Ok(price)
}

/// Extracts bitFlyer's `error_message` from an error body, bounded and stripped
/// of control characters. The rest of the body is never echoed.
pub(crate) fn parse_error_message(body: &str) -> String {
    let message = serde_json::from_str::<RawApiError>(body)
        .ok()
        .and_then(|e| e.error_message)
        .unwrap_or_else(|| "no message".to_owned());
    message
        .chars()
        .filter(|c| !c.is_control())
        .take(200)
        .collect()
}

/// A currency code is later placed in a request URL, so only a short
/// upper-case alphanumeric code is accepted.
pub(crate) fn validate_currency_code(code: &str) -> Result<(), Error> {
    let valid = (1..=10).contains(&code.len())
        && code
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit());
    if valid {
        Ok(())
    } else {
        Err(Error::Format("invalid currency code".to_owned()))
    }
}

fn parse_decimal(raw: &RawValue) -> Result<Decimal, Error> {
    let text = raw.get();
    Decimal::from_str(text)
        .or_else(|_| Decimal::from_scientific(text))
        .map_err(|_| Error::Format("an amount is not a valid number".to_owned()))
}

fn non_negative(value: Decimal) -> Result<Decimal, Error> {
    if value.is_sign_negative() {
        Err(Error::Format("an amount is negative".to_owned()))
    } else {
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::dec;

    #[test]
    fn parse_balances_keeps_the_exact_decimal_text() {
        let body = r#"[{"currency_code":"BTC","amount":0.00000001,"available":0.1}]"#;

        let balances = parse_balances(body).unwrap();

        assert_eq!(balances[0].amount, dec!(0.00000001));
        assert_eq!(balances[0].available, dec!(0.1));
    }

    #[test]
    fn parse_balances_accepts_integers_and_scientific_notation() {
        let body = r#"[{"currency_code":"JPY","amount":1000,"available":1e3}]"#;

        let balances = parse_balances(body).unwrap();

        assert_eq!(balances[0].amount, dec!(1000));
        assert_eq!(balances[0].available, dec!(1000));
    }

    #[test]
    fn parse_balances_rejects_unsafe_currency_codes() {
        for code in ["btc", "BTC_JPY", "BT C", "", "A_VERY_LONG_CODE", "BTC&x=1"] {
            let body = format!(r#"[{{"currency_code":"{code}","amount":1,"available":1}}]"#);

            assert!(
                matches!(parse_balances(&body), Err(Error::Format(_))),
                "{code}"
            );
        }
    }

    #[test]
    fn parse_balances_rejects_negative_amounts_and_non_numbers() {
        let negative = r#"[{"currency_code":"BTC","amount":-1,"available":0}]"#;
        let text = r#"[{"currency_code":"BTC","amount":"1","available":0}]"#;
        let not_array = r#"{"status":-500}"#;

        assert!(matches!(parse_balances(negative), Err(Error::Format(_))));
        assert!(matches!(parse_balances(text), Err(Error::Format(_))));
        assert!(matches!(parse_balances(not_array), Err(Error::Format(_))));
    }

    #[test]
    fn parse_last_price_requires_a_positive_price() {
        assert_eq!(
            parse_last_price(r#"{"ltp":4000000.5}"#).unwrap(),
            dec!(4000000.5)
        );
        assert!(parse_last_price(r#"{"ltp":0}"#).is_err());
        assert!(parse_last_price(r#"{"ltp":-1}"#).is_err());
        assert!(parse_last_price(r#"{}"#).is_err());
    }

    #[test]
    fn parse_error_message_is_bounded_and_ignores_other_fields() {
        let long = "x".repeat(500);
        let body = format!(r#"{{"status":-1,"error_message":"{long}","data":"secret"}}"#);

        let message = parse_error_message(&body);

        assert_eq!(message.len(), 200);
        assert!(!message.contains("secret"));
        assert_eq!(parse_error_message("<html>oops</html>"), "no message");
    }

    #[test]
    fn parse_error_message_strips_control_characters() {
        let body = r#"{"error_message":"line1\nline2\u001b[31m"}"#;

        assert_eq!(parse_error_message(body), "line1line2[31m");
    }
}
