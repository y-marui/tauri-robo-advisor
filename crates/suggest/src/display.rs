//! Serializing JPY amounts for the screen.
//!
//! The structs keep exact decimals for every calculation; only what goes over
//! the wire to the frontend is rounded to a whole yen (half away from zero), so
//! the frontend shows the string it is given and never rounds anything itself.
//! Shares and other ratios do not use this.

use rust_decimal::Decimal;
use serde::Serializer;

pub fn yen<S: Serializer>(amount: &Decimal, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.collect_str(&portfolio::round_for_display("JPY", *amount))
}

pub fn yen_option<S: Serializer>(
    amount: &Option<Decimal>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match amount {
        Some(amount) => yen(amount, serializer),
        None => serializer.serialize_none(),
    }
}
