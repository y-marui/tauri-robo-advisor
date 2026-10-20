use std::time::{SystemTime, UNIX_EPOCH};

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::Error;
use crate::credentials::Credentials;
use crate::model::{
    Balance, parse_balances, parse_error_message, parse_last_price, validate_currency_code,
};
use crate::page_prices::PagePrices;
use crate::sign::sign;
use crate::transport::{Request, Response, Transport};

const BALANCE_PATH: &str = "/v1/me/getbalance";
const JPY: &str = "JPY";

/// One balance valued in JPY: the currency, the value in JPY, and the amount held.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Holding {
    pub symbol: String,
    pub value: Decimal,
    /// How much of the currency is held (kept so an import can record the quantity).
    pub amount: Decimal,
    /// The yen price the value was worked out with, when the prices came from the price
    /// page (so an import can record the price next to the quantity).
    #[serde(default)]
    pub price: Option<Decimal>,
    /// The Lightning last price, read for comparison only (a currency with a Lightning market).
    #[serde(default)]
    pub api_price: Option<Decimal>,
    /// (page price - API price) / API price, in percent, rounded to 2 places. For reference:
    /// the two prices differ in meaning and timing, so a difference is not an error.
    #[serde(default)]
    pub difference_percent: Option<Decimal>,
}

/// Balances valued in JPY.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Holdings {
    pub holdings: Vec<Holding>,
    /// Currencies with a balance but no JPY market, so they could not be
    /// valued. They are reported instead of silently dropped, because leaving
    /// them out would misstate the portfolio.
    pub unpriced: Vec<String>,
}

pub struct Client<T> {
    transport: T,
    credentials: Credentials,
    clock: fn() -> u64,
}

impl<T: Transport> Client<T> {
    pub fn new(transport: T, credentials: Credentials) -> Self {
        Self {
            transport,
            credentials,
            clock: unix_seconds,
        }
    }

    #[cfg(test)]
    fn with_clock(mut self, clock: fn() -> u64) -> Self {
        self.clock = clock;
        self
    }

    /// Fetches balances (`GET /v1/me/getbalance`). The API key only needs the
    /// "get balances" permission.
    pub async fn balances(&self) -> Result<Vec<Balance>, Error> {
        let response = self.transport.get(self.signed(BALANCE_PATH)).await?;
        parse_balances(&body_text(response)?)
    }

    /// Fetches balances and values each one in JPY with the public ticker.
    /// Zero balances are skipped.
    pub async fn holdings(&self) -> Result<Holdings, Error> {
        let mut holdings = Vec::new();
        let mut unpriced = Vec::new();
        for balance in self.balances().await? {
            if balance.amount.is_zero() {
                continue;
            }
            let value = if balance.currency_code == JPY {
                balance.amount
            } else {
                match self.last_price_jpy(&balance.currency_code).await? {
                    Some(price) => balance.amount.checked_mul(price).ok_or(Error::Overflow)?,
                    None => {
                        unpriced.push(balance.currency_code);
                        continue;
                    }
                }
            };
            holdings.push(Holding {
                symbol: balance.currency_code,
                value,
                amount: balance.amount,
                price: None,
                api_price: None,
                difference_percent: None,
            });
        }
        holdings.sort_by(|a, b| a.symbol.cmp(&b.symbol));
        unpriced.sort();
        Ok(Holdings { holdings, unpriced })
    }

    /// Fetches balances and values each one with the price from the price page, one price
    /// basis for every currency. The Lightning last price is read too, only to show the
    /// difference: a failure to read it (or no market) leaves the comparison empty and never
    /// stops the import. A currency the page does not list is not valued with the API price
    /// (that would mix two bases); it is reported in `unpriced`. Zero balances are skipped.
    pub async fn holdings_with_page_prices(&self, page: &PagePrices) -> Result<Holdings, Error> {
        let mut holdings = Vec::new();
        let mut unpriced = Vec::new();
        for balance in self.balances().await? {
            if balance.amount.is_zero() {
                continue;
            }
            if balance.currency_code == JPY {
                holdings.push(Holding {
                    symbol: balance.currency_code,
                    value: balance.amount,
                    amount: balance.amount,
                    price: None,
                    api_price: None,
                    difference_percent: None,
                });
                continue;
            }
            let Some(price) = page.get(&balance.currency_code) else {
                unpriced.push(balance.currency_code);
                continue;
            };
            let value = balance.amount.checked_mul(price).ok_or(Error::Overflow)?;
            let api_price = self
                .last_price_jpy(&balance.currency_code)
                .await
                .ok()
                .flatten();
            holdings.push(Holding {
                value,
                amount: balance.amount,
                price: Some(price),
                api_price,
                difference_percent: api_price.and_then(|api| difference_percent(price, api)),
                symbol: balance.currency_code,
            });
        }
        holdings.sort_by(|a, b| a.symbol.cmp(&b.symbol));
        unpriced.sort();
        Ok(Holdings { holdings, unpriced })
    }

    /// The last traded price of `<currency>_JPY` from the public ticker, or
    /// `None` when bitFlyer has no such market (HTTP 400).
    async fn last_price_jpy(&self, currency: &str) -> Result<Option<Decimal>, Error> {
        validate_currency_code(currency)?;
        let request = Request {
            path_and_query: format!("/v1/ticker?product_code={currency}_{JPY}"),
            headers: Vec::new(),
        };
        let response = self.transport.get(request).await?;
        if response.status == 400 {
            return Ok(None);
        }
        parse_last_price(&body_text(response)?).map(Some)
    }

    fn signed(&self, path_and_query: &str) -> Request {
        let timestamp = (self.clock)().to_string();
        let signature = sign(
            self.credentials.api_secret(),
            &timestamp,
            "GET",
            path_and_query,
            "",
        );
        Request {
            path_and_query: path_and_query.to_owned(),
            headers: vec![
                (
                    "ACCESS-KEY".to_owned(),
                    self.credentials.api_key().to_owned(),
                ),
                ("ACCESS-TIMESTAMP".to_owned(), timestamp),
                ("ACCESS-SIGN".to_owned(), signature),
            ],
        }
    }
}

fn difference_percent(price: Decimal, api: Decimal) -> Option<Decimal> {
    let ratio = price.checked_sub(api)?.checked_div(api)?;
    Some(ratio.checked_mul(Decimal::ONE_HUNDRED)?.round_dp(2))
}

fn body_text(response: Response) -> Result<String, Error> {
    let text = String::from_utf8(response.body)
        .map_err(|_| Error::Format("the response is not valid UTF-8".to_owned()))?;
    if (200..300).contains(&response.status) {
        Ok(text)
    } else {
        Err(Error::Http {
            status: response.status,
            message: parse_error_message(&text),
        })
    }
}

fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::future::{Future, ready};
    use std::pin::pin;
    use std::sync::Mutex;
    use std::task::{Context, Poll, Waker};

    use rust_decimal::dec;

    use super::*;

    fn block_on<F: Future>(future: F) -> F::Output {
        let mut future = pin!(future);
        let mut context = Context::from_waker(Waker::noop());
        loop {
            if let Poll::Ready(value) = future.as_mut().poll(&mut context) {
                return value;
            }
        }
    }

    /// Answers by exact path and query, and records every request it receives.
    struct FakeTransport {
        responses: HashMap<String, (u16, String)>,
        seen: Mutex<Vec<Request>>,
    }

    impl FakeTransport {
        fn new(responses: &[(&str, u16, &str)]) -> Self {
            Self {
                responses: responses
                    .iter()
                    .map(|(path, status, body)| ((*path).to_owned(), (*status, (*body).to_owned())))
                    .collect(),
                seen: Mutex::new(Vec::new()),
            }
        }
    }

    impl Transport for &FakeTransport {
        fn get(&self, request: Request) -> impl Future<Output = Result<Response, Error>> + Send {
            let result = match self.responses.get(&request.path_and_query) {
                Some((status, body)) => Ok(Response {
                    status: *status,
                    body: body.clone().into_bytes(),
                }),
                None => Err(Error::Network(format!(
                    "unexpected {}",
                    request.path_and_query
                ))),
            };
            self.seen.lock().unwrap().push(request);
            ready(result)
        }
    }

    fn client(fake: &FakeTransport) -> Client<&FakeTransport> {
        Client::new(fake, Credentials::new("test-key", "test-secret")).with_clock(|| 1_700_000_000)
    }

    const BALANCES: &str = r#"[
        {"currency_code":"JPY","amount":100000,"available":90000},
        {"currency_code":"BTC","amount":0.5,"available":0.5},
        {"currency_code":"ETH","amount":0,"available":0},
        {"currency_code":"XYM","amount":10,"available":10}
    ]"#;

    #[test]
    fn balances_sends_a_signed_get_with_the_expected_headers() {
        let fake = FakeTransport::new(&[("/v1/me/getbalance", 200, BALANCES)]);

        let balances = block_on(client(&fake).balances()).unwrap();

        assert_eq!(balances.len(), 4);
        let seen = fake.seen.lock().unwrap();
        let request = &seen[0];
        let header = |name: &str| {
            request
                .headers
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, v)| v.as_str())
        };
        assert_eq!(header("ACCESS-KEY"), Some("test-key"));
        assert_eq!(header("ACCESS-TIMESTAMP"), Some("1700000000"));
        assert_eq!(
            header("ACCESS-SIGN"),
            Some(sign("test-secret", "1700000000", "GET", "/v1/me/getbalance", "").as_str())
        );
    }

    #[test]
    fn holdings_values_balances_in_jpy_and_skips_zero_balances() {
        let fake = FakeTransport::new(&[
            ("/v1/me/getbalance", 200, BALANCES),
            ("/v1/ticker?product_code=BTC_JPY", 200, r#"{"ltp":4000000}"#),
            (
                "/v1/ticker?product_code=XYM_JPY",
                400,
                r#"{"error_message":"Invalid product"}"#,
            ),
        ]);

        let result = block_on(client(&fake).holdings()).unwrap();

        assert_eq!(
            result.holdings,
            vec![
                Holding {
                    symbol: "BTC".to_owned(),
                    value: dec!(2000000),
                    amount: dec!(0.5),
                    price: None,
                    api_price: None,
                    difference_percent: None,
                },
                Holding {
                    symbol: "JPY".to_owned(),
                    value: dec!(100000),
                    amount: dec!(100000),
                    price: None,
                    api_price: None,
                    difference_percent: None,
                },
            ]
        );
        assert_eq!(result.unpriced, vec!["XYM".to_owned()]);
    }

    fn page(rows: &[(&str, &str)]) -> PagePrices {
        let mut table = vec![vec!["通貨ペア".to_owned(), "価格".to_owned()]];
        table.extend(
            rows.iter()
                .map(|(pair, price)| vec![(*pair).to_owned(), format!("{price} 円")]),
        );
        PagePrices::from_table(&table).unwrap()
    }

    #[test]
    fn page_prices_value_every_currency_on_one_basis_and_compare_with_the_api() {
        // XYM has no Lightning market but is on the page; BTC is on both.
        let fake = FakeTransport::new(&[
            ("/v1/me/getbalance", 200, BALANCES),
            ("/v1/ticker?product_code=BTC_JPY", 200, r#"{"ltp":4000000}"#),
            ("/v1/ticker?product_code=XYM_JPY", 400, "{}"),
        ]);
        let prices = page(&[("BTC/JPY", "4,040,000"), ("XYM/JPY", "10.5")]);

        let result = block_on(client(&fake).holdings_with_page_prices(&prices)).unwrap();

        let by = |s: &str| result.holdings.iter().find(|h| h.symbol == s).unwrap();
        assert_eq!(by("BTC").value, dec!(2020000));
        assert_eq!(by("BTC").price, Some(dec!(4040000)));
        assert_eq!(by("BTC").api_price, Some(dec!(4000000)));
        assert_eq!(by("BTC").difference_percent, Some(dec!(1)));
        assert_eq!(by("XYM").value, dec!(105));
        assert_eq!(by("XYM").api_price, None);
        assert_eq!(by("XYM").difference_percent, None);
        assert_eq!(by("JPY").value, dec!(100000));
        assert!(result.unpriced.is_empty());
    }

    #[test]
    fn a_currency_missing_from_the_page_is_unpriced_even_with_a_lightning_market() {
        let fake = FakeTransport::new(&[("/v1/me/getbalance", 200, BALANCES)]);
        let prices = page(&[("XYM/JPY", "10")]);

        let result = block_on(client(&fake).holdings_with_page_prices(&prices)).unwrap();

        assert_eq!(result.unpriced, vec!["BTC".to_owned()]);
        assert!(result.holdings.iter().all(|h| h.symbol != "BTC"));
        // the API price is not even requested for it
        assert!(
            fake.seen
                .lock()
                .unwrap()
                .iter()
                .all(|r| !r.path_and_query.contains("BTC_JPY"))
        );
    }

    #[test]
    fn a_failing_comparison_price_does_not_stop_the_import() {
        let fake = FakeTransport::new(&[
            (
                "/v1/me/getbalance",
                200,
                r#"[{"currency_code":"BTC","amount":2,"available":2}]"#,
            ),
            ("/v1/ticker?product_code=BTC_JPY", 500, "oops"),
        ]);
        let prices = page(&[("BTC/JPY", "100")]);

        let result = block_on(client(&fake).holdings_with_page_prices(&prices)).unwrap();

        assert_eq!(result.holdings[0].value, dec!(200));
        assert_eq!(result.holdings[0].api_price, None);
    }

    #[test]
    fn holdings_never_sends_credentials_to_the_public_ticker() {
        let fake = FakeTransport::new(&[
            (
                "/v1/me/getbalance",
                200,
                r#"[{"currency_code":"BTC","amount":1,"available":1}]"#,
            ),
            ("/v1/ticker?product_code=BTC_JPY", 200, r#"{"ltp":1}"#),
        ]);

        block_on(client(&fake).holdings()).unwrap();

        let seen = fake.seen.lock().unwrap();
        let ticker = seen
            .iter()
            .find(|r| r.path_and_query.starts_with("/v1/ticker"))
            .unwrap();
        assert!(ticker.headers.is_empty());
    }

    #[test]
    fn holdings_only_issues_get_requests_to_known_paths() {
        let fake = FakeTransport::new(&[
            ("/v1/me/getbalance", 200, BALANCES),
            ("/v1/ticker?product_code=BTC_JPY", 200, r#"{"ltp":1}"#),
            ("/v1/ticker?product_code=XYM_JPY", 400, "{}"),
        ]);

        block_on(client(&fake).holdings()).unwrap();

        for request in fake.seen.lock().unwrap().iter() {
            assert!(
                request.path_and_query == "/v1/me/getbalance"
                    || request
                        .path_and_query
                        .starts_with("/v1/ticker?product_code="),
                "{}",
                request.path_and_query
            );
        }
    }

    #[test]
    fn balances_reports_http_errors_without_echoing_the_body() {
        let body = r#"{"status":-2,"error_message":"Insufficient permission","data":"leak"}"#;
        let fake = FakeTransport::new(&[("/v1/me/getbalance", 403, body)]);

        let error = block_on(client(&fake).balances()).unwrap_err();

        assert_eq!(
            error.to_string(),
            "bitFlyer returned HTTP 403: Insufficient permission"
        );
    }

    #[test]
    fn errors_never_contain_the_credentials() {
        let fake = FakeTransport::new(&[("/v1/me/getbalance", 401, r#"{"error_message":"bad"}"#)]);

        let shown = format!(
            "{:?} {}",
            client(&fake).credentials,
            block_on(client(&fake).balances()).unwrap_err()
        );

        assert!(!shown.contains("test-key"));
        assert!(!shown.contains("test-secret"));
    }

    #[test]
    fn holdings_returns_overflow_for_astronomical_values() {
        let fake = FakeTransport::new(&[
            (
                "/v1/me/getbalance",
                200,
                r#"[{"currency_code":"BTC","amount":79228162514264337593543950335,"available":0}]"#,
            ),
            ("/v1/ticker?product_code=BTC_JPY", 200, r#"{"ltp":10}"#),
        ]);

        let result = block_on(client(&fake).holdings());

        assert!(matches!(result, Err(Error::Overflow)));
    }
}
