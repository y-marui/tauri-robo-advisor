//! Reads repeated blocks ("cards") under a heading, for pages that show holdings as cards and not
//! as a table. The script is fixed and only reads (see `js/read_cards.js`); the result has the
//! shape of a table read (one row per card, one text line per cell), so the table import is
//! reused as it is.

use serde_json::Value;

use crate::{Cdp, Error};

/// The limits on what the page may hand back; the script cuts at the same numbers.
pub const MAX_HEADINGS: usize = 50;
pub const MAX_CARD_ITEMS: usize = 500;
const MAX_HEADING_CHARS: usize = 60;
const MAX_LINES: usize = 20;
const MAX_LINE_CHARS: usize = 1_000;

pub(crate) const READ_CARDS: &str = include_str!("../js/read_cards.js");

/// The visible headings of a page, to choose the one a card list is under.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Headings {
    pub list: Vec<String>,
    /// The page has more headings than `MAX_HEADINGS`.
    pub cut: bool,
}

/// The expression to run: the fixed script called with its arguments. The heading goes in as a JSON
/// string, never as code.
fn call(mode: &str, heading: &str, depth: u8) -> String {
    format!(
        "({READ_CARDS})({}, {}, {})",
        Value::from(mode),
        Value::from(heading),
        depth
    )
}

fn parse_headings(value: &Value) -> Result<Headings, Error> {
    let list = value
        .get("headings")
        .and_then(Value::as_array)
        .ok_or(Error::Script)?;
    if list.len() > MAX_HEADINGS {
        return Err(Error::Script);
    }
    let list = list
        .iter()
        .map(|h| match h.as_str() {
            Some(text) if text.chars().count() <= MAX_HEADING_CHARS => Ok(text.to_owned()),
            _ => Err(Error::Script),
        })
        .collect::<Result<_, _>>()?;
    let cut = value
        .get("cut")
        .and_then(Value::as_bool)
        .ok_or(Error::Script)?;
    Ok(Headings { list, cut })
}

/// Checks the shape of what the page returned instead of trusting it. A page that was cut at a
/// limit is refused (applying a partial result would delete holdings), and no card at all is an
/// error.
fn parse_cards(value: &Value) -> Result<Vec<Vec<String>>, Error> {
    if let Some(code) = value.get("error") {
        return Err(match code.as_str() {
            Some("none") => Error::CardsHeadingNotFound,
            Some("many") => Error::CardsHeadingAmbiguous,
            Some("shallow") => Error::CardsTooShallow,
            _ => Error::Script,
        });
    }
    let items = value
        .get("items")
        .and_then(Value::as_array)
        .ok_or(Error::Script)?;
    let cut = value
        .get("cut")
        .and_then(Value::as_bool)
        .ok_or(Error::Script)?;
    if items.len() > MAX_CARD_ITEMS {
        return Err(Error::Script);
    }
    let rows = items
        .iter()
        .map(|item| {
            let lines = item.as_array().ok_or(Error::Script)?;
            if lines.len() > MAX_LINES {
                return Err(Error::Script);
            }
            lines
                .iter()
                .map(|line| match line.as_str() {
                    Some(text) if text.chars().count() <= MAX_LINE_CHARS => Ok(text.to_owned()),
                    _ => Err(Error::Script),
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .collect::<Result<Vec<_>, _>>()?;
    if cut || rows.len() >= MAX_CARD_ITEMS {
        return Err(Error::CardsCut);
    }
    if rows.is_empty() || rows.iter().all(Vec::is_empty) {
        return Err(Error::CardsEmpty);
    }
    Ok(rows)
}

impl Cdp {
    /// The visible headings of the page (`h1` to `h6`). Only reads.
    pub fn read_headings(&mut self, session: &str) -> Result<Headings, Error> {
        parse_headings(&self.evaluate(session, &call("list", "", 1))?)
    }

    /// Reads the cards under the one visible heading that starts with `heading`: each block
    /// `depth` (1 to 3) levels below the region after the heading is one row, and each text line
    /// of it is one cell. Only reads; nothing is clicked or typed.
    pub fn read_cards(
        &mut self,
        session: &str,
        heading: &str,
        depth: u8,
    ) -> Result<Vec<Vec<String>>, Error> {
        parse_cards(&self.evaluate(session, &call("read", heading, depth.clamp(1, 3)))?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn cards_come_back_as_rows_of_lines_with_different_line_counts() {
        let rows = parse_cards(&json!({"items": [["A", "1"], ["B", "2", "x"], []], "cut": false}));
        assert_eq!(
            rows,
            Ok(vec![
                vec!["A".to_owned(), "1".to_owned()],
                vec!["B".to_owned(), "2".to_owned(), "x".to_owned()],
                vec![],
            ])
        );
    }

    #[test]
    fn a_page_cut_at_a_limit_is_refused() {
        assert_eq!(
            parse_cards(&json!({"items": [["A"]], "cut": true})),
            Err(Error::CardsCut)
        );
        let full = vec![json!(["a"]); MAX_CARD_ITEMS];
        assert_eq!(
            parse_cards(&json!({"items": full, "cut": false})),
            Err(Error::CardsCut)
        );
    }

    #[test]
    fn no_card_and_the_scripts_errors_are_errors() {
        assert_eq!(
            parse_cards(&json!({"items": [], "cut": false})),
            Err(Error::CardsEmpty)
        );
        assert_eq!(
            parse_cards(&json!({"items": [[], []], "cut": false})),
            Err(Error::CardsEmpty)
        );
        for (code, error) in [
            ("none", Error::CardsHeadingNotFound),
            ("many", Error::CardsHeadingAmbiguous),
            ("shallow", Error::CardsTooShallow),
            ("other", Error::Script),
        ] {
            assert_eq!(parse_cards(&json!({"error": code})), Err(error));
        }
    }

    #[test]
    fn an_answer_of_the_wrong_shape_is_refused() {
        let long_line = "x".repeat(MAX_LINE_CHARS + 1);
        let too_many_lines = vec!["a"; MAX_LINES + 1];
        for wrong in [
            Value::Null,
            json!("text"),
            json!({"items": [["a"]]}),
            json!({"items": "a", "cut": false}),
            json!({"items": [[1]], "cut": false}),
            json!({"items": [[long_line]], "cut": false}),
            json!({"items": [too_many_lines], "cut": false}),
            json!({"items": vec![json!(["a"]); MAX_CARD_ITEMS + 1], "cut": false}),
        ] {
            assert_eq!(parse_cards(&wrong), Err(Error::Script), "{wrong}");
        }
    }

    #[test]
    fn headings_are_checked_and_limited() {
        assert_eq!(
            parse_headings(&json!({"headings": ["A", "B"], "cut": true})),
            Ok(Headings {
                list: vec!["A".to_owned(), "B".to_owned()],
                cut: true
            })
        );
        for wrong in [
            json!({"headings": [1], "cut": false}),
            json!({"headings": ["x".repeat(61)], "cut": false}),
            json!({"headings": vec!["a"; MAX_HEADINGS + 1], "cut": false}),
            json!({"headings": []}),
        ] {
            assert_eq!(parse_headings(&wrong), Err(Error::Script), "{wrong}");
        }
    }

    #[test]
    fn the_heading_is_passed_as_a_string_and_the_script_only_reads() {
        let expression = call("read", "A\"); evil(\"", 2);
        assert!(expression.contains(r#""A\"); evil(\"""#));
        for action in [
            "click(",
            "submit(",
            "dispatchEvent",
            ".value =",
            ".href",
            "getAttribute",
            "fetch(",
            "XMLHttpRequest",
            "eval(",
            "Function(",
            "location",
            "innerHTML",
            "scroll",
            "focus(",
        ] {
            assert!(!READ_CARDS.contains(action), "{action}");
        }
    }
}
