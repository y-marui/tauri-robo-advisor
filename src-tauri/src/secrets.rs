//! API credentials in the operating system's keychain.
//!
//! The key and secret live only in the OS credential store (Keychain Services,
//! Windows Credential Manager, or the Secret Service). They are never written to
//! files, settings, or logs, and nothing here returns them to the frontend.

use bitflyer::Credentials;
use keyring::Entry;

// Not the app identifier: this name only labels the keychain entry. It is kept as it was so that
// a key saved earlier is still found after the identifier changed.
const SERVICE: &str = "dev.marui.robo-advisor";
const BITFLYER_KEY: &str = "bitflyer-api-key";
const BITFLYER_SECRET: &str = "bitflyer-api-secret";

/// Both parts are pasted by hand, so reject anything that could not be a real
/// bitFlyer key: empty, very long, or containing whitespace or control
/// characters (which would also be invalid in an HTTP header).
fn validate(label: &str, value: &str) -> Result<(), String> {
    let valid = (1..=128).contains(&value.len()) && value.bytes().all(|b| b.is_ascii_graphic());
    if valid {
        Ok(())
    } else {
        Err(format!(
            "The {label} must be 1 to 128 visible ASCII characters with no spaces."
        ))
    }
}

fn entry(user: &str) -> Result<Entry, String> {
    Entry::new(SERVICE, user).map_err(|e| format!("The OS keychain is unavailable: {e}"))
}

fn read(user: &str) -> Result<Option<String>, String> {
    match entry(user)?.get_password() {
        Ok(value) => Ok(Some(value)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("Could not read from the OS keychain: {e}")),
    }
}

fn remove(user: &str) -> Result<(), String> {
    match entry(user)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("Could not delete from the OS keychain: {e}")),
    }
}

pub fn save_bitflyer(api_key: &str, api_secret: &str) -> Result<(), String> {
    validate("API key", api_key)?;
    validate("API secret", api_secret)?;
    let key = entry(BITFLYER_KEY)?;
    let secret = entry(BITFLYER_SECRET)?;
    key.set_password(api_key)
        .and_then(|()| secret.set_password(api_secret))
        .map_err(|e| format!("Could not save to the OS keychain: {e}"))
}

pub fn load_bitflyer() -> Result<Option<Credentials>, String> {
    match (read(BITFLYER_KEY)?, read(BITFLYER_SECRET)?) {
        (Some(key), Some(secret)) => Ok(Some(Credentials::new(key, secret))),
        _ => Ok(None),
    }
}

pub fn delete_bitflyer() -> Result<(), String> {
    remove(BITFLYER_KEY)?;
    remove(BITFLYER_SECRET)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_accepts_typical_keys() {
        assert!(validate("API key", "AbC123xyz").is_ok());
        assert!(validate("API secret", &"a".repeat(128)).is_ok());
    }

    #[test]
    fn validate_rejects_empty_long_and_unprintable_values() {
        for value in ["", " ", "has space", "tab\there", "line\nbreak", "日本語"] {
            assert!(validate("API key", value).is_err(), "{value:?}");
        }
        assert!(validate("API key", &"a".repeat(129)).is_err());
    }

    #[test]
    fn validate_errors_never_echo_the_value() {
        let message = validate("API secret", "bad value").unwrap_err();

        assert!(!message.contains("bad value"));
    }

    // Uses the real OS keychain, so it is skipped by default. Run it on a
    // desktop with `cargo test -p tauri-robo-advisor -- --ignored`. It stores
    // and removes dummy values only.
    #[test]
    #[ignore = "touches the real OS keychain"]
    fn keychain_round_trip_with_dummy_values() {
        delete_bitflyer().unwrap();
        assert!(load_bitflyer().unwrap().is_none());

        save_bitflyer("dummy-key", "dummy-secret").unwrap();
        assert!(load_bitflyer().unwrap().is_some());

        delete_bitflyer().unwrap();
        assert!(load_bitflyer().unwrap().is_none());
    }
}
