use std::fmt;

/// An API key and secret pair.
///
/// `Debug` never prints either value, so credentials cannot leak into logs or
/// error messages through formatting. There is intentionally no `Serialize`
/// and no public accessor for the secret.
#[derive(Clone)]
pub struct Credentials {
    api_key: String,
    api_secret: String,
}

impl Credentials {
    pub fn new(api_key: impl Into<String>, api_secret: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            api_secret: api_secret.into(),
        }
    }

    pub(crate) fn api_key(&self) -> &str {
        &self.api_key
    }

    pub(crate) fn api_secret(&self) -> &str {
        &self.api_secret
    }
}

impl fmt::Debug for Credentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Credentials { api_key: <redacted>, api_secret: <redacted> }")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_output_never_contains_the_key_or_the_secret() {
        let credentials = Credentials::new("my-key-123", "my-secret-456");

        let shown = format!("{credentials:?}");

        assert!(!shown.contains("my-key-123"));
        assert!(!shown.contains("my-secret-456"));
    }
}
