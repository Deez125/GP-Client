//! Secure storage for Microsoft **refresh tokens**, now multi-account.
//!
//! Each signed-in account's long-lived refresh token goes into the OS keychain
//! (Windows Credential Manager / macOS Keychain / Linux secret service) keyed by
//! the account's Minecraft UUID — never a plaintext file. A small, non-secret
//! index file (`accounts.json`) records which accounts exist and which one is
//! active, so we can list/switch accounts without unlocking every token.
//!
//! Short-lived Minecraft access tokens are kept in memory only (see `mod.rs`).

use keyring::Entry;
use serde::{Deserialize, Serialize};

use crate::installations::shared_root;

use super::config;
use super::error::{AuthError, AuthResult};

/// One stored account's public (non-secret) info. The refresh token itself
/// lives in the keychain, keyed by `uuid`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountRecord {
    pub uuid: String,
    pub username: String,
}

/// The on-disk account index: which accounts exist and which is active.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AccountIndex {
    #[serde(default)]
    pub accounts: Vec<AccountRecord>,
    #[serde(default)]
    pub active: Option<String>,
}

fn index_path() -> std::path::PathBuf {
    shared_root().join("accounts.json")
}

/// Read the account index, falling back to empty if missing/corrupt.
pub fn load_index() -> AccountIndex {
    match std::fs::read_to_string(index_path()) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
        Err(_) => AccountIndex::default(),
    }
}

/// Persist the account index, creating the shared folder if needed.
pub fn save_index(index: &AccountIndex) -> AuthResult<()> {
    let path = index_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| AuthError::TokenStore(format!("create accounts dir: {e}")))?;
    }
    let text =
        serde_json::to_string_pretty(index).map_err(|e| AuthError::TokenStore(e.to_string()))?;
    std::fs::write(&path, text).map_err(|e| AuthError::TokenStore(format!("write accounts: {e}")))
}

/// Add or update an account in the index and make it the active one.
pub fn upsert_active(index: &mut AccountIndex, uuid: &str, username: &str) {
    if let Some(rec) = index.accounts.iter_mut().find(|a| a.uuid == uuid) {
        rec.username = username.to_string();
    } else {
        index.accounts.push(AccountRecord {
            uuid: uuid.to_string(),
            username: username.to_string(),
        });
    }
    index.active = Some(uuid.to_string());
}

/// Remove an account from the index; if it was active, fall back to the first
/// remaining account (or none).
pub fn remove(index: &mut AccountIndex, uuid: &str) {
    index.accounts.retain(|a| a.uuid != uuid);
    if index.active.as_deref() == Some(uuid) {
        index.active = index.accounts.first().map(|a| a.uuid.clone());
    }
}

// --- per-account refresh tokens (keychain) ----------------------------------

fn entry(uuid: &str) -> AuthResult<Entry> {
    Entry::new(config::KEYRING_SERVICE, uuid).map_err(|e| AuthError::TokenStore(e.to_string()))
}

/// Persist (or overwrite) the refresh token for one account.
pub fn save_refresh_token(uuid: &str, token: &str) -> AuthResult<()> {
    entry(uuid)?
        .set_password(token)
        .map_err(|e| AuthError::TokenStore(e.to_string()))
}

/// Load an account's refresh token, if any.
pub fn load_refresh_token(uuid: &str) -> AuthResult<Option<String>> {
    match entry(uuid)?.get_password() {
        Ok(token) => Ok(Some(token)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(AuthError::TokenStore(e.to_string())),
    }
}

/// Remove an account's refresh token. No-op if nothing is stored.
pub fn clear_refresh_token(uuid: &str) -> AuthResult<()> {
    match entry(uuid)?.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(AuthError::TokenStore(e.to_string())),
    }
}

// --- legacy single-token migration ------------------------------------------
// Pre-multi-account builds stored one refresh token under a fixed account key.
// We migrate it into the per-UUID scheme on first silent sign-in.

fn legacy_entry() -> AuthResult<Entry> {
    Entry::new(config::KEYRING_SERVICE, config::KEYRING_ACCOUNT)
        .map_err(|e| AuthError::TokenStore(e.to_string()))
}

/// The pre-multi-account single refresh token, if still present.
pub fn legacy_refresh_token() -> AuthResult<Option<String>> {
    match legacy_entry()?.get_password() {
        Ok(token) => Ok(Some(token)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(AuthError::TokenStore(e.to_string())),
    }
}

/// Delete the legacy single-token entry (after a successful migration).
pub fn clear_legacy() -> AuthResult<()> {
    let _ = legacy_entry()?.delete_credential();
    Ok(())
}
