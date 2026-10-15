use serde::Serialize;

use crate::{Error, MAX_CELL_CHARS, MAX_COLUMNS, MAX_ROWS};

/// What separates the cells of a line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Delimiter {
    Comma,
    Tab,
}

impl Delimiter {
    fn character(self) -> char {
        match self {
            Self::Comma => ',',
            Self::Tab => '\t',
        }
    }
}

/// Tab-separated when the first lines contain a tab (a table copied from a web
/// page), otherwise comma-separated. A tab inside a quoted CSV cell is rare
/// enough that a file that starts with a comma header is still read as CSV.
fn detect(text: &str) -> Delimiter {
    let head: Vec<&str> = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .take(5)
        .collect();
    let tabs = head.iter().filter(|l| l.contains('\t')).count();
    let commas = head.iter().filter(|l| l.contains(',')).count();
    if tabs > 0 && tabs >= commas {
        Delimiter::Tab
    } else {
        Delimiter::Comma
    }
}

/// Splits text into rows of cells. Quoted cells may contain the delimiter, line
/// breaks, and doubled quotes. Blank lines are dropped. The size limits are
/// checked while reading, so a huge input stops early instead of exhausting
/// memory.
pub fn parse(text: &str) -> Result<(Vec<Vec<String>>, Delimiter), Error> {
    let delimiter = detect(text);
    let separator = delimiter.character();
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut cell = String::new();
    let mut cell_chars = 0usize;
    let mut in_quotes = false;
    let mut quote_line = 0usize;
    let mut line = 1usize;
    let mut chars = text.chars().peekable();

    fn push_cell(
        row: &mut Vec<String>,
        cell: &mut String,
        cell_chars: &mut usize,
    ) -> Result<(), Error> {
        if row.len() >= MAX_COLUMNS {
            return Err(Error::TooLarge);
        }
        row.push(std::mem::take(cell));
        *cell_chars = 0;
        Ok(())
    }

    let finish_row = |rows: &mut Vec<Vec<String>>, row: &mut Vec<String>| -> Result<(), Error> {
        let blank = row.iter().all(|c| c.trim().is_empty());
        let finished = std::mem::take(row);
        if !blank {
            if rows.len() >= MAX_ROWS {
                return Err(Error::TooLarge);
            }
            rows.push(finished);
        }
        Ok(())
    };

    while let Some(c) = chars.next() {
        if in_quotes {
            match c {
                '"' if chars.peek() == Some(&'"') => {
                    chars.next();
                    cell.push('"');
                    cell_chars += 1;
                }
                '"' => in_quotes = false,
                _ => {
                    if c == '\n' {
                        line += 1;
                    }
                    cell.push(c);
                    cell_chars += 1;
                }
            }
        } else if c == '"' && cell.is_empty() {
            in_quotes = true;
            quote_line = line;
        } else if c == separator {
            push_cell(&mut row, &mut cell, &mut cell_chars)?;
        } else if c == '\n' || c == '\r' {
            if c == '\r' && chars.peek() == Some(&'\n') {
                chars.next();
            }
            line += 1;
            push_cell(&mut row, &mut cell, &mut cell_chars)?;
            finish_row(&mut rows, &mut row)?;
        } else {
            cell.push(c);
            cell_chars += 1;
        }
        if cell_chars > MAX_CELL_CHARS {
            return Err(Error::TooLarge);
        }
    }
    if in_quotes {
        return Err(Error::Malformed(quote_line));
    }
    push_cell(&mut row, &mut cell, &mut cell_chars)?;
    finish_row(&mut rows, &mut row)?;
    if rows.is_empty() {
        return Err(Error::Empty);
    }
    Ok((rows, delimiter))
}
