//! Read-only client for the bitFlyer Lightning API.
//!
//! The crate signs requests, fetches balances and public tickers, and converts
//! the balances into JPY [`Holding`]s (value and amount). It is
//! independent of Tauri; the HTTP transport is a trait so the logic can be
//! tested without the network, and secrets never leave [`Credentials`].
//!
//! Only `GET` requests are built, and only against the fixed `https` base URL,
//! so no code path can place an order or move funds.

mod client;
mod credentials;
mod error;
mod model;
mod page_prices;
mod sign;
mod transport;

pub use client::{Client, Holding, Holdings};
pub use credentials::Credentials;
pub use error::Error;
pub use model::Balance;
pub use page_prices::PagePrices;
pub use sign::sign;
pub use transport::{Request, ReqwestTransport, Response, Transport};
