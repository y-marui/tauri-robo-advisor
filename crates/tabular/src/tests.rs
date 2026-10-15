// Every name and amount in these tests is made up.

use rust_decimal::dec;

use crate::*;

fn mapping(name: usize, value: usize, header_rows: usize) -> ColumnMapping {
    ColumnMapping {
        name,
        value,
        price: None,
        price_unit: 1,
        header_rows,
        rows_per_record: 1,
        name_row: 0,
        value_row: 0,
    }
}

/// A holding over two rows: a name row, then a row of quantity, cost, price.
fn two_rows(header_rows: usize) -> ColumnMapping {
    ColumnMapping {
        price: Some(2),
        header_rows,
        rows_per_record: 2,
        value_row: 1,
        ..mapping(0, 0, header_rows)
    }
}

fn grid(rows: &[&[&str]]) -> Vec<Vec<String>> {
    rows.iter()
        .map(|r| r.iter().map(|c| (*c).to_owned()).collect())
        .collect()
}

// ---- encodings ------------------------------------------------------------

#[test]
fn utf8_is_read_with_or_without_a_byte_order_mark() {
    let plain = read_file("名前,評価額\nA,100\n".as_bytes()).unwrap();
    let mut with_mark = b"\xEF\xBB\xBF".to_vec();
    with_mark.extend_from_slice("名前,評価額\nA,100\n".as_bytes());
    let marked = read_file(&with_mark).unwrap();

    assert_eq!(plain.encoding, Some(Encoding::Utf8));
    assert_eq!(plain.rows, marked.rows);
    assert_eq!(plain.rows[0], vec!["名前", "評価額"]);
}

#[test]
fn shift_jis_is_detected_and_decoded() {
    let (bytes, _, _) = encoding_rs::SHIFT_JIS.encode("銘柄,評価額\nテスト債券,1000\n");

    let table = read_file(&bytes).unwrap();

    assert_eq!(table.encoding, Some(Encoding::ShiftJis));
    assert_eq!(table.rows[1], vec!["テスト債券", "1000"]);
}

#[test]
fn bytes_that_are_neither_encoding_are_refused() {
    // 0x81 starts a two-byte Shift-JIS character, and 0x20 is not a valid
    // second byte.
    assert_eq!(read_file(b"a,b\n\x81\x20,1\n"), Err(Error::Encoding));
}

#[test]
fn binary_and_empty_input_are_refused() {
    assert_eq!(read_file(b"PK\x03\x04\x00\x00"), Err(Error::NotText));
    assert_eq!(read_file(b""), Err(Error::Empty));
    assert_eq!(read_text("  \n\n"), Err(Error::Empty));
}

#[test]
fn oversized_input_is_refused_before_parsing() {
    let big = vec![b'a'; MAX_INPUT_BYTES + 1];

    assert_eq!(read_file(&big), Err(Error::TooLarge));
}

// ---- parsing --------------------------------------------------------------

#[test]
fn quoted_fields_may_contain_separators_quotes_and_line_breaks() {
    let table =
        read_text("name,value\n\"A, Inc.\",\"1,000\"\n\"say \"\"hi\"\"\",\"2\n3\"\n").unwrap();

    assert_eq!(table.rows[1], vec!["A, Inc.", "1,000"]);
    assert_eq!(table.rows[2], vec!["say \"hi\"", "2\n3"]);
}

#[test]
fn pasted_tab_separated_text_is_detected() {
    let table = read_text("名前\t評価額\nA\t1,000\r\nB\t2,000\r\n").unwrap();

    assert_eq!(table.delimiter, Delimiter::Tab);
    assert_eq!(table.rows.len(), 3);
    assert_eq!(table.rows[2], vec!["B", "2,000"]);
}

#[test]
fn blank_lines_are_dropped_and_rows_may_be_ragged() {
    let table = read_text("a,b,c\n\n\nd,e\n").unwrap();

    assert_eq!(table.rows, vec![vec!["a", "b", "c"], vec!["d", "e"]]);
    assert_eq!(table.columns(), 3);
}

#[test]
fn an_unclosed_quote_is_an_error_not_a_panic() {
    assert_eq!(read_text("a,b\nc,\"d\n"), Err(Error::Malformed(2)));
}

#[test]
fn too_many_columns_and_too_long_cells_are_refused() {
    let wide = (0..=MAX_COLUMNS).map(|_| "x").collect::<Vec<_>>().join(",");
    let long = "y".repeat(MAX_CELL_CHARS + 1);

    assert_eq!(read_text(&wide), Err(Error::TooLarge));
    assert_eq!(read_text(&long), Err(Error::TooLarge));
}

#[test]
fn too_many_rows_are_refused() {
    let text = "a,b\n".repeat(MAX_ROWS + 1);

    assert_eq!(read_text(&text), Err(Error::TooLarge));
}

#[test]
fn odd_input_never_panics() {
    for text in [
        "\"", "\"\"\"", ",,,,", "\t\t", "a\"b,c", "\r\r\r", "\u{feff}", "\"a\"b",
    ] {
        let _ = read_text(text);
    }
}

// ---- columns --------------------------------------------------------------

#[test]
fn amounts_are_read_as_shown_on_japanese_pages() {
    let table = vec![
        vec!["name".to_owned(), "value".to_owned()],
        vec!["A".to_owned(), "1,234,567円".to_owned()],
        vec!["B".to_owned(), "¥ 2,000.50".to_owned()],
        vec!["C".to_owned(), "１２３".to_owned()],
        vec!["D".to_owned(), "+5".to_owned()],
    ];

    let result = extract(&table, &mapping(0, 1, 1));

    let values: Vec<_> = result.rows.iter().map(|r| r.value).collect();
    assert_eq!(
        values,
        vec![dec!(1234567), dec!(2000.50), dec!(123), dec!(5)]
    );
    assert!(result.skipped.is_empty());
}

#[test]
fn rows_that_do_not_fit_are_reported_with_a_reason() {
    let row = |a: &str, b: &str| vec![a.to_owned(), b.to_owned()];
    let table = vec![
        row("name", "value"),
        row("A", "100"),
        row("小計", "合計"),
        row("", "50"),
        row("B", "-5"),
        row("C", "▲7"),
        row("D", "99999999999999999999"),
        vec!["E".to_owned()],
    ];

    let result = extract(&table, &mapping(0, 1, 1));

    assert_eq!(result.rows.len(), 1);
    let reasons: Vec<_> = result.skipped.iter().map(|s| (s.row, s.reason)).collect();
    assert_eq!(
        reasons,
        vec![
            (3, SkipReason::NotANumber),
            (4, SkipReason::NoName),
            (5, SkipReason::OutOfRange),
            (6, SkipReason::OutOfRange),
            (7, SkipReason::OutOfRange),
            (8, SkipReason::MissingColumn),
        ]
    );
}

#[test]
fn names_are_trimmed_of_ordinary_and_full_width_spaces() {
    let table = vec![vec!["\u{3000} Fund A\u{a0}".to_owned(), "10".to_owned()]];

    let result = extract(&table, &mapping(0, 1, 0));

    assert_eq!(result.rows[0].name, "Fund A");
}

#[test]
fn text_that_only_looks_like_a_number_is_not_one() {
    for bad in [
        "1-2", "1e5", "0x10", "--1", "1.2.3", "abc", "", "1+2", "12+", "+-5", "5-", "-+5",
    ] {
        let table = vec![vec!["A".to_owned(), bad.to_owned()]];
        let result = extract(&table, &mapping(0, 1, 0));
        assert!(result.rows.is_empty(), "{bad}");
    }
}

#[test]
fn a_single_leading_sign_is_accepted_even_after_a_currency_sign() {
    for (text, expected) in [
        ("+5", dec!(5)),
        ("-5", dec!(-5)),
        ("¥-5", dec!(-5)),
        ("▲1,000", dec!(-1000)),
    ] {
        assert_eq!(crate::mapping::parse_amount(text), Some(expected), "{text}");
    }
}

// ---- records over several rows and quantity x price ------------------------

#[test]
fn old_mappings_without_the_new_fields_still_load() {
    let json = r#"{"name":0,"value":1,"header_rows":1}"#;
    let loaded: ColumnMapping = serde_json::from_str(json).unwrap();

    assert_eq!(loaded, mapping(0, 1, 1));
}

#[test]
fn a_holding_over_two_rows_is_one_record() {
    // Section title, column labels, then name row / numbers row per holding.
    let table = grid(&[
        &["Section", "", ""],
        &["Quantity", "Cost", "Price"],
        &["Fund A", "", ""],
        &["10", "900", "1,250.5"],
        &["Fund B", "", ""],
        &["3", "100", "200"],
    ]);

    let result = extract(&table, &two_rows(2));

    let read: Vec<_> = result
        .rows
        .iter()
        .map(|r| (r.name.as_str(), r.value))
        .collect();
    assert_eq!(read, vec![("Fund A", dec!(12505)), ("Fund B", dec!(600))]);
    assert!(result.skipped.is_empty());
}

#[test]
fn the_price_unit_divides_the_product() {
    let table = grid(&[&["Fund", "", ""], &["20,000", "0", "12,345"]]);
    let mapping = ColumnMapping {
        price_unit: 10_000,
        ..two_rows(0)
    };

    let result = extract(&table, &mapping);

    assert_eq!(result.rows[0].value, dec!(24690));
}

#[test]
fn an_incomplete_last_record_is_reported() {
    let table = grid(&[&["Fund A", "", ""], &["1", "1", "1"], &["Fund B", "", ""]]);

    let result = extract(&table, &two_rows(0));

    assert_eq!(result.rows.len(), 1);
    assert_eq!(
        result.skipped,
        vec![Skipped {
            row: 3,
            reason: SkipReason::MissingColumn
        }]
    );
}

#[test]
fn a_product_that_overflows_is_out_of_range_not_a_panic() {
    let big = "7".repeat(20);
    let table = grid(&[&["Fund", "", ""], &[big.as_str(), "0", big.as_str()]]);

    let result = extract(&table, &two_rows(0));

    assert!(result.rows.is_empty());
    assert_eq!(result.skipped[0].reason, SkipReason::OutOfRange);
}

#[test]
fn a_non_numeric_quantity_or_price_is_not_a_number() {
    let table = grid(&[
        &["Fund", "", ""],
        &["abc", "0", "5"],
        &["Fund", "", ""],
        &["5", "0", "x"],
    ]);

    let result = extract(&table, &two_rows(0));

    assert!(result.rows.is_empty());
    assert!(
        result
            .skipped
            .iter()
            .all(|s| s.reason == SkipReason::NotANumber)
    );
    assert_eq!(result.skipped.len(), 2);
}

#[test]
fn rows_outside_the_record_are_never_read() {
    // The number row offset is not inside a one-row record.
    let table = grid(&[&["A", "5"]]);
    let mapping = ColumnMapping {
        value_row: 1,
        ..mapping(0, 1, 0)
    };

    let result = extract(&table, &mapping);

    assert!(result.rows.is_empty());
    assert_eq!(result.skipped[0].reason, SkipReason::MissingColumn);
}

#[test]
fn absurd_mappings_neither_overflow_nor_hang() {
    let table = grid(&[&["A", "1"], &["B", "2"], &["C", "3"]]);

    for (header_rows, rows_per_record, name_row, value_row) in [
        (0, usize::MAX, 0, 0),
        (usize::MAX, 1, 0, 0),
        (usize::MAX - 1, usize::MAX, usize::MAX, usize::MAX),
        (0, 1, usize::MAX, usize::MAX),
    ] {
        let mapping = ColumnMapping {
            header_rows,
            rows_per_record,
            name_row,
            value_row,
            ..mapping(0, 1, 0)
        };
        let _ = extract(&table, &mapping);
    }
}
