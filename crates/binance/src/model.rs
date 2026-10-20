use std::collections::HashMap;
use std::str::FromStr;

use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json::value::RawValue;

use crate::Error;

/// One asset of `GET /api/v3/account`.
#[derive(Debug, Clone, PartialEq)]
pub struct Balance {
    /// Upper-case symbol, e.g. `BTC` or `JPY`.
    pub currency: String,
    /// Amount held: `free + locked` (locked is what open orders reserve).
    pub amount: Decimal,
}

#[derive(Deserialize)]
struct RawAccount<'a> {
    #[serde(borrow)]
    balances: Option<Vec<RawBalance<'a>>>,
}

#[derive(Deserialize)]
struct RawBalance<'a> {
    asset: String,
    #[serde(borrow)]
    free: &'a RawValue,
    #[serde(borrow)]
    locked: &'a RawValue,
}

#[derive(Deserialize)]
struct RawTicker<'a> {
    symbol: String,
    #[serde(borrow)]
    price: &'a RawValue,
}

/// Binance reports failures as `{"code": -2015, "msg": "..."}` with a non-2xx status.
#[derive(Deserialize)]
struct RawFailure {
    code: Option<i64>,
    msg: Option<String>,
}

/// Parses the account response; amounts are decimal strings and the raw text is
/// kept so nothing is rounded through `f64`.
pub(crate) fn parse_account(body: &str) -> Result<Vec<Balance>, Error> {
    let account: RawAccount =
        serde_json::from_str(body).map_err(|e| Error::Format(e.to_string()))?;
    let balances = account
        .balances
        .ok_or_else(|| Error::Format("the response has no balances".to_owned()))?;
    let mut result = Vec::new();
    for balance in balances {
        validate_symbol(&balance.asset)?;
        let amount = non_negative(parse_decimal(balance.free)?)?
            .checked_add(non_negative(parse_decimal(balance.locked)?)?)
            .ok_or(Error::Overflow)?;
        result.push(Balance {
            currency: balance.asset,
            amount,
        });
    }
    Ok(result)
}

/// Parses `GET /api/v3/ticker/price` into `symbol -> last price`, keeping only the
/// pairs the valuation can use: quoted in JPY or in USDT. A zero price is dropped.
pub(crate) fn parse_prices(body: &str) -> Result<HashMap<String, Decimal>, Error> {
    let tickers: Vec<RawTicker> =
        serde_json::from_str(body).map_err(|e| Error::Format(e.to_string()))?;
    let mut prices = HashMap::new();
    for ticker in tickers {
        if !(ticker.symbol.ends_with("JPY") || ticker.symbol.ends_with("USDT")) {
            continue;
        }
        let price = non_negative(parse_decimal(ticker.price)?)?;
        if price > Decimal::ZERO {
            prices.insert(ticker.symbol, price);
        }
    }
    Ok(prices)
}

/// Extracts `code msg` of an error body, bounded and stripped of control
/// characters; the rest of the body is never shown.
pub(crate) fn parse_error_message(body: &str) -> String {
    let text = serde_json::from_str::<RawFailure>(body)
        .ok()
        .map(|e| {
            format!(
                "{} {}",
                e.code.map(|c| c.to_string()).unwrap_or_default(),
                e.msg.unwrap_or_default()
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

/// A symbol is shown to the user and matched against names, so only a short
/// upper-case alphanumeric code is accepted.
fn validate_symbol(code: &str) -> Result<(), Error> {
    let valid = (1..=20).contains(&code.len())
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

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::dec;

    #[test]
    fn parse_account_adds_free_and_locked_as_decimals() {
        let body = r#"{"makerCommission":10,"canTrade":true,"balances":[
            {"asset":"BTC","free":"0.00000001","locked":"0.00000002"},
            {"asset":"1000SATS","free":"5","locked":"0"}]}"#;

        let balances = parse_account(body).unwrap();

        assert_eq!(balances.len(), 2);
        assert_eq!(balances[0].currency, "BTC");
        assert_eq!(balances[0].amount, dec!(0.00000003));
        assert_eq!(balances[1].currency, "1000SATS");
    }

    #[test]
    fn parse_account_rejects_unsafe_symbols_negative_amounts_and_bad_shapes() {
        let balance = |asset: &str, free: &str| {
            format!(r#"{{"balances":[{{"asset":"{asset}","free":"{free}","locked":"0"}}]}}"#)
        };
        for body in [
            balance("btc", "1"),
            balance("BT C", "1"),
            balance("", "1"),
            balance("BTC", "-1"),
            balance("BTC", "abc"),
            r#"{"canTrade":true}"#.to_owned(),
            "[1]".to_owned(),
        ] {
            assert!(
                matches!(parse_account(&body), Err(Error::Format(_))),
                "{body}"
            );
        }
    }

    #[test]
    fn parse_prices_keeps_only_jpy_and_usdt_pairs_with_a_price() {
        let body = r#"[
            {"symbol":"BTCJPY","price":"15000000.5"},
            {"symbol":"BTCUSDT","price":"100000"},
            {"symbol":"ETHBTC","price":"0.03"},
            {"symbol":"DEADJPY","price":"0.00000000"}]"#;

        let prices = parse_prices(body).unwrap();

        assert_eq!(prices.len(), 2);
        assert_eq!(prices["BTCJPY"], dec!(15000000.5));
        assert!(!prices.contains_key("DEADJPY"));
        assert!(matches!(parse_prices("{}"), Err(Error::Format(_))));
    }

    #[test]
    fn parse_error_message_is_bounded_and_strips_control_characters() {
        let body = r#"{"code":-2015,"msg":"Invalid API-key\n\u001b[31m"}"#;

        assert_eq!(parse_error_message(body), "-2015 Invalid API-key[31m");
        let long = format!(r#"{{"code":-1,"msg":"{}"}}"#, "x".repeat(500));
        assert_eq!(parse_error_message(&long).len(), 200);
        assert_eq!(parse_error_message("<html>oops</html>"), "no message");
        assert_eq!(parse_error_message("{}"), "no message");
    }
}
