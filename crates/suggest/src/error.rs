use portfolio::Issue;

/// Why no suggestion could be made. Messages never repeat names or amounts.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum Error {
    #[error("fix the problems in the portfolio first")]
    Blocked(Vec<Issue>),
    #[error("the portfolio data is inconsistent")]
    Inconsistent,
    #[error("unknown account")]
    UnknownAccount,
    #[error("there is nothing in the portfolio to base a suggestion on")]
    EmptyPortfolio,
    #[error("the amount must be positive")]
    InvalidAmount,
    #[error("the amount is smaller than this account's rounding unit")]
    AmountBelowUnit,
    #[error("holdings in {0} cannot be valued; only JPY and USD are supported")]
    UnsupportedCurrency(String),
    #[error("set the USD/JPY rate first")]
    MissingExchangeRate,
    #[error("an amount or weight is too large to calculate")]
    TooLarge,
    #[error("this account is not set to allow selling")]
    NotSellable,
    #[error("this account does not hold enough to sell that amount")]
    NotEnoughToSell,
    #[error("this account has no product it can buy in the categories that need money")]
    NothingToBuy,
    #[error(
        "a category with a target of zero still holds value, so the target cannot be reached without selling"
    )]
    CannotReachWithoutSelling,
}
