use std::time::{SystemTime, UNIX_EPOCH};

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::Error;
use crate::credentials::Credentials;
use crate::model::{Balance, parse_assets, parse_error_message};
use crate::sign::sign;
use crate::transport::{Request, Response, Transport};

/// The path as it is signed (version first, no `/private`) and as it is requested.
const ASSETS_SIGN_PATH: &str = "/v1/account/assets";
const ASSETS_PATH: &str = "/private/v1/account/assets";
const JPY: &str = "JPY";

/// Assets GMO Coin handles only on the exchange (取引所), with no sales desk (販売所) price.
/// For these, the documented `conversionRate` is the last executed price on the exchange
/// instead of the sales desk's selling price. The response does not say which basis an
/// asset used, so the list comes from the API documentation (the assets table, checked
/// on 2026-10-20) and is kept here in one place.
const EXCHANGE_ONLY: &[&str] = &["FCR", "ASTR", "NAC", "WILD"];

/// Which price a value was worked out with. It is carried to the screen so the two bases
/// are never mixed without saying so.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PriceBasis {
    /// The sales desk's price for selling (what the person gets by selling there).
    Sales,
    /// The exchange's last executed price: the asset has no sales desk price.
    Board,
}

/// One balance valued in JPY: the currency, the value in JPY, and the amount held.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Holding {
    pub symbol: String,
    pub value: Decimal,
    /// How much of the currency is held (kept so an import can record the quantity).
    pub amount: Decimal,
    /// The yen price the value was worked out with, so an import can record the
    /// price next to the quantity. `None` for yen itself.
    #[serde(default)]
    pub price: Option<Decimal>,
    /// Which price `price` is. `None` for yen itself.
    #[serde(default)]
    pub basis: Option<PriceBasis>,
}

/// Balances valued in JPY.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Holdings {
    pub holdings: Vec<Holding>,
    /// Currencies with a balance but no yen rate, so they could not be
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
            clock: unix_millis,
        }
    }

    /// Uses a fake clock, so tests do not depend on the time.
    #[cfg(test)]
    fn with_clock(mut self, clock: fn() -> u64) -> Self {
        self.clock = clock;
        self
    }

    /// Fetches the balances (`GET /private/v1/account/assets`). The API key only
    /// needs the permission to read the asset balance.
    pub async fn balances(&self) -> Result<Vec<Balance>, Error> {
        let response = self.transport.get(self.signed()).await?;
        parse_assets(&body_text(response)?)
    }

    /// Fetches the balances and values each one in JPY with the `conversionRate` of the same
    /// response (the sales desk's selling price, the price the person trades at). Zero
    /// balances are skipped; an asset without a rate is reported in `unpriced`. Assets that
    /// have no sales desk price are valued at the exchange's last price and marked
    /// [`PriceBasis::Board`].
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
                    basis: None,
                });
                continue;
            }
            let Some(price) = balance.rate else {
                unpriced.push(balance.currency);
                continue;
            };
            let basis = if EXCHANGE_ONLY.contains(&balance.currency.as_str()) {
                PriceBasis::Board
            } else {
                PriceBasis::Sales
            };
            holdings.push(Holding {
                value: balance.amount.checked_mul(price).ok_or(Error::Overflow)?,
                amount: balance.amount,
                price: Some(price),
                basis: Some(basis),
                symbol: balance.currency,
            });
        }
        holdings.sort_by(|a, b| a.symbol.cmp(&b.symbol));
        unpriced.sort();
        Ok(Holdings { holdings, unpriced })
    }

    /// GMO Coin only checks that the timestamp is close to its own clock, so it needs no
    /// counter across requests.
    fn signed(&self) -> Request {
        let timestamp = (self.clock)().to_string();
        let signature = sign(
            self.credentials.api_secret(),
            &timestamp,
            "GET",
            ASSETS_SIGN_PATH,
            "",
        );
        Request {
            path_and_query: ASSETS_PATH.to_owned(),
            headers: vec![
                ("API-KEY".to_owned(), self.credentials.api_key().to_owned()),
                ("API-TIMESTAMP".to_owned(), timestamp),
                ("API-SIGN".to_owned(), signature),
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

    /// Answers every request with one canned response, and records the requests.
    struct FakeTransport {
        response: Option<(u16, String)>,
        seen: Mutex<Vec<Request>>,
    }

    impl FakeTransport {
        fn new(status: u16, body: &str) -> Self {
            Self {
                response: Some((status, body.to_owned())),
                seen: Mutex::new(Vec::new()),
            }
        }

        fn failing() -> Self {
            Self {
                response: None,
                seen: Mutex::new(Vec::new()),
            }
        }
    }

    impl Transport for &FakeTransport {
        fn get(&self, request: Request) -> impl Future<Output = Result<Response, Error>> + Send {
            let result = match &self.response {
                Some((status, body)) if request.path_and_query == ASSETS_PATH => Ok(Response {
                    status: *status,
                    body: body.clone().into_bytes(),
                }),
                _ => Err(Error::Network(format!(
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

    const ASSETS: &str = r#"{"status":0,"data":[
        {"amount":"100000","available":"100000","conversionRate":"1","symbol":"JPY"},
        {"amount":"0.5","available":"0.4","conversionRate":"8000000","symbol":"BTC"},
        {"amount":"0","available":"0","conversionRate":"300000","symbol":"ETH"},
        {"amount":"1000","available":"1000","conversionRate":"0.9","symbol":"ASTR"},
        {"amount":"7","available":"7","conversionRate":"0","symbol":"XYZ"}]}"#;

    fn header<'a>(request: &'a Request, name: &str) -> Option<&'a str> {
        request
            .headers
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }

    #[test]
    fn balances_sends_a_signed_get_with_the_expected_headers() {
        let fake = FakeTransport::new(200, ASSETS);

        let balances = block_on(client(&fake).balances()).unwrap();

        assert_eq!(balances.len(), 5);
        let seen = fake.seen.lock().unwrap();
        assert_eq!(seen.len(), 1);
        let request = &seen[0];
        // The URL path has /private, the signed path does not.
        assert_eq!(request.path_and_query, "/private/v1/account/assets");
        assert_eq!(header(request, "API-KEY"), Some("test-key"));
        assert_eq!(header(request, "API-TIMESTAMP"), Some("1700000000000"));
        assert_eq!(
            header(request, "API-SIGN"),
            Some(
                sign(
                    "test-secret",
                    "1700000000000",
                    "GET",
                    "/v1/account/assets",
                    ""
                )
                .as_str()
            )
        );
    }

    #[test]
    fn holdings_values_with_the_sales_rate_and_marks_the_basis() {
        let fake = FakeTransport::new(200, ASSETS);

        let result = block_on(client(&fake).holdings()).unwrap();

        assert_eq!(
            result.holdings,
            vec![
                Holding {
                    symbol: "ASTR".to_owned(),
                    value: dec!(900),
                    amount: dec!(1000),
                    price: Some(dec!(0.9)),
                    basis: Some(PriceBasis::Board),
                },
                Holding {
                    symbol: "BTC".to_owned(),
                    value: dec!(4000000),
                    amount: dec!(0.5),
                    price: Some(dec!(8000000)),
                    basis: Some(PriceBasis::Sales),
                },
                Holding {
                    symbol: "JPY".to_owned(),
                    value: dec!(100000),
                    amount: dec!(100000),
                    price: None,
                    basis: None,
                },
            ]
        );
        // The zero ETH balance is skipped; a zero rate is reported, not valued.
        assert_eq!(result.unpriced, vec!["XYZ".to_owned()]);
    }

    #[test]
    fn only_one_signed_get_is_sent_and_no_other_host_path_is_used() {
        let fake = FakeTransport::new(200, ASSETS);

        block_on(client(&fake).holdings()).unwrap();

        let seen = fake.seen.lock().unwrap();
        assert_eq!(seen.len(), 1);
        assert!(seen[0].path_and_query.starts_with("/private/v1/"));
    }

    #[test]
    fn a_status_error_in_a_200_response_carries_a_bounded_message_and_no_credentials() {
        let fake = FakeTransport::new(
            200,
            r#"{"status":1,"messages":[{"message_code":"ERR-5012","message_string":"Invalid API-KEY"}]}"#,
        );

        let error = block_on(client(&fake).balances()).unwrap_err();

        let shown = error.to_string();
        assert!(shown.contains("ERR-5012"));
        assert!(!shown.contains("test-key"));
        assert!(!shown.contains("test-secret"));
    }

    #[test]
    fn an_http_error_status_is_reported() {
        let fake = FakeTransport::new(503, "<html>down</html>");

        let Err(Error::Http { status, message }) = block_on(client(&fake).balances()) else {
            panic!("expected an HTTP error");
        };

        assert_eq!(status, 503);
        assert_eq!(message, "no message");
    }

    #[test]
    fn a_network_error_stops_the_import() {
        let fake = FakeTransport::failing();

        assert!(matches!(
            block_on(client(&fake).holdings()),
            Err(Error::Network(_))
        ));
    }
}
