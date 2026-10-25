use portfolio::{AccountId, CategoryId, ProductId};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// What to compute.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Mode {
    /// Spread `amount` over what is furthest below target, without selling.
    FixedAdd { amount: Decimal },
    /// The smallest amount that reaches the target without selling.
    BuyOnlyToTarget,
    /// Sell and buy in the request's account (every account when none) to reach the target.
    FullRebalance,
    /// Raise `amount` by selling what is furthest above target.
    FixedSell { amount: Decimal },
}

/// How the money is allocated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Method {
    /// By the portfolio's category targets, then by the product weights inside each category.
    #[default]
    FollowPortfolio,
    /// Ignoring the category targets: every product that takes part gets its weight's share of
    /// the whole, across categories.
    ByProductWeights,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Request {
    /// The account to trade in, or `None` to trade across every account (each one's own rules
    /// apply: what it may sell, buy, and exclude, and its rounding unit).
    #[serde(default)]
    pub account: Option<AccountId>,
    pub mode: Mode,
    /// How the money is allocated (an account with a product strategy always follows its own).
    #[serde(default)]
    pub method: Method,
    /// Only buy products the account holds (value above zero in any record). Every
    /// Rebalance tab sets it, including "All".
    #[serde(default)]
    pub held_only: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ModeName {
    FixedAdd,
    BuyOnlyToTarget,
    FullRebalance,
    FixedSell,
}

/// What happens to one category. Amounts are in JPY; `delta` is positive for
/// buying and negative for selling.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CategoryLine {
    pub category: CategoryId,
    #[serde(serialize_with = "crate::display::yen")]
    pub current: Decimal,
    #[serde(serialize_with = "crate::display::yen")]
    pub delta: Decimal,
    #[serde(serialize_with = "crate::display::yen")]
    pub after: Decimal,
    /// The value the category would have at its target weight once the
    /// suggestion is carried out.
    #[serde(serialize_with = "crate::display::yen")]
    pub target_after: Decimal,
}

/// One trade of a product in an account. `delta` is positive to buy, negative to sell.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ProductLine {
    pub account: AccountId,
    pub product: ProductId,
    /// For a line split by type (`by_type`): the account type of the records it sells from or buys
    /// into; `None` means the records without a type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_type: Option<String>,
    /// Whether the line is one record type's share of a trade. A sale, or a purchase (by the record
    /// weights), of a product held in several types has one such line per type (see `split`).
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub by_type: bool,
    #[serde(serialize_with = "crate::display::yen")]
    pub delta: Decimal,
    /// What is held once the trade is made. Not split by type: the account's current value
    /// of the product (in every record, as the account balance counts it) plus `delta`. Split
    /// (`by_type`): the current value of that type's records plus `delta`.
    #[serde(serialize_with = "crate::display::yen")]
    pub after: Decimal,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Note {
    /// The amount was lowered to a multiple of the account's rounding unit.
    AmountRounded {
        #[serde(serialize_with = "crate::display::yen")]
        requested: Decimal,
        #[serde(serialize_with = "crate::display::yen")]
        applied: Decimal,
    },
    /// This category could not be brought all the way to its target, for
    /// example because the account cannot buy or sell products in it.
    ///
    /// `shortfall` is positive when buying was needed and negative when a sale
    /// that would have been needed could not be made (the category stays over
    /// its target).
    CategoryNotFilled {
        category: CategoryId,
        #[serde(serialize_with = "crate::display::yen")]
        shortfall: Decimal,
    },
    /// An account with a product strategy could not bring this product to its share; `shortfall`
    /// is positive when buying was needed and negative when a sale could not be made.
    ProductNotFilled {
        product: ProductId,
        #[serde(serialize_with = "crate::display::yen")]
        shortfall: Decimal,
    },
    /// Every category is already within its tolerance, or too close to the
    /// target for a trade of at least one rounding unit.
    AlreadyBalanced,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Suggestion {
    pub mode: ModeName,
    /// The account the suggestion is for; `None` when it spans every account.
    pub account: Option<AccountId>,
    /// The amount the user asked for, when the mode takes one.
    #[serde(serialize_with = "crate::display::yen_option")]
    pub requested: Option<Decimal>,
    /// The total bought (or, for a sale, sold) after rounding.
    #[serde(serialize_with = "crate::display::yen")]
    pub applied: Decimal,
    /// For the buy-only mode: the exact amount needed to reach the target.
    #[serde(serialize_with = "crate::display::yen_option")]
    pub required_total: Option<Decimal>,
    /// The whole portfolio's value once the suggestion is carried out.
    #[serde(serialize_with = "crate::display::yen")]
    pub total_after: Decimal,
    pub categories: Vec<CategoryLine>,
    pub products: Vec<ProductLine>,
    pub notes: Vec<Note>,
}
