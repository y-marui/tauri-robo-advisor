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
    /// A cell that holds several values (separated by line breaks or no-break spaces, like
    /// "ticker / name" or "quantity / sellable quantity / ordered quantity") is read from its
    /// n-th value (counted from 0). `None`: the whole cell, with the line breaks as spaces.
    #[serde(default)]
    pub name_part: Option<usize>,
    #[serde(default)]
    pub value_part: Option<usize>,
    #[serde(default)]
    pub price_part: Option<usize>,
    /// Read the first number in a cell that has other text around it ("123.45US$",
    /// "150.25（10/15）"). Off: the cell must be only a number.
    #[serde(default)]
    pub loose_numbers: bool,
    /// A quantity column to record when the value is read from an evaluation column.
    #[serde(default)]
    pub quantity: Option<usize>,
    #[serde(default)]
    pub quantity_part: Option<usize>,
    /// Read at most this many records from the top; `None`: all.
    #[serde(default)]
    pub max_records: Option<usize>,
}

/// A row that was read.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Extracted {
    pub name: String,
    pub value: Decimal,
    /// What the value was worked out from, when the table gave it: the quantity and the
    /// price (quantity x price / `price_unit`), or the quantity of a column that was
    /// recorded next to an evaluation column.
    pub quantity: Option<Decimal>,
    pub price: Option<Decimal>,
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
        if mapping.max_records.is_some_and(|max| rows.len() >= max) {
            break;
        }
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
        let name = clean(&pick(name, mapping.name_part, true));
        if name.is_empty() {
            skip(&mut skipped, SkipReason::NoName);
            start = record + 1;
            continue;
        }
        let first = pick(first, mapping.value_part, false);
        let second = second.map(|cell| pick(cell, mapping.price_part, false));
        let read = |text: &str| {
            if mapping.loose_numbers {
                parse_amount_loose(text)
            } else {
                parse_amount(text)
            }
        };
        let recordable = |n: Decimal| (n >= Decimal::ZERO && n <= MAX_VALUE).then_some(n);
        // What is kept next to the value: quantity and price, or only a recorded quantity.
        let (quantity, price) = match &second {
            None => (
                mapping
                    .quantity
                    .and_then(|column| value_cells.get(column))
                    .and_then(|cell| read(&pick(cell, mapping.quantity_part, false)))
                    .and_then(recordable),
                None,
            ),
            Some(price) => (
                read(&first).and_then(recordable),
                read(price).and_then(recordable),
            ),
        };
        let value = match second {
            None => read(&first),
            Some(price) => match (read(&first), read(&price)) {
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
            Some(value) => rows.push(Extracted {
                name,
                value,
                quantity,
                price,
            }),
        }
    }
    Extraction { rows, skipped }
}

/// The longest a single cell is taken to be, so a long text cannot make a part search slow.
const MAX_PARTS: usize = 50;

/// The n-th value of a cell, or the whole cell (line breaks as spaces) when `part` is `None`.
/// Values are separated by line breaks, and, in a name, by no-break spaces (a no-break space
/// inside a number is a thousands separator, so numbers are split at line breaks only); empty
/// ones do not count.
fn pick(cell: &str, part: Option<usize>, split_nbsp: bool) -> String {
    match part {
        None => cell
            .split(['\n', '\r'])
            .map(|line| clean(&line.replace('\u{a0}', " ")))
            .filter(|p| !p.is_empty())
            .collect::<Vec<_>>()
            .join(" "),
        Some(n) => cell
            .split(|c| matches!(c, '\n' | '\r') || (split_nbsp && c == '\u{a0}'))
            .map(clean)
            .filter(|p| !p.is_empty())
            .take(MAX_PARTS)
            .nth(n)
            .unwrap_or_default(),
    }
}

/// Reads the first number in a text that has other text around it: "123.45US$" and
/// "150.25（10/15）" give 123.45 and 150.25. A sign directly before the digits counts.
/// No number in the text gives `None`.
pub(crate) fn parse_amount_loose(text: &str) -> Option<Decimal> {
    let text = clean(text);
    let chars: Vec<char> = text.chars().collect();
    let digit = |c: char| c.is_ascii_digit() || ('０'..='９').contains(&c);
    let start = chars.iter().position(|&c| digit(c))?;
    let mut token = String::new();
    let mut seen_point = false;
    // A decimal point directly before the first digit (".5") is part of the number.
    let mut sign_at = start;
    if start > 0 && matches!(chars[start - 1], '.' | '．') {
        seen_point = true;
        sign_at = start - 1;
    }
    if sign_at > 0 && matches!(chars[sign_at - 1], '-' | '−' | '－' | '▲' | '△') {
        token.push('-');
    }
    if seen_point {
        token.push_str("0.");
    }
    let rest = &chars[start..];
    for (k, &c) in rest.iter().enumerate() {
        match c {
            '0'..='9' => token.push(c),
            '０'..='９' => token.push(char::from_u32(c as u32 - '０' as u32 + '0' as u32)?),
            ',' | '，' => {}
            // A space inside a number groups thousands only when exactly three digits follow
            // ("1 234円"); anything else ends the number.
            ' ' | '\u{a0}' | '\u{3000}' if !seen_point => {
                let three = rest
                    .get(k + 1..k + 4)
                    .is_some_and(|next| next.iter().all(|&d| digit(d)));
                let after_is_digit = rest.get(k + 4).is_some_and(|&d| digit(d));
                if !three || after_is_digit {
                    break;
                }
            }
            '.' | '．' if !seen_point => {
                seen_point = true;
                token.push('.');
            }
            _ => break,
        }
    }
    Decimal::from_str(token.trim_end_matches('.')).ok()
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
