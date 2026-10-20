use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::Error;
use crate::credentials::Credentials;
use crate::model::{Balance, parse_account, parse_error_message, parse_prices};
use crate::sign::sign;
use crate::transport::{Request, Response, Transport};

const ACCOUNT_PATH: &str = "/api/v3/account";
const PRICES_PATH: &str = "/api/v3/ticker/price";
const JPY: &str = "JPY";
const USDT: &str = "USDT";
/// How far the request timestamp may differ from Binance's clock (milliseconds).
const RECV_WINDOW: u32 = 5000;

/// Which price a value was worked out with. It is carried to the screen so the bases
/// are never mixed without saying so. Binance's API has no sales desk (hanbaijo) rate,
/// so every price comes from the exchange order book's last trade.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PriceBasis {
    /// The last price of the asset's own JPY market (`<ASSET>JPY`).
    Board,
    /// The last price of the `<ASSET>USDT` market times the last price of `USDTJPY`.
    BoardViaUsdt,
}

/// One balance valued in JPY: the currency, the value in JPY, and the amount held.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Holding {
    pub symbol: String,
    pub value: Decimal,
    /// How much of the currency is held (kept so an import can record the quantity).
    pub amount: Decimal,
    /// The yen price the value was worked out with. `None` for yen itself.
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
    /// Currencies with a balance but no JPY (or USDT and `USDTJPY`) market, so they
    /// could not be valued. They are reported instead of silently dropped, because
    /// leaving them out would misstate the portfolio.
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

    /// Fetches the non-zero balances (`GET /api/v3/account`, `USER_DATA`). The API key
    /// only needs to be allowed to read; trading and withdrawal are not needed.
    pub async fn balances(&self) -> Result<Vec<Balance>, Error> {
        let response = self.transport.get(self.signed_account()).await?;
        parse_account(&body_text(response)?)
    }

    /// Fetches the last prices (`GET /api/v3/ticker/price`, public) of the JPY and
    /// USDT markets.
    async fn prices(&self) -> Result<HashMap<String, Decimal>, Error> {
        let response = self
            .transport
            .get(Request {
                path_and_query: PRICES_PATH.to_owned(),
                headers: Vec::new(),
            })
            .await?;
        parse_prices(&body_text(response)?)
    }

    /// Fetches the balances and values each one in JPY. Zero balances are skipped. An
    /// asset with its own JPY market uses that price ([`PriceBasis::Board`]); otherwise
    /// the USDT market times `USDTJPY` ([`PriceBasis::BoardViaUsdt`]); an asset with
    /// neither is reported in `unpriced`.
    pub async fn holdings(&self) -> Result<Holdings, Error> {
        let balances: Vec<Balance> = self
            .balances()
            .await?
            .into_iter()
            .filter(|b| !b.amount.is_zero())
            .collect();
        let prices = if balances.iter().any(|b| b.currency != JPY) {
            self.prices().await?
        } else {
            HashMap::new()
        };

        let mut holdings = Vec::new();
        let mut unpriced = Vec::new();
        for balance in balances {
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
            let Some((price, basis)) = price_in_jpy(&balance.currency, &prices)? else {
                unpriced.push(balance.currency);
                continue;
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

    /// The query is signed as sent (`totalParams`); the key goes in a header. Binance
    /// only checks that the timestamp is within `recvWindow` of its own clock.
    fn signed_account(&self) -> Request {
        let query = format!(
            "omitZeroBalances=true&recvWindow={RECV_WINDOW}&timestamp={}",
            (self.clock)()
        );
        let signature = sign(self.credentials.api_secret(), &query);
        Request {
            path_and_query: format!("{ACCOUNT_PATH}?{query}&signature={signature}"),
            headers: vec![(
                "X-MBX-APIKEY".to_owned(),
                self.credentials.api_key().to_owned(),
            )],
        }
    }
}

fn price_in_jpy(
    currency: &str,
    prices: &HashMap<String, Decimal>,
) -> Result<Option<(Decimal, PriceBasis)>, Error> {
    if let Some(price) = prices.get(&format!("{currency}{JPY}")) {
        return Ok(Some((*price, PriceBasis::Board)));
    }
    let (Some(in_usdt), Some(usdt_jpy)) = (
        prices.get(&format!("{currency}{USDT}")),
        prices.get(&format!("{USDT}{JPY}")),
    ) else {
        return Ok(None);
    };
    let price = in_usdt.checked_mul(*usdt_jpy).ok_or(Error::Overflow)?;
    Ok(Some((price, PriceBasis::BoardViaUsdt)))
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

    /// Answers the account and price requests with canned responses, and records the requests.
    struct FakeTransport {
        account: Option<(u16, String)>,
        prices: String,
        seen: Mutex<Vec<Request>>,
    }

    impl FakeTransport {
        fn new(account_status: u16, account: &str, prices: &str) -> Self {
            Self {
                account: Some((account_status, account.to_owned())),
                prices: prices.to_owned(),
                seen: Mutex::new(Vec::new()),
            }
        }

        fn failing() -> Self {
            Self {
                account: None,
                prices: "[]".to_owned(),
                seen: Mutex::new(Vec::new()),
            }
        }
    }

    impl Transport for &FakeTransport {
        fn get(&self, request: Request) -> impl Future<Output = Result<Response, Error>> + Send {
            let result = if request.path_and_query.starts_with(ACCOUNT_PATH) {
                match &self.account {
                    Some((status, body)) => Ok(Response {
                        status: *status,
                        body: body.clone().into_bytes(),
                    }),
                    None => Err(Error::Network("down".to_owned())),
                }
            } else if request.path_and_query == PRICES_PATH {
                Ok(Response {
                    status: 200,
                    body: self.prices.clone().into_bytes(),
                })
            } else {
                Err(Error::Network(format!(
                    "unexpected {}",
                    request.path_and_query
                )))
            };
            self.seen.lock().unwrap().push(request);
            ready(result)
        }
    }

    fn client(fake: &FakeTransport) -> Client<&FakeTransport> {
        Client::new(fake, Credentials::new("test-key", "test-secret"))
            .with_clock(|| 1_700_000_000_000)
    }

    const ACCOUNT: &str = r#"{"balances":[
        {"asset":"JPY","free":"100000","locked":"0"},
        {"asset":"BTC","free":"0.4","locked":"0.1"},
        {"asset":"ETH","free":"0","locked":"0"},
        {"asset":"SOL","free":"10","locked":"0"},
        {"asset":"XYZ","free":"7","locked":"0"}]}"#;

    const PRICES: &str = r#"[
        {"symbol":"BTCJPY","price":"8000000"},
        {"symbol":"BTCUSDT","price":"50000"},
        {"symbol":"SOLUSDT","price":"150"},
        {"symbol":"USDTJPY","price":"160"}]"#;

    #[test]
    fn balances_sends_a_signed_get_with_the_key_in_a_header() {
        let fake = FakeTransport::new(200, ACCOUNT, PRICES);

        let balances = block_on(client(&fake).balances()).unwrap();

        assert_eq!(balances.len(), 5);
        let seen = fake.seen.lock().unwrap();
        assert_eq!(seen.len(), 1);
        let request = &seen[0];
        let query = "omitZeroBalances=true&recvWindow=5000&timestamp=1700000000000";
        assert_eq!(
            request.path_and_query,
            format!(
                "/api/v3/account?{query}&signature={}",
                sign("test-secret", query)
            )
        );
        assert_eq!(
            request.headers,
            vec![("X-MBX-APIKEY".to_owned(), "test-key".to_owned())]
        );
        // The secret itself is never part of the request.
        assert!(!request.path_and_query.contains("test-secret"));
    }

    #[test]
    fn holdings_prefers_the_jpy_market_then_usdt_and_reports_the_rest() {
        let fake = FakeTransport::new(200, ACCOUNT, PRICES);

        let result = block_on(client(&fake).holdings()).unwrap();

        assert_eq!(
            result.holdings,
            vec![
                Holding {
                    symbol: "BTC".to_owned(),
                    value: dec!(4000000),
                    amount: dec!(0.5),
                    price: Some(dec!(8000000)),
                    basis: Some(PriceBasis::Board),
                },
                Holding {
                    symbol: "JPY".to_owned(),
                    value: dec!(100000),
                    amount: dec!(100000),
                    price: None,
                    basis: None,
                },
                Holding {
                    symbol: "SOL".to_owned(),
                    value: dec!(240000),
                    amount: dec!(10),
                    price: Some(dec!(24000)),
                    basis: Some(PriceBasis::BoardViaUsdt),
                },
            ]
        );
        // The zero ETH balance is skipped; XYZ has no market, so it is reported.
        assert_eq!(result.unpriced, vec!["XYZ".to_owned()]);
    }

    #[test]
    fn without_usdtjpy_a_usdt_priced_asset_is_unpriced() {
        let fake = FakeTransport::new(200, ACCOUNT, r#"[{"symbol":"SOLUSDT","price":"150"}]"#);

        let result = block_on(client(&fake).holdings()).unwrap();

        assert_eq!(result.unpriced, vec!["BTC", "SOL", "XYZ"]);
    }

    #[test]
    fn only_yen_needs_no_price_request() {
        let fake = FakeTransport::new(
            200,
            r#"{"balances":[{"asset":"JPY","free":"5","locked":"0"}]}"#,
            "[]",
        );

        block_on(client(&fake).holdings()).unwrap();

        assert_eq!(fake.seen.lock().unwrap().len(), 1);
    }

    #[test]
    fn only_get_requests_to_the_two_fixed_paths_are_sent() {
        let fake = FakeTransport::new(200, ACCOUNT, PRICES);

        block_on(client(&fake).holdings()).unwrap();

        let seen = fake.seen.lock().unwrap();
        assert_eq!(seen.len(), 2);
        assert!(seen[0].path_and_query.starts_with("/api/v3/account?"));
        assert_eq!(seen[1].path_and_query, "/api/v3/ticker/price");
        // The public request carries no key.
        assert!(seen[1].headers.is_empty());
    }

    #[test]
    fn a_rejected_key_carries_binances_message_and_no_credentials() {
        let fake = FakeTransport::new(
            401,
            r#"{"code":-2015,"msg":"Invalid API-key, IP, or permissions for action."}"#,
            "[]",
        );

        let error = block_on(client(&fake).balances()).unwrap_err();

        let shown = error.to_string();
        assert!(shown.contains("HTTP 401"));
        assert!(shown.contains("-2015 Invalid API-key"));
        assert!(!shown.contains("test-key"));
        assert!(!shown.contains("test-secret"));
    }

    #[test]
    fn an_http_error_without_json_is_reported() {
        let fake = FakeTransport::new(503, "<html>down</html>", "[]");

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
