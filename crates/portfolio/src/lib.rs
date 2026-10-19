//! The portfolio the user builds: categories with target weights, products,
//! accounts, and holdings.
//!
//! The crate is pure data and rules. It knows nothing about Tauri, files, or
//! brokerages: the app persists it (see `src-tauri/src/store.rs`), and every
//! change goes through [`Portfolio::apply`], which validates the edit first.
//! Amounts are exact decimals, never floating point.

mod edit;
mod error;
mod import;
mod model;
mod presets;
mod validate;
mod valuation;

pub use edit::{Edit, HoldingInput};
pub use error::{Error, Issue};
pub use import::{ImportPlan, ImportRow, MatchedRow};
pub use model::*;
pub use presets::{PresetInfo, presets};

#[cfg(test)]
mod tests;
