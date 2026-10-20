//! Read-only client for the Binance API.
//!
//! The crate signs one request (`GET /api/v3/account`) and reads one public
//! one (`GET /api/v3/ticker/price`), then converts the balances into JPY
//! [`Holding`]s (value, amount, price and the basis of the price). It is
//! independent of Tauri; the HTTP transport is a trait so the logic can be
//! tested without the network, and secrets never leave [`Credentials`].
//!
//! The host is fixed to `api.binance.com` (the global API). That a Binance Japan
//! key works there is the user's assumption, not something Binance documents; a
//! key it rejects is reported with Binance's own error text.
//!
//! Only `GET` requests are built, and only against the fixed `https` base URL,
//! so no code path can place an order or move funds.

mod client;
mod credentials;
mod error;
mod model;
mod sign;
mod transport;

pub use client::{Client, Holding, Holdings, PriceBasis};
pub use credentials::Credentials;
pub use error::Error;
pub use model::Balance;
pub use sign::sign;
pub use transport::{BASE_URL, Request, ReqwestTransport, Response, Transport};
