//! Reads tables from CSV files and pasted text, and picks the name and value
//! columns the user chose.
//!
//! There is no brokerage-specific code here. Export formats differ per
//! brokerage and per version and are not publicly specified, so the user maps
//! the columns instead, and the mapping can be fixed without an app update.
//!
//! The input is untrusted: its size is limited before and during parsing, bad
//! input returns an error, and nothing here panics on any byte sequence.

mod decode;
mod error;
mod mapping;
mod parse;

use rust_decimal::Decimal;
use serde::Serialize;

pub use decode::{Encoding, decode};
pub use error::Error;
pub use mapping::{ColumnMapping, Extracted, Extraction, SkipReason, Skipped, extract};
pub use parse::{Delimiter, parse};

/// The largest input accepted, in bytes.
pub const MAX_INPUT_BYTES: usize = 2 * 1024 * 1024;
/// The most rows and columns kept, and the longest cell.
pub const MAX_ROWS: usize = 20_000;
pub const MAX_COLUMNS: usize = 200;
pub const MAX_CELL_CHARS: usize = 1_000;
/// The most table rows one record may span.
pub const MAX_ROWS_PER_RECORD: usize = 10;
/// Values above one quadrillion yen are refused, as everywhere else.
pub const MAX_VALUE: Decimal = Decimal::from_parts(2_764_472_320, 232_830, 0, false, 0);

/// A table read from a file or from pasted text.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Table {
    pub rows: Vec<Vec<String>>,
    /// `None` for pasted text, which is already text.
    pub encoding: Option<Encoding>,
    /// `None` for rows that were read from a page and never were text.
    pub delimiter: Option<Delimiter>,
}

impl Table {
    /// The widest row.
    pub fn columns(&self) -> usize {
        self.rows.iter().map(Vec::len).max().unwrap_or(0)
    }
}

/// Wraps rows that already are cells (read from a page). The same limits apply
/// as for text, and blank rows are dropped.
pub fn from_rows(rows: Vec<Vec<String>>) -> Result<Table, Error> {
    let rows: Vec<Vec<String>> = rows
        .into_iter()
        .filter(|row| row.iter().any(|cell| !cell.trim().is_empty()))
        .collect();
    if rows.is_empty() {
        return Err(Error::Empty);
    }
    if rows.len() > MAX_ROWS
        || rows.iter().any(|row| {
            row.len() > MAX_COLUMNS || row.iter().any(|c| c.chars().count() > MAX_CELL_CHARS)
        })
    {
        return Err(Error::TooLarge);
    }
    Ok(Table {
        rows,
        encoding: None,
        delimiter: None,
    })
}

/// Reads the bytes of a CSV (or tab-separated) file.
pub fn read_file(bytes: &[u8]) -> Result<Table, Error> {
    let (text, encoding) = decode(bytes)?;
    let (rows, delimiter) = parse(&text)?;
    Ok(Table {
        rows,
        encoding: Some(encoding),
        delimiter: Some(delimiter),
    })
}

/// Reads text pasted from a brokerage's holdings table.
pub fn read_text(text: &str) -> Result<Table, Error> {
    if text.len() > MAX_INPUT_BYTES {
        return Err(Error::TooLarge);
    }
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let (rows, delimiter) = parse(text)?;
    Ok(Table {
        rows,
        encoding: None,
        delimiter: Some(delimiter),
    })
}

#[cfg(test)]
mod tests;
