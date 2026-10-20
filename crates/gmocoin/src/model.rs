use std::str::FromStr;

use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json::value::RawValue;

use crate::Error;

/// One asset of `GET /private/v1/account/assets`.
#[derive(Debug, Clone, PartialEq)]
pub struct Balance {
    /// Upper-case symbol, e.g. `BTC` or `JPY`.
    pub currency: String,
    /// Amount held (`amount`: the balance, including what is locked by orders).
    pub amount: Decimal,
    /// Amount that is free to use (`available`).
    pub available: Decimal,
    /// The yen rate GMO Coin attaches to the asset (`conversionRate`), or `None`
    /// when it is missing or zero. See [`crate::PriceBasis`] for what it means.
    pub rate: Option<Decimal>,
}

#[derive(Deserialize)]
struct RawEnvelope<'a> {
    status: i64,
    #[serde(borrow)]
    data: Option<Vec<RawAsset<'a>>>,
    messages: Option<Vec<RawMessage>>,
}

#[derive(Deserialize)]
struct RawAsset<'a> {
    symbol: String,
    #[serde(borrow)]
    amount: &'a RawValue,
    #[serde(borrow)]
    available: &'a RawValue,
    #[serde(borrow, rename = "conversionRate")]
    conversion_rate: Option<&'a RawValue>,
}

/// Only the messages: the `data` of an error body is never read.
#[derive(Deserialize)]
struct RawFailure {
    messages: Option<Vec<RawMessage>>,
}

#[derive(Deserialize)]
struct RawMessage {
    message_code: Option<String>,
    message_string: Option<String>,
}

/// Parses the assets response. Amounts are decimal strings; the raw text is
/// kept so nothing is rounded through `f64`. A `status` other than 0 is an
/// error reported with GMO Coin's own code and text.
pub(crate) fn parse_assets(body: &str) -> Result<Vec<Balance>, Error> {
    let envelope: RawEnvelope =
        serde_json::from_str(body).map_err(|e| Error::Format(e.to_string()))?;
    if envelope.status != 0 {
        return Err(Error::Http {
            status: 200,
            message: error_message(envelope.messages.as_deref()),
        });
    }
    let data = envelope
        .data
        .ok_or_else(|| Error::Format("the response has no data".to_owned()))?;
    let mut balances = Vec::new();
    for asset in data {
        validate_symbol(&asset.symbol)?;
        balances.push(Balance {
            currency: asset.symbol,
            amount: non_negative(parse_decimal(asset.amount)?)?,
            available: non_negative(parse_decimal(asset.available)?)?,
            rate: match asset.conversion_rate {
                Some(raw) => positive_or_none(non_negative(parse_decimal(raw)?)?),
                None => None,
            },
        });
    }
    Ok(balances)
}

/// GMO Coin reports failures as `status` != 0 with `messages`, usually with
/// HTTP 200. Only the code and text are echoed, bounded and stripped of control
/// characters; the rest of the body is never shown.
fn error_message(messages: Option<&[RawMessage]>) -> String {
    let text = messages
        .and_then(<[RawMessage]>::first)
        .map(|m| {
            format!(
                "{} {}",
                m.message_code.as_deref().unwrap_or(""),
                m.message_string.as_deref().unwrap_or("")
            )
        })
        .unwrap_or_default();
    let text: String = text.chars().filter(|c| !c.is_control()).take(200).collect();
    let text = text.trim();
    if text.is_empty() {
        "no message".to_owned()
    } else {
        text.to_owned()
    }
}

/// Extracts the message of an error body (HTTP status other than 2xx).
pub(crate) fn parse_error_message(body: &str) -> String {
    serde_json::from_str::<RawFailure>(body).ok().map_or_else(
        || "no message".to_owned(),
        |e| error_message(e.messages.as_deref()),
    )
}

/// A symbol is shown to the user and matched against names, so only a short
/// upper-case alphanumeric code is accepted.
fn validate_symbol(code: &str) -> Result<(), Error> {
    let valid = (1..=10).contains(&code.len())
        && code
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit());
    if valid {
        Ok(())
    } else {
        Err(Error::Format("invalid symbol".to_owned()))
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

fn positive_or_none(value: Decimal) -> Option<Decimal> {
    (value > Decimal::ZERO).then_some(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::dec;

    #[test]
    fn parse_assets_reads_amounts_and_rates_as_decimals() {
        let body = r#"{"status":0,"data":[
            {"amount":"993982448","available":"993982448","conversionRate":"1","symbol":"JPY"},
            {"amount":"0.00000001","available":"0","conversionRate":"859614.5","symbol":"BTC"}],
            "responsetime":"2019-03-19T02:15:06.055Z"}"#;

        let balances = parse_assets(body).unwrap();

        assert_eq!(balances.len(), 2);
        assert_eq!(balances[1].currency, "BTC");
        assert_eq!(balances[1].amount, dec!(0.00000001));
        assert_eq!(balances[1].available, dec!(0));
        assert_eq!(balances[1].rate, Some(dec!(859614.5)));
    }

    #[test]
    fn a_missing_or_zero_rate_is_none() {
        let body = r#"{"status":0,"data":[
            {"amount":"1","available":"1","conversionRate":"0","symbol":"AAA"},
            {"amount":"1","available":"1","symbol":"BBB"}]}"#;

        let balances = parse_assets(body).unwrap();

        assert_eq!(balances[0].rate, None);
        assert_eq!(balances[1].rate, None);
    }

    #[test]
    fn parse_assets_reports_a_failure_with_a_bounded_message() {
        let long = "x".repeat(500);
        let body = format!(
            r#"{{"status":1,"messages":[{{"message_code":"ERR-5012","message_string":"{long}"}}]}}"#
        );

        let Err(Error::Http { message, .. }) = parse_assets(&body) else {
            panic!("expected an error");
        };

        assert!(message.starts_with("ERR-5012 x"));
        assert_eq!(message.len(), 200);
    }

    #[test]
    fn parse_assets_rejects_unsafe_symbols_negative_amounts_and_bad_shapes() {
        let asset = |symbol: &str, amount: &str, rate: &str| {
            format!(
                r#"{{"status":0,"data":[{{"amount":"{amount}","available":"0","conversionRate":"{rate}","symbol":"{symbol}"}}]}}"#
            )
        };
        for body in [
            asset("btc", "1", "1"),
            asset("BT C", "1", "1"),
            asset("", "1", "1"),
            asset("BTC", "-1", "1"),
            asset("BTC", "abc", "1"),
            asset("BTC", "1", "-5"),
            asset("BTC", "1", "x"),
            r#"{"status":0}"#.to_owned(),
            r#"{"data":[]}"#.to_owned(),
            "[1]".to_owned(),
        ] {
            assert!(
                matches!(parse_assets(&body), Err(Error::Format(_))),
                "{body}"
            );
        }
    }

    #[test]
    fn parse_error_message_strips_control_characters_and_other_fields() {
        let body = r#"{"status":1,"messages":[{"message_code":"ERR-1","message_string":"a\nb\u001b[31m"}],"data":"secret"}"#;

        assert_eq!(parse_error_message(body), "ERR-1 ab[31m");
        assert_eq!(parse_error_message("<html>oops</html>"), "no message");
        assert_eq!(parse_error_message(r#"{"status":1}"#), "no message");
    }
}
