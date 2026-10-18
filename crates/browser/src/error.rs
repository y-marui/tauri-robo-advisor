/// Why the browser could not be found, started, or read. Messages never repeat
/// page contents.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("no Chrome, Edge, or Brave was found")]
    NoBrowser,
    #[error("the browser could not be started: {0}")]
    Launch(String),
    #[error("the browser did not open its debugging connection in time")]
    NotReady,
    #[error("the connection to the browser failed: {0}")]
    Connection(String),
    #[error("the browser did not answer in time")]
    Timeout,
    #[error("the browser reported an error: {0}")]
    Protocol(String),
    #[error("the page could not be read")]
    Script,
    #[error("no page is open")]
    NoPage,
    #[error("no link with that text was found on the page")]
    LinkNotFound,
    #[error("that link is not one the app may follow")]
    LinkRefused,
    #[error("the link has changed since it was saved; choose it again")]
    LinkChanged,
    #[error("several different links have that text; choose one")]
    LinkAmbiguous(Vec<LinkChoice>),
}

/// One of several links that have the same text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkChoice {
    /// What to save to find this link again.
    pub reference: String,
    /// Where it goes, in words, to show to the user (never saved).
    pub label: String,
}
