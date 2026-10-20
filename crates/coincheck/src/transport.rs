use std::future::Future;
use std::time::Duration;

use crate::Error;

pub const BASE_URL: &str = "https://coincheck.com";
const TIMEOUT: Duration = Duration::from_secs(15);
const MAX_BODY_BYTES: usize = 1024 * 1024;

/// A `GET` request. The transport owns the scheme and host, so a caller can
/// only choose the path and headers, never where the request goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// Path and query string, starting with `/`.
    pub path_and_query: String,
    pub headers: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    pub status: u16,
    pub body: Vec<u8>,
}

pub trait Transport {
    fn get(&self, request: Request) -> impl Future<Output = Result<Response, Error>> + Send;
}

/// The real transport: HTTPS only, no redirects (so credentials in headers
/// cannot be forwarded to another host), a timeout, and a response size limit.
pub struct ReqwestTransport {
    client: reqwest::Client,
}

impl ReqwestTransport {
    pub fn new() -> Result<Self, Error> {
        let client = reqwest::Client::builder()
            .https_only(true)
            .redirect(reqwest::redirect::Policy::none())
            .timeout(TIMEOUT)
            .user_agent(concat!("tauri-robo-advisor/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| Error::Network(e.without_url().to_string()))?;
        Ok(Self { client })
    }
}

impl Transport for ReqwestTransport {
    async fn get(&self, request: Request) -> Result<Response, Error> {
        let mut builder = self
            .client
            .get(format!("{BASE_URL}{}", request.path_and_query));
        for (name, value) in &request.headers {
            builder = builder.header(name, value);
        }
        // `without_url` keeps the query string out of error messages.
        let mut response = builder
            .send()
            .await
            .map_err(|e| Error::Network(e.without_url().to_string()))?;
        let status = response.status().as_u16();

        let mut body = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|e| Error::Network(e.without_url().to_string()))?
        {
            if body.len() + chunk.len() > MAX_BODY_BYTES {
                return Err(Error::ResponseTooLarge);
            }
            body.extend_from_slice(&chunk);
        }
        Ok(Response { status, body })
    }
}
