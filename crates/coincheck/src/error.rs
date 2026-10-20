/// Failures while talking to Coincheck.
///
/// No variant carries credentials, signatures, or request headers, so the
/// formatted message is safe to show to the user and to log.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("network error: {0}")]
    Network(String),
    #[error("Coincheck returned HTTP {status}: {message}")]
    Http { status: u16, message: String },
    #[error("the response was larger than the allowed limit")]
    ResponseTooLarge,
    #[error("unexpected response format: {0}")]
    Format(String),
    #[error("an amount is too large to calculate exactly")]
    Overflow,
}
