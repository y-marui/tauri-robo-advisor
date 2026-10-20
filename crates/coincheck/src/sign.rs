use hmac::{Hmac, Mac};
use sha2::Sha256;

/// Computes the `ACCESS-SIGNATURE` header value: the lowercase hex HMAC-SHA256,
/// keyed by the API secret, of `nonce + url + body`.
///
/// `url` must be the full URL exactly as sent, including the scheme, host and
/// any query string. A `GET` has no body.
pub fn sign(secret: &str, nonce: &str, url: &str, body: &str) -> String {
    // HMAC accepts keys of any length, so this cannot fail.
    let mut mac =
        Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("HMAC accepts keys of any length");
    mac.update(nonce.as_bytes());
    mac.update(url.as_bytes());
    mac.update(body.as_bytes());
    mac.finalize()
        .into_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Reference value from RFC 4231 test case 2 (key "Jefe", data "what do ya
    // want for nothing?"), split across the three signed parts.
    #[test]
    fn sign_matches_the_rfc_4231_vector() {
        let signature = sign("Jefe", "what do ya ", "want for nothing", "?");

        assert_eq!(
            signature,
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
    }

    #[test]
    fn sign_changes_when_any_part_changes() {
        let url = "https://coincheck.com/api/accounts/balance";
        let base = sign("secret", "1", url, "");

        assert_ne!(base, sign("secret2", "1", url, ""));
        assert_ne!(base, sign("secret", "2", url, ""));
        assert_ne!(
            base,
            sign("secret", "1", "https://coincheck.com/api/ticker", "")
        );
    }
}
