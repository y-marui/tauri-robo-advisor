//! Rebalancing suggestions over a [`portfolio::Portfolio`].
//!
//! Four modes are supported, all computed with exact decimal arithmetic and all
//! respecting each account's rules (what it may sell, what it can buy, and its
//! rounding unit):
//!
//! * fixed add: spread an amount over what is furthest below target, never selling;
//! * buy-only to target: the smallest amount that reaches the target without
//!   selling, and how to spend it;
//! * full rebalance: sell and buy in the request's account to reach the target;
//! * fixed sell: raise an amount by selling what is furthest above target.
//!
//! The crate is pure calculation. It never changes the portfolio and does not
//! know about Tauri, files, or brokerages.

mod account_balance;
mod display;
mod engine;
mod error;
mod fill;
mod overview;
mod round;
mod split;
mod strategy;
mod types;

pub use account_balance::{AccountBalance, CategoryBalance, ProductBalance, account_balance};
pub use engine::suggest;
pub use error::Error;
pub use overview::{
    AccountOverview, CategoryOverview, GroupOverview, HoldingOverview, Overview, TypeOverview,
    overview,
};
pub use split::TypeValue;
pub use strategy::StrategyBalance;
pub use types::{CategoryLine, Method, Mode, ModeName, Note, ProductLine, Request, Suggestion};

#[cfg(test)]
mod account_level_tests;
#[cfg(test)]
mod strategy_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod type_tests;
