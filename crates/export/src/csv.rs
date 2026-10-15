//! CSV writing that is safe to open in a spreadsheet.

use rust_decimal::Decimal;

/// A UTF-8 byte order mark. Excel needs it to read Japanese text correctly; other
/// tools ignore it.
const BOM: &[u8] = b"\xEF\xBB\xBF";

/// Quotes a cell when it contains a comma, quote, or line break.
fn quote(text: &str) -> String {
    let needs =
        text.contains([',', '"', '\r', '\n']) || text.starts_with(' ') || text.ends_with(' ');
    if needs {
        format!("\"{}\"", text.replace('"', "\"\""))
    } else {
        text.to_owned()
    }
}

/// A cell holding text the user typed (a name). A spreadsheet runs a cell that
/// starts with `=`, `+`, `-`, `@`, a tab, or a carriage return as a formula, so
/// a name such as `=HYPERLINK(...)` could execute when the export is opened.
/// Such cells are prefixed with an apostrophe, which makes them plain text.
pub fn text_cell(text: &str) -> String {
    if text.starts_with(['=', '+', '-', '@', '\t', '\r']) {
        quote(&format!("'{text}"))
    } else {
        quote(text)
    }
}

/// A cell holding a value this program generated (an id, a number, a keyword).
pub fn plain_cell(text: &str) -> String {
    quote(text)
}

/// A decimal in plain notation without trailing zeros.
pub fn number(value: Decimal) -> String {
    value.normalize().to_string()
}

pub fn optional_number(value: Option<Decimal>) -> String {
    value.map(number).unwrap_or_default()
}

/// One CSV file: a header row and data rows, written with CRLF line ends.
pub struct Table {
    header: Vec<&'static str>,
    rows: Vec<Vec<String>>,
}

impl Table {
    pub fn new(header: &[&'static str]) -> Self {
        Self {
            header: header.to_vec(),
            rows: Vec::new(),
        }
    }

    /// Adds a row of already-escaped cells.
    pub fn push(&mut self, cells: Vec<String>) {
        debug_assert_eq!(cells.len(), self.header.len());
        self.rows.push(cells);
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = BOM.to_vec();
        out.extend_from_slice(self.header.join(",").as_bytes());
        out.extend_from_slice(b"\r\n");
        for row in &self.rows {
            out.extend_from_slice(row.join(",").as_bytes());
            out.extend_from_slice(b"\r\n");
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use rust_decimal::dec;

    use super::*;

    #[test]
    fn formula_prefixes_are_neutralised_in_text_cells() {
        assert_eq!(text_cell("=SUM(A1:A9)"), "'=SUM(A1:A9)");
        assert_eq!(text_cell("+81 90"), "'+81 90");
        assert_eq!(text_cell("-1"), "'-1");
        assert_eq!(text_cell("@cmd"), "'@cmd");
        assert_eq!(text_cell("\tTab"), "'\tTab");
    }

    #[test]
    fn ordinary_text_is_left_alone() {
        assert_eq!(text_cell("Stock Fund"), "Stock Fund");
        assert_eq!(text_cell("日本株ファンド"), "日本株ファンド");
        assert_eq!(text_cell("A=B"), "A=B");
    }

    #[test]
    fn cells_with_commas_quotes_or_breaks_are_quoted() {
        assert_eq!(text_cell("a,b"), "\"a,b\"");
        assert_eq!(text_cell("say \"hi\""), "\"say \"\"hi\"\"\"");
        assert_eq!(text_cell("two\nlines"), "\"two\nlines\"");
        assert_eq!(text_cell(" padded "), "\" padded \"");
        assert_eq!(text_cell("=a,b"), "\"'=a,b\"");
    }

    #[test]
    fn numbers_are_plain_decimals_without_trailing_zeros() {
        assert_eq!(number(dec!(1234.500)), "1234.5");
        assert_eq!(number(dec!(0)), "0");
        assert_eq!(number(dec!(0.000001)), "0.000001");
        assert_eq!(optional_number(None), "");
        assert_eq!(optional_number(Some(dec!(2.50))), "2.5");
    }

    #[test]
    fn a_table_has_a_byte_order_mark_and_crlf_line_ends() {
        let mut table = Table::new(&["a", "b"]);
        table.push(vec!["1".to_owned(), plain_cell("x,y")]);

        let bytes = table.to_bytes();

        assert_eq!(&bytes[..3], b"\xEF\xBB\xBF");
        assert_eq!(&bytes[3..], b"a,b\r\n1,\"x,y\"\r\n");
    }
}
