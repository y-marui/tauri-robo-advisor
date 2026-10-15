use rust_decimal::Decimal;

/// Why an edit or a stored file was rejected. Messages describe the problem
/// without repeating amounts or names, so they are safe to log.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum Error {
    #[error("invalid value: {0}")]
    Invalid(String),
    #[error("not found: {0}")]
    NotFound(&'static str),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("the stored data uses schema version {found}, which this app does not support")]
    UnsupportedVersion { found: u32 },
    #[error("the stored data is inconsistent: {0}")]
    Corrupt(String),
    #[error("an amount is too large to calculate exactly")]
    Overflow,
    #[error("holdings in {0} cannot be valued; only JPY and USD are supported")]
    UnsupportedCurrency(String),
    #[error("the USD/JPY rate is not set")]
    MissingExchangeRate,
}

/// A problem that does not block saving but blocks computing suggestions.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Issue {
    /// The category target weights do not add up to 1.
    CategoryWeightsDoNotSumToOne { sum: Decimal },
    /// A category with weighted products has a product without a weight.
    WeightedProductMissingWeight { category: crate::CategoryId },
    /// A category set to equal, weighted, or keep-current has no products.
    CategoryHasNoProducts { category: crate::CategoryId },
    /// A foreign-currency holding exists but the exchange rate is not set.
    MissingExchangeRate { currency: String },
}
