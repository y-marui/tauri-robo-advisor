/// Why a table could not be read. Messages never repeat the data itself.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("the input is empty")]
    Empty,
    #[error("the input is too large")]
    TooLarge,
    #[error("the input is not text")]
    NotText,
    #[error("the text is neither UTF-8 nor Shift-JIS")]
    Encoding,
    #[error("a quoted field is not closed (it starts on line {0})")]
    Malformed(usize),
}
