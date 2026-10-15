use hmac::{Hmac, Mac};
use sha2::Sha256;

/// Computes the `ACCESS-SIGN` header value: the lowercase hex HMAC-SHA256, keyed
/// by the API secret, of `timestamp + method + path_and_query + body`.
///
/// `path_and_query` must be exactly what is sent on the wire, including the
/// leading `/` and any query string.
pub fn sign(
    secret: &str,
    timestamp: &str,
    method: &str,
    path_and_query: &str,
    body: &str,
) -> String {
    // HMAC accepts keys of any length, so this cannot fail.
    let mut mac =
        Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("HMAC accepts keys of any length");
    mac.update(timestamp.as_bytes());
    mac.update(method.as_bytes());
    mac.update(path_and_query.as_bytes());
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
    // want for nothing?"), split across the four signed parts.
    #[test]
    fn sign_matches_the_rfc_4231_vector() {
        let signature = sign("Jefe", "what do ya ", "want for ", "nothing", "?");

        assert_eq!(
            signature,
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
    }

    #[test]
    fn sign_changes_when_any_part_changes() {
        let base = sign("secret", "1", "GET", "/v1/me/getbalance", "");

        assert_ne!(base, sign("secret2", "1", "GET", "/v1/me/getbalance", ""));
        assert_ne!(base, sign("secret", "2", "GET", "/v1/me/getbalance", ""));
        assert_ne!(base, sign("secret", "1", "POST", "/v1/me/getbalance", ""));
        assert_ne!(base, sign("secret", "1", "GET", "/v1/me/getcollateral", ""));
    }
}
