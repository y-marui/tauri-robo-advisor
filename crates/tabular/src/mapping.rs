use std::str::FromStr;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::{MAX_ROWS_PER_RECORD, MAX_VALUE};

fn one<T: From<u8>>() -> T {
    T::from(1)
}

/// Where the name and the value are, and how many leading rows are headings.
/// Columns and rows are counted from 0.
///
/// A record is `rows_per_record` table rows. The name is in row `name_row` of
/// the record and the value in row `value_row`, so a holding that a page shows
/// over two rows (a name row, then a row of numbers) is one record. The value
/// is the number in column `value`; or, when `price` is set, `value` is a
/// quantity and the value is quantity x price / `price_unit` (funds quote the
/// price per 10,000 units).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColumnMapping {
    pub name: usize,
    pub value: usize,
    #[serde(default)]
    pub price: Option<usize>,
    #[serde(default = "one")]
    pub price_unit: u64,
    #[serde(default)]
    pub header_rows: usize,
    #[serde(default = "one")]
    pub rows_per_record: usize,
    #[serde(default)]
    pub name_row: usize,
    #[serde(default)]
    pub value_row: usize,
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

/// Picks the name and value of every record after the headings. Records that
/// do not fit are reported, never guessed at.
pub fn extract(table: &[Vec<String>], mapping: &ColumnMapping) -> Extraction {
    let mut rows = Vec::new();
    let mut skipped = Vec::new();
    // The mapping comes from outside this crate: keep every step small enough
    // that the loop below can neither overflow nor stall.
    let per_record = mapping.rows_per_record.clamp(1, MAX_ROWS_PER_RECORD);
    let unit = Decimal::from(mapping.price_unit.max(1));
    let mut start = mapping.header_rows;
    while start < table.len() {
        let row = start + 1;
        let record = start;
        // A record that is read moves on by its whole length. A row that does
        // not fit (a section title, a subtotal, a label row) moves on by one
        // row only, so the records after it line up again.
        start += per_record;
        let skip = |skipped: &mut Vec<Skipped>, reason| {
            skipped.push(Skipped { row, reason });
        };

        let cells = |offset: usize| -> Option<&Vec<String>> {
            (offset < per_record)
                .then(|| table.get(record + offset))
                .flatten()
        };
        let (Some(name_cells), Some(value_cells)) =
            (cells(mapping.name_row), cells(mapping.value_row))
        else {
            skip(&mut skipped, SkipReason::MissingColumn);
            start = record + 1;
            continue;
        };
        let Some(name) = name_cells.get(mapping.name) else {
            skip(&mut skipped, SkipReason::MissingColumn);
            start = record + 1;
            continue;
        };
        let Some(first) = value_cells.get(mapping.value) else {
            skip(&mut skipped, SkipReason::MissingColumn);
            start = record + 1;
            continue;
        };
        let second = match mapping.price {
            None => None,
            Some(column) => match value_cells.get(column) {
                Some(cell) => Some(cell),
                None => {
                    skip(&mut skipped, SkipReason::MissingColumn);
                    start = record + 1;
                    continue;
                }
            },
        };
        let name = clean(name);
        if name.is_empty() {
            skip(&mut skipped, SkipReason::NoName);
            start = record + 1;
            continue;
        }
        let value = match second {
            None => parse_amount(first),
            Some(price) => match (parse_amount(first), parse_amount(price)) {
                (Some(quantity), Some(price)) => {
                    // An overflowing product is out of range, not a number.
                    match quantity
                        .checked_mul(price)
                        .and_then(|v| v.checked_div(unit))
                    {
                        Some(value) => Some(value),
                        None => {
                            skip(&mut skipped, SkipReason::OutOfRange);
                            start = record + 1;
                            continue;
                        }
                    }
                }
                _ => None,
            },
        };
        match value {
            None => {
                skip(&mut skipped, SkipReason::NotANumber);
                start = record + 1;
            }
            Some(v) if v < Decimal::ZERO || v > MAX_VALUE => {
                skip(&mut skipped, SkipReason::OutOfRange);
                start = record + 1;
            }
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
