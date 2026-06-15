//! Secure storage for the Microsoft **refresh token**.
//!
//! Only the long-lived refresh token is persisted, and it goes into the OS
//! keychain (Windows Credential Manager / macOS Keychain / Linux secret
//! service) via the `keyring` crate — never a plaintext file. The short-lived
//! Minecraft access token is kept in memory only and re-derived on demand.

use keyring::Entry;

use super::config;
use super::error::{AuthError, AuthResult};

fn entry() -> AuthResult<Entry> {
    Entry::new(config::KEYRING_SERVICE, config::KEYRING_ACCOUNT)
        .map_err(|e| AuthError::TokenStore(e.to_string()))
}

/// Persist (or overwrite) the stored refresh token.
pub fn save_refresh_token(token: &str) -> AuthResult<()> {
    entry()?
        .set_password(token)
        .map_err(|e| AuthError::TokenStore(e.to_string()))
}

/// Load the stored refresh token, if any. Returns `Ok(None)` when nothing is
/// stored yet (a first run / signed-out state), which is not an error.
pub fn load_refresh_token() -> AuthResult<Option<String>> {
    match entry()?.get_password() {
        Ok(token) => Ok(Some(token)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(AuthError::TokenStore(e.to_string())),
    }
}

/// Remove the stored refresh token (sign out). No-op if nothing is stored.
pub fn clear_refresh_token() -> AuthResult<()> {
    match entry()?.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(AuthError::TokenStore(e.to_string())),
    }
}
