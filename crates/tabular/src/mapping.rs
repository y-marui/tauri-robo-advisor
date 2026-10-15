use std::str::FromStr;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::MAX_VALUE;

/// Which columns hold the name and the value, and how many leading rows are
/// headings. Columns are counted from 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColumnMapping {
    pub name: usize,
    pub value: usize,
    #[serde(default)]
    pub header_rows: usize,
}

/// A row that was read.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Extracted {
    pub name: String,
    pub value: Decimal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SkipReason {
    /// The row is too short to have the chosen columns.
    MissingColumn,
    /// The name cell is empty.
    NoName,
    /// The value cell is not a number (a subtotal label, a note, a heading).
    NotANumber,
    /// The value is negative or beyond the supported range.
    OutOfRange,
}

/// A row that was left out, so the user can see what was not imported.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Skipped {
    /// The row's position in the table, counted from 1.
    pub row: usize,
    pub reason: SkipReason,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Extraction {
    pub rows: Vec<Extracted>,
    pub skipped: Vec<Skipped>,
}

/// Picks the name and value of every row after the headings. Rows that do not
/// fit are reported, never guessed at.
pub fn extract(table: &[Vec<String>], mapping: &ColumnMapping) -> Extraction {
    let mut rows = Vec::new();
    let mut skipped = Vec::new();
    for (index, cells) in table.iter().enumerate().skip(mapping.header_rows) {
        let row = index + 1;
        let (Some(name), Some(value)) = (cells.get(mapping.name), cells.get(mapping.value)) else {
            skipped.push(Skipped {
                row,
                reason: SkipReason::MissingColumn,
            });
            continue;
        };
        let name = clean(name);
        if name.is_empty() {
            skipped.push(Skipped {
                row,
                reason: SkipReason::NoName,
            });
            continue;
        }
        match parse_amount(value) {
            None => skipped.push(Skipped {
                row,
                reason: SkipReason::NotANumber,
            }),
            Some(v) if v < Decimal::ZERO || v > MAX_VALUE => skipped.push(Skipped {
                row,
                reason: SkipReason::OutOfRange,
            }),
            Some(value) => rows.push(Extracted { name, value }),
        }
    }
    Extraction { rows, skipped }
}

/// Trims ordinary, no-break, and full-width spaces.
fn clean(text: &str) -> String {
    text.trim_matches(|c: char| c.is_whitespace() || c == '\u{a0}' || c == '\u{3000}')
        .to_owned()
}

/// Reads an amount as shown on a Japanese brokerage page: thousands separators,
/// a yen or dollar sign, a trailing 円, full-width digits, and a leading minus
/// (including ▲ and △). Anything else is not a number, so `None`.
pub(crate) fn parse_amount(text: &str) -> Option<Decimal> {
    let mut cleaned = String::new();
    let mut sign_seen = false;
    let mut digits_seen = false;
    for c in clean(text).chars() {
        match c {
            '0'..='9' | '.' => {
                digits_seen = true;
                cleaned.push(c);
            }
            '０'..='９' => {
                digits_seen = true;
                cleaned.push(char::from_u32(c as u32 - '０' as u32 + '0' as u32)?);
            }
            '．' => {
                digits_seen = true;
                cleaned.push('.');
            }
            // One sign at most, and only before the digits: "1+2", "12+", "5-"
            // and "+-5" are not amounts.
            '-' | '−' | '－' | '▲' | '△' | '+' | '＋' => {
                if sign_seen || digits_seen {
                    return None;
                }
                sign_seen = true;
                if !matches!(c, '+' | '＋') {
                    cleaned.push('-');
                }
            }
            ',' | '，' | ' ' | '\u{a0}' | '\u{3000}' | '¥' | '￥' | '$' | '＄' | '円' => {}
            _ => return None,
        }
    }
    Decimal::from_str(&cleaned).ok()
}
