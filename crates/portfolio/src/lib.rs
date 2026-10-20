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
mod money;
mod presets;
mod validate;
mod valuation;

pub use edit::{Edit, HoldingInput};
pub use error::{Error, Issue};
pub use import::{ImportPlan, ImportRow, MatchedRow};
pub use model::*;
pub use money::{ShownValue, minor_units, round_for_display};
pub use presets::{PresetInfo, login_pages, login_url_for, presets};

#[cfg(test)]
mod tests;
