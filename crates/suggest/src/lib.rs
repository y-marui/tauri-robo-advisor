//! Rebalancing suggestions over a [`portfolio::Portfolio`].
//!
//! Four modes are supported, all computed with exact decimal arithmetic and all
//! respecting each account's rules (what it may sell, what it can buy, and its
//! rounding unit):
//!
//! * fixed add: spread an amount over what is furthest below target, never selling;
//! * buy-only to target: the smallest amount that reaches the target without
//!   selling, and how to spend it;
//! * full rebalance: sell and buy in the rebalancing account to reach the target;
//! * fixed sell: raise an amount by selling what is furthest above target.
//!
//! The crate is pure calculation. It never changes the portfolio and does not
//! know about Tauri, files, or brokerages.

mod engine;
mod error;
mod fill;
mod overview;
mod round;
mod types;

pub use engine::suggest;
pub use error::Error;
pub use overview::{CategoryOverview, Overview, overview};
pub use types::{CategoryLine, Mode, ModeName, Note, ProductLine, Request, Suggestion};

#[cfg(test)]
mod tests;
