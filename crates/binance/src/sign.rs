use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

/// Computes the `signature` query value: the lowercase hex HMAC-SHA256, keyed by
/// the API secret, of the query string that precedes it (`totalParams`).
pub fn sign(secret: &str, payload: &str) -> String {
    // HMAC accepts keys of any length, so this cannot fail.
    let mut mac =
        Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("HMAC accepts keys of any length");
    mac.update(payload.as_bytes());
    mac.finalize()
        .into_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // RFC 4231 test case 2 (key "Jefe", data "what do ya want for nothing?").
    #[test]
    fn sign_matches_the_rfc_4231_vector() {
        assert_eq!(
            sign("Jefe", "what do ya want for nothing?"),
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
    }

    #[test]
    fn sign_changes_when_the_secret_or_the_payload_changes() {
        let base = sign("secret", "timestamp=1");

        assert_ne!(base, sign("secret2", "timestamp=1"));
        assert_ne!(base, sign("secret", "timestamp=2"));
    }
}
