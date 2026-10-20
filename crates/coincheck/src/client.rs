use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::Error;
use crate::credentials::Credentials;
use crate::model::{
    Balance, is_failure, parse_balances, parse_error_message, parse_last_price,
    validate_currency_code,
};
use crate::sign::sign;
use crate::transport::{BASE_URL, Request, Response, Transport};

const BALANCE_PATH: &str = "/api/accounts/balance";
const JPY: &str = "JPY";

/// One balance valued in JPY: the currency, the value in JPY, and the amount held.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Holding {
    pub symbol: String,
    pub value: Decimal,
    /// How much of the currency is held (kept so an import can record the quantity).
    pub amount: Decimal,
    /// The yen price (the ticker's last price) the value was worked out with, so an import
    /// can record the price next to the quantity. `None` for yen itself.
    #[serde(default)]
    pub price: Option<Decimal>,
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
    last_nonce: Mutex<u64>,
}

impl<T: Transport> Client<T> {
    pub fn new(transport: T, credentials: Credentials) -> Self {
        Self {
            transport,
            credentials,
            clock: unix_millis,
            last_nonce: Mutex::new(0),
        }
    }

    #[cfg(test)]
    fn with_clock(mut self, clock: fn() -> u64) -> Self {
        self.clock = clock;
        self
    }

    /// Fetches balances (`GET /api/accounts/balance`). The API key only needs
    /// the permission to read balances.
    pub async fn balances(&self) -> Result<Vec<Balance>, Error> {
        let response = self.transport.get(self.signed(BALANCE_PATH)).await?;
        parse_balances(&body_text(response)?)
    }

    /// Fetches balances and values each one in JPY with the public ticker
    /// (the last price, one basis for every currency). Zero balances are
    /// skipped; a currency with no JPY pair is reported in `unpriced`.
    pub async fn holdings(&self) -> Result<Holdings, Error> {
        let mut holdings = Vec::new();
        let mut unpriced = Vec::new();
        for balance in self.balances().await? {
            if balance.amount.is_zero() {
                continue;
            }
            if balance.currency == JPY {
                holdings.push(Holding {
                    symbol: balance.currency,
                    value: balance.amount,
                    amount: balance.amount,
                    price: None,
                });
                continue;
            }
            let Some(price) = self.last_price_jpy(&balance.currency).await? else {
                unpriced.push(balance.currency);
                continue;
            };
            holdings.push(Holding {
                value: balance.amount.checked_mul(price).ok_or(Error::Overflow)?,
                amount: balance.amount,
                price: Some(price),
                symbol: balance.currency,
            });
        }
        holdings.sort_by(|a, b| a.symbol.cmp(&b.symbol));
        unpriced.sort();
        Ok(Holdings { holdings, unpriced })
    }

    /// The last traded price of `<currency>_jpy` from the public ticker, or
    /// `None` when Coincheck has no such pair.
    async fn last_price_jpy(&self, currency: &str) -> Result<Option<Decimal>, Error> {
        let code = currency.to_ascii_lowercase();
        validate_currency_code(&code)?;
        let pair = format!("{code}_jpy");
        let request = Request {
            path_and_query: format!("/api/ticker?pair={pair}"),
            headers: Vec::new(),
        };
        let response = self.transport.get(request).await?;
        if matches!(response.status, 400 | 404) {
            return Ok(None);
        }
        let text = body_text(response)?;
        if is_failure(&text) {
            return Ok(None);
        }
        parse_last_price(&text).map(Some)
    }

    /// A nonce that is the time in milliseconds, and strictly greater than the
    /// previous one even when the clock repeats or goes back.
    fn next_nonce(&self) -> u64 {
        let now = (self.clock)();
        let mut last = self
            .last_nonce
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *last = now.max(last.saturating_add(1));
        *last
    }

    fn signed(&self, path_and_query: &str) -> Request {
        let nonce = self.next_nonce().to_string();
        let url = format!("{BASE_URL}{path_and_query}");
        let signature = sign(self.credentials.api_secret(), &nonce, &url, "");
        Request {
            path_and_query: path_and_query.to_owned(),
            headers: vec![
                (
                    "ACCESS-KEY".to_owned(),
                    self.credentials.api_key().to_owned(),
                ),
                ("ACCESS-NONCE".to_owned(), nonce),
                ("ACCESS-SIGNATURE".to_owned(), signature),
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

fn unix_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
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
        Client::new(fake, Credentials::new("test-key", "test-secret"))
            .with_clock(|| 1_700_000_000_000)
    }

    const BALANCES: &str = r#"{"success":true,
        "jpy":"100000","jpy_reserved":"0",
        "btc":"0.4","btc_reserved":"0.1",
        "eth":"0","eth_reserved":"0",
        "xym":"10"}"#;

    fn header<'a>(request: &'a Request, name: &str) -> Option<&'a str> {
        request
            .headers
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }

    #[test]
    fn balances_sends_a_signed_get_with_the_expected_headers() {
        let fake = FakeTransport::new(&[("/api/accounts/balance", 200, BALANCES)]);

        let balances = block_on(client(&fake).balances()).unwrap();

        assert_eq!(balances.len(), 4);
        let seen = fake.seen.lock().unwrap();
        let request = &seen[0];
        assert_eq!(header(request, "ACCESS-KEY"), Some("test-key"));
        assert_eq!(header(request, "ACCESS-NONCE"), Some("1700000000000"));
        assert_eq!(
            header(request, "ACCESS-SIGNATURE"),
            Some(
                sign(
                    "test-secret",
                    "1700000000000",
                    "https://coincheck.com/api/accounts/balance",
                    ""
                )
                .as_str()
            )
        );
    }

    #[test]
    fn nonces_increase_even_when_the_clock_repeats_or_goes_back() {
        let fake = FakeTransport::new(&[]);
        let client = client(&fake);

        let first = client.next_nonce();
        let second = client.next_nonce();
        let third = client.next_nonce();

        assert_eq!(first, 1_700_000_000_000);
        assert_eq!(second, first + 1);
        assert_eq!(third, second + 1);
    }

    #[test]
    fn holdings_values_balances_in_jpy_and_skips_zero_balances() {
        let fake = FakeTransport::new(&[
            ("/api/accounts/balance", 200, BALANCES),
            ("/api/ticker?pair=btc_jpy", 200, r#"{"last":4000000}"#),
            (
                "/api/ticker?pair=xym_jpy",
                200,
                r#"{"success":false,"error":"Pair is not supported"}"#,
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
                    price: Some(dec!(4000000)),
                },
                Holding {
                    symbol: "JPY".to_owned(),
                    value: dec!(100000),
                    amount: dec!(100000),
                    price: None,
                },
            ]
        );
        assert_eq!(result.unpriced, vec!["XYM".to_owned()]);
        // The zero ETH balance is never priced.
        let seen = fake.seen.lock().unwrap();
        assert!(seen.iter().all(|r| !r.path_and_query.contains("eth")));
    }

    #[test]
    fn a_ticker_http_400_means_unpriced() {
        let fake = FakeTransport::new(&[
            (
                "/api/accounts/balance",
                200,
                r#"{"success":true,"xym":"1"}"#,
            ),
            ("/api/ticker?pair=xym_jpy", 400, "{}"),
        ]);

        let result = block_on(client(&fake).holdings()).unwrap();

        assert!(result.holdings.is_empty());
        assert_eq!(result.unpriced, vec!["XYM".to_owned()]);
    }

    #[test]
    fn only_the_balance_request_is_signed_and_every_request_is_a_get_path() {
        let fake = FakeTransport::new(&[
            ("/api/accounts/balance", 200, BALANCES),
            ("/api/ticker?pair=btc_jpy", 200, r#"{"last":1}"#),
            ("/api/ticker?pair=xym_jpy", 200, r#"{"last":1}"#),
        ]);

        block_on(client(&fake).holdings()).unwrap();

        let seen = fake.seen.lock().unwrap();
        for request in seen.iter() {
            assert!(request.path_and_query.starts_with("/api/"));
            let signed = request.path_and_query == "/api/accounts/balance";
            assert_eq!(header(request, "ACCESS-KEY").is_some(), signed);
        }
    }

    #[test]
    fn http_errors_carry_a_bounded_message_and_no_credentials() {
        let fake = FakeTransport::new(&[(
            "/api/accounts/balance",
            401,
            r#"{"success":false,"error":"invalid authentication"}"#,
        )]);

        let error = block_on(client(&fake).balances()).unwrap_err();

        let shown = error.to_string();
        assert!(shown.contains("401"));
        assert!(shown.contains("invalid authentication"));
        assert!(!shown.contains("test-key"));
        assert!(!shown.contains("test-secret"));
    }

    #[test]
    fn a_network_error_stops_the_import() {
        let fake = FakeTransport::new(&[]);

        assert!(matches!(
            block_on(client(&fake).holdings()),
            Err(Error::Network(_))
        ));
    }
}
