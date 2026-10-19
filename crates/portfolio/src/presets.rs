//! Fetch recipes that ship with the app, as data.
//!
//! A preset is an ordinary recipe (the same fields the user can write) for a page layout that
//! is known, so a user does not have to author it. There is no brokerage-specific code: when a
//! page changes, the fix is a change to the data here (or to the user's copy of the recipe).
//! What a preset names was checked against the structure of the real page (headings and shape
//! only); the names below are the labels the pages themselves show.

use serde::Serialize;

use crate::{AccountId, FetchLink, ImportMapping, Recipe};

/// A recipe for a known page, waiting for an account.
pub struct Preset {
    pub id: &'static str,
    pub label: &'static str,
    /// Where to start, and what it reads, in a sentence for the screen.
    pub description: &'static str,
    links: &'static [&'static str],
    table_starts_with: &'static str,
    price_unit: u64,
}

/// What the screen needs to list a preset.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PresetInfo {
    pub id: &'static str,
    pub label: &'static str,
    pub description: &'static str,
}

const PRESETS: &[Preset] = &[
    Preset {
        id: "sbi-domestic-stocks",
        label: "SBI Securities: domestic stocks",
        description: "Starts from the top page of SBI Securities (after logging in): follows \"保有資産評価\" and reads the domestic stock tables of every account type (quantity x current price).",
        links: &["保有資産評価"],
        table_starts_with: "株式（現物/",
        price_unit: 1,
    },
    Preset {
        id: "sbi-funds",
        label: "SBI Securities: investment trusts",
        description: "Starts from the top page of SBI Securities (after logging in): follows \"保有資産評価\" and reads the investment trust tables of every account type (units x price per 10,000 units).",
        links: &["保有資産評価"],
        table_starts_with: "投資信託（金額/",
        price_unit: 10_000,
    },
];

/// The presets, in the order they are shown.
pub fn presets() -> Vec<PresetInfo> {
    PRESETS
        .iter()
        .map(|p| PresetInfo {
            id: p.id,
            label: p.label,
            description: p.description,
        })
        .collect()
}

impl Preset {
    pub(crate) fn find(id: &str) -> Option<&'static Preset> {
        PRESETS.iter().find(|p| p.id == id)
    }

    /// The recipe this preset stands for. Every SBI table has the same layout: a title row,
    /// a heading row, then two rows per holding (a row with the name, then a row with the
    /// quantity, the cost, the price, and the profit; only the quantity and the price are read).
    pub(crate) fn recipe(&self, account: AccountId) -> Recipe {
        Recipe {
            name: self.label.to_owned(),
            links: self
                .links
                .iter()
                .map(|text| FetchLink {
                    text: (*text).to_owned(),
                    reference: None,
                })
                .collect(),
            table_starts_with: self.table_starts_with.to_owned(),
            all_tables: true,
            // The values are in the currency of the product they are linked to.
            currency: String::new(),
            account,
            mapping: ImportMapping {
                source: "SBI".to_owned(),
                name_column: 0,
                value_column: 0,
                price_column: Some(2),
                price_unit: self.price_unit,
                header_rows: 2,
                rows_per_record: 2,
                name_row: 0,
                value_row: 1,
                name_part: None,
                value_part: None,
                price_part: None,
                loose_numbers: false,
                quantity_column: None,
                quantity_part: None,
            },
        }
    }
}
