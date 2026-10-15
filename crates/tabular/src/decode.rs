use encoding_rs::SHIFT_JIS;
use serde::Serialize;

use crate::Error;

/// How the bytes were turned into text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Encoding {
    Utf8,
    ShiftJis,
}

/// Turns the bytes of a file into text. UTF-8 is tried first (with or without a
/// byte order mark), then Shift-JIS, which is what Japanese brokerages commonly
/// export. Text that is valid in neither is refused rather than guessed at, so
/// garbled characters never reach the data.
pub fn decode(bytes: &[u8]) -> Result<(String, Encoding), Error> {
    if bytes.is_empty() {
        return Err(Error::Empty);
    }
    if bytes.len() > crate::MAX_INPUT_BYTES {
        return Err(Error::TooLarge);
    }
    // A NUL byte means a binary file (or UTF-16), not a table.
    if bytes.contains(&0) {
        return Err(Error::NotText);
    }
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    if let Ok(text) = std::str::from_utf8(bytes) {
        return Ok((text.to_owned(), Encoding::Utf8));
    }
    SHIFT_JIS
        .decode_without_bom_handling_and_without_replacement(bytes)
        .map(|text| (text.into_owned(), Encoding::ShiftJis))
        .ok_or(Error::Encoding)
}
