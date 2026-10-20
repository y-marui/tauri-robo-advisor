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
    /// Sell and buy in the rebalancing account to reach the target.
    FullRebalance,
    /// Raise `amount` by selling what is furthest above target.
    FixedSell { amount: Decimal },
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Request {
    pub account: AccountId,
    pub mode: Mode,
    /// Only buy products the account holds (value above zero in any record). The by-account
    /// tabs set it; the overall suggestion does not.
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

/// One trade in the chosen account. `delta` is positive to buy, negative to sell.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ProductLine {
    pub product: ProductId,
    #[serde(serialize_with = "crate::display::yen")]
    pub delta: Decimal,
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
    /// Every category is already within its tolerance, or too close to the
    /// target for a trade of at least one rounding unit.
    AlreadyBalanced,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Suggestion {
    pub mode: ModeName,
    pub account: AccountId,
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
