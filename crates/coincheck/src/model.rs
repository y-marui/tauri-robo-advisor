use std::collections::BTreeMap;
use std::str::FromStr;

use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json::value::RawValue;

use crate::Error;

/// One currency of `GET /api/accounts/balance`.
#[derive(Debug, Clone, PartialEq)]
pub struct Balance {
    /// Upper-case currency code, e.g. `BTC` or `JPY`.
    pub currency: String,
    /// Amount held: the free amount plus the amount reserved by open orders
    /// (still owned, only locked). Lending, debt and the monthly accumulation
    /// (`_tsumitate`) fields are not counted.
    pub amount: Decimal,
    /// Amount that is free to trade.
    pub available: Decimal,
}

#[derive(Deserialize)]
struct RawRate<'a> {
    #[serde(borrow)]
    rate: &'a RawValue,
}

#[derive(Deserialize)]
struct RawApiError {
    error: Option<String>,
}

/// The balance response is one flat object: `success`, then per currency
/// `<code>`, `<code>_reserved`, `<code>_lending`, ... Amounts are decimal
/// strings; the raw text is kept so nothing is rounded through `f64`.
pub(crate) fn parse_balances(body: &str) -> Result<Vec<Balance>, Error> {
    let fields: BTreeMap<String, &RawValue> =
        serde_json::from_str(body).map_err(|e| Error::Format(e.to_string()))?;
    match fields.get("success").map(|v| v.get()) {
        Some("true") => {}
        Some("false") => {
            return Err(Error::Http {
                status: 200,
                message: parse_error_message(body),
            });
        }
        _ => return Err(Error::Format("the response has no success flag".to_owned())),
    }
    let mut balances = Vec::new();
    for (name, raw) in &fields {
        // `success` and every `<code>_<kind>` field are not the free amount.
        if name == "success" || name.contains('_') {
            continue;
        }
        validate_currency_code(name)?;
        let available = non_negative(parse_decimal(raw)?)?;
        let reserved = match fields.get(format!("{name}_reserved").as_str()) {
            Some(raw) => non_negative(parse_decimal(raw)?)?,
            None => Decimal::ZERO,
        };
        balances.push(Balance {
            currency: name.to_ascii_uppercase(),
            amount: available.checked_add(reserved).ok_or(Error::Overflow)?,
            available,
        });
    }
    Ok(balances)
}

/// The sales rate from `GET /api/rate/<pair>` (`{"rate":"..."}`).
pub(crate) fn parse_sales_rate(body: &str) -> Result<Decimal, Error> {
    let rate: RawRate = serde_json::from_str(body).map_err(|e| Error::Format(e.to_string()))?;
    let price = parse_decimal(rate.rate)?;
    if price <= Decimal::ZERO {
        return Err(Error::Format("the sales rate must be positive".to_owned()));
    }
    Ok(price)
}

/// Whether a body says `success: false` (for a rate: the pair does not exist).
pub(crate) fn is_failure(body: &str) -> bool {
    serde_json::from_str::<BTreeMap<String, &RawValue>>(body)
        .is_ok_and(|f| f.get("success").is_some_and(|v| v.get() == "false"))
}

/// Extracts Coincheck's `error` text from an error body, bounded and stripped
/// of control characters. The rest of the body is never echoed.
pub(crate) fn parse_error_message(body: &str) -> String {
    let message = serde_json::from_str::<RawApiError>(body)
        .ok()
        .and_then(|e| e.error)
        .unwrap_or_else(|| "no message".to_owned());
    message
        .chars()
        .filter(|c| !c.is_control())
        .take(200)
        .collect()
}

/// A currency code is later placed in a request URL, so only a short
/// lower-case alphanumeric code is accepted.
pub(crate) fn validate_currency_code(code: &str) -> Result<(), Error> {
    let valid = (1..=10).contains(&code.len())
        && code
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit());
    if valid {
        Ok(())
    } else {
        Err(Error::Format("invalid currency code".to_owned()))
    }
}

// Amounts are normally JSON strings; a bare number is accepted too.
fn parse_decimal(raw: &RawValue) -> Result<Decimal, Error> {
    let text = raw.get();
    let text = text
        .strip_prefix('"')
        .and_then(|t| t.strip_suffix('"'))
        .unwrap_or(text);
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
    fn parse_balances_adds_the_reserved_amount_and_ignores_other_fields() {
        let body = r#"{"success":true,"jpy":"1000.5","btc":"0.00000001","btc_reserved":"0.1",
            "btc_lending":"5","btc_debt":"7","btc_tsumitate":"9","jpy_reserved":"0"}"#;

        let balances = parse_balances(body).unwrap();

        assert_eq!(balances.len(), 2);
        assert_eq!(balances[0].currency, "BTC");
        assert_eq!(balances[0].amount, dec!(0.10000001));
        assert_eq!(balances[0].available, dec!(0.00000001));
        assert_eq!(balances[1].currency, "JPY");
        assert_eq!(balances[1].amount, dec!(1000.5));
    }

    #[test]
    fn parse_balances_reports_a_failure_with_a_bounded_message() {
        let long = "x".repeat(500);
        let body = format!(r#"{{"success":false,"error":"{long}"}}"#);

        let Err(Error::Http { message, .. }) = parse_balances(&body) else {
            panic!("expected an error");
        };

        assert_eq!(message.len(), 200);
    }

    #[test]
    fn parse_balances_rejects_unsafe_codes_negative_amounts_and_bad_shapes() {
        for body in [
            r#"{"success":true,"BTC":"1"}"#,
            r#"{"success":true,"btc&x=1":"1"}"#,
            r#"{"success":true,"btc":"-1"}"#,
            r#"{"success":true,"btc":"abc"}"#,
            r#"{"btc":"1"}"#,
            r#"[1]"#,
        ] {
            assert!(
                matches!(parse_balances(body), Err(Error::Format(_))),
                "{body}"
            );
        }
    }

    #[test]
    fn parse_sales_rate_requires_a_positive_rate_in_any_number_form() {
        assert_eq!(
            parse_sales_rate(r#"{"rate":4000000.5}"#).unwrap(),
            dec!(4000000.5)
        );
        assert_eq!(parse_sales_rate(r#"{"rate":"12.5"}"#).unwrap(), dec!(12.5));
        assert!(parse_sales_rate(r#"{"rate":0}"#).is_err());
        assert!(parse_sales_rate(r#"{"rate":-1}"#).is_err());
        assert!(parse_sales_rate(r#"{}"#).is_err());
    }

    #[test]
    fn parse_error_message_strips_control_characters_and_other_fields() {
        let body = r#"{"error":"line1\nline2\u001b[31m","data":"secret"}"#;

        assert_eq!(parse_error_message(body), "line1line2[31m");
        assert_eq!(parse_error_message("<html>oops</html>"), "no message");
    }

    #[test]
    fn is_failure_detects_the_success_flag() {
        assert!(is_failure(r#"{"success":false,"error":"x"}"#));
        assert!(!is_failure(r#"{"rate":"1"}"#));
        assert!(!is_failure("not json"));
    }
}
