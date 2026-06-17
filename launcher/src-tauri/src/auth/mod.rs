//! The Microsoft -> Xbox -> XSTS -> Minecraft auth chain, plus the Tauri
//! commands the UI calls. Built to be verifiable in isolation (the spec's
//! "build and verify this part first" guidance).
//!
//! Multi-account: any number of accounts can be signed in at once. Each one's
//! refresh token is stored under its Minecraft UUID; an index records which is
//! active. The active account is the one used for silent sign-in and launching.
//!
//! Public commands:
//!   * `auth_login`          — interactive browser sign-in; ADDS an account and
//!                             makes it active.
//!   * `auth_login_silent`   — refresh the ACTIVE account from its stored token.
//!   * `auth_list_accounts`  — all stored accounts + which is active.
//!   * `auth_switch_account` — make a stored account active (refreshes it).
//!   * `auth_logout`         — remove ONE account (by uuid).
//!   * `auth_status`         — is a client ID configured / any account stored?

mod cache;
mod config;
mod error;
mod minecraft;
mod oauth;
mod pkce;
mod xbox;

pub use error::{AuthError, AuthResult};
pub use minecraft::MinecraftProfile;

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use serde::Serialize;

/// In-memory profiles for this process run, keyed by UUID. A Ctrl+R reloads
/// only the webview, not the Rust process, so caching signed-in profiles here
/// lets silent sign-in / account switching return instantly after a refresh
/// instead of re-running the (network, rate-limitable) auth chain.
fn session_map() -> &'static Mutex<HashMap<String, MinecraftProfile>> {
    static SESSION: OnceLock<Mutex<HashMap<String, MinecraftProfile>>> = OnceLock::new();
    SESSION.get_or_init(|| Mutex::new(HashMap::new()))
}

fn cache_session(profile: &MinecraftProfile) {
    if let Ok(mut m) = session_map().lock() {
        m.insert(profile.uuid.clone(), profile.clone());
    }
}

fn session_get(uuid: &str) -> Option<MinecraftProfile> {
    session_map().lock().ok().and_then(|m| m.get(uuid).cloned())
}

fn session_remove(uuid: &str) {
    if let Ok(mut m) = session_map().lock() {
        m.remove(uuid);
    }
}

/// Build the HTTP client. A real user-agent keeps the Mojang/Xbox APIs happy.
fn http_client() -> AuthResult<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent("GPClient/0.1 (+launcher)")
        .build()
        .map_err(AuthError::from)
}

/// Run the chain from a set of Microsoft tokens through to a Minecraft profile,
/// then persist the (rotated) refresh token under the account's UUID and make
/// that account active. The UUID is only known after the profile call, so the
/// token is saved last; a mid-chain failure just means re-logging in.
async fn complete_chain(
    http: &reqwest::Client,
    ms: oauth::MsTokens,
) -> AuthResult<MinecraftProfile> {
    let xbl = xbox::authenticate(http, &ms.access_token).await?;
    let xsts = xbox::authorize(http, &xbl).await?;
    let mc_token = minecraft::login_with_xbox(http, &xsts.user_hash, &xsts.xsts_token).await?;
    let profile = minecraft::fetch_profile(http, &mc_token).await?;

    if let Some(rt) = &ms.refresh_token {
        cache::save_refresh_token(&profile.uuid, rt)?;
    }
    let mut index = cache::load_index();
    cache::upsert_active(&mut index, &profile.uuid, &profile.username);
    cache::save_index(&index)?;

    Ok(profile)
}

/// Interactive sign-in: opens the browser, runs the full chain, and adds the
/// resulting account (making it active).
pub async fn login() -> AuthResult<MinecraftProfile> {
    let http = http_client()?;
    let ms = oauth::interactive_login(&http).await?;
    let profile = complete_chain(&http, ms).await?;
    cache_session(&profile);
    Ok(profile)
}

/// One-time migration of the pre-multi-account single refresh token into the
/// per-UUID scheme. Best-effort: any failure just leaves the legacy token in
/// place to retry next launch. Runs only when no accounts are indexed yet.
async fn migrate_legacy() {
    if !cache::load_index().accounts.is_empty() {
        return;
    }
    let Ok(Some(rt)) = cache::legacy_refresh_token() else {
        return;
    };
    let Ok(http) = http_client() else { return };
    let Ok(ms) = oauth::refresh(&http, &rt).await else {
        return;
    };
    if let Ok(profile) = complete_chain(&http, ms).await {
        cache_session(&profile);
        let _ = cache::clear_legacy();
    }
}

/// Silent sign-in for the ACTIVE account. Returns the in-memory profile if one
/// exists (survives a Ctrl+R), otherwise refreshes from its stored token.
/// `Ok(None)` if no account is active (caller should then do interactive).
pub async fn login_silent() -> AuthResult<Option<MinecraftProfile>> {
    migrate_legacy().await;

    let Some(active) = cache::load_index().active else {
        return Ok(None);
    };
    // Fast path: reuse this run's profile (no network → no rate limiting).
    if let Some(profile) = session_get(&active) {
        return Ok(Some(profile));
    }
    let Some(refresh_token) = cache::load_refresh_token(&active)? else {
        return Ok(None);
    };
    let http = http_client()?;
    let ms = oauth::refresh(&http, &refresh_token).await?;
    let profile = complete_chain(&http, ms).await?;
    cache_session(&profile);
    Ok(Some(profile))
}

/// All stored accounts + which one is active.
#[derive(Serialize)]
pub struct AccountList {
    pub accounts: Vec<cache::AccountRecord>,
    pub active: Option<String>,
}

pub fn list_accounts() -> AccountList {
    let index = cache::load_index();
    AccountList {
        accounts: index.accounts,
        active: index.active,
    }
}

/// Make a stored account active and return its profile (refreshing if needed).
pub async fn switch_account(uuid: &str) -> AuthResult<MinecraftProfile> {
    // Fast path: already have this run's profile — just flip the active flag.
    if let Some(profile) = session_get(uuid) {
        let mut index = cache::load_index();
        index.active = Some(uuid.to_string());
        cache::save_index(&index)?;
        return Ok(profile);
    }
    let Some(refresh_token) = cache::load_refresh_token(uuid)? else {
        return Err(AuthError::TokenStore("no such account".into()));
    };
    let http = http_client()?;
    let ms = oauth::refresh(&http, &refresh_token).await?;
    // complete_chain stores the rotated token and sets this account active.
    let profile = complete_chain(&http, ms).await?;
    cache_session(&profile);
    Ok(profile)
}

/// Remove ONE account (token + index entry + cached profile). If it was active,
/// the index falls back to another account (or none).
pub fn logout(uuid: &str) -> AuthResult<()> {
    session_remove(uuid);
    let _ = cache::clear_refresh_token(uuid);
    let mut index = cache::load_index();
    cache::remove(&mut index, uuid);
    cache::save_index(&index)
}

#[derive(Serialize)]
pub struct AuthStatus {
    /// Whether a real Azure client ID has been configured.
    pub client_id_configured: bool,
    /// Whether any account is stored (a previous successful login exists).
    pub has_cached_session: bool,
}

pub fn status() -> AuthResult<AuthStatus> {
    Ok(AuthStatus {
        client_id_configured: !config::client_id_is_placeholder(),
        has_cached_session: !cache::load_index().accounts.is_empty(),
    })
}

// --- Tauri command wrappers -------------------------------------------------

#[tauri::command]
pub async fn auth_login() -> AuthResult<MinecraftProfile> {
    login().await
}

#[tauri::command]
pub async fn auth_login_silent() -> AuthResult<Option<MinecraftProfile>> {
    login_silent().await
}

#[tauri::command]
pub fn auth_list_accounts() -> AccountList {
    list_accounts()
}

#[tauri::command]
pub async fn auth_switch_account(uuid: String) -> AuthResult<MinecraftProfile> {
    switch_account(&uuid).await
}

#[tauri::command]
pub fn auth_logout(uuid: String) -> AuthResult<()> {
    logout(&uuid)
}

#[tauri::command]
pub fn auth_status() -> AuthResult<AuthStatus> {
    status()
}
