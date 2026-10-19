use std::time::{SystemTime, UNIX_EPOCH};

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::Error;
use crate::credentials::Credentials;
use crate::model::{
    Balance, parse_balances, parse_error_message, parse_last_price, validate_currency_code,
};
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
                    amount: dec!(0.5)
                },
                Holding {
                    symbol: "JPY".to_owned(),
                    value: dec!(100000),
                    amount: dec!(100000)
                },
            ]
        );
        assert_eq!(result.unpriced, vec!["XYM".to_owned()]);
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
