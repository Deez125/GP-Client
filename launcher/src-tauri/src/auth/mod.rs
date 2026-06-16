//! The Microsoft -> Xbox -> XSTS -> Minecraft auth chain, plus the Tauri
//! commands the UI calls. Built to be verifiable in isolation (the spec's
//! "build and verify this part first" guidance).
//!
//! Public commands:
//!   * `auth_login`        — interactive browser sign-in (full chain).
//!   * `auth_login_silent` — refresh from the stored token, no browser.
//!   * `auth_logout`       — forget the stored refresh token.
//!   * `auth_status`       — is a client ID configured / is a session cached?

mod cache;
mod config;
mod error;
mod minecraft;
mod oauth;
mod pkce;
mod xbox;

pub use error::{AuthError, AuthResult};
pub use minecraft::MinecraftProfile;

use std::sync::Mutex;

use serde::Serialize;

/// In-memory session for this process run. A Ctrl+R reloads only the webview,
/// not the Rust process, so caching the signed-in profile here lets the silent
/// sign-in return instantly after a refresh instead of re-running the (network,
/// rate-limitable) auth chain. Cleared on logout.
static SESSION: Mutex<Option<MinecraftProfile>> = Mutex::new(None);

fn cache_session(profile: &MinecraftProfile) {
    if let Ok(mut guard) = SESSION.lock() {
        *guard = Some(profile.clone());
    }
}

/// Build the HTTP client. A real user-agent keeps the Mojang/Xbox APIs happy.
fn http_client() -> AuthResult<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent("GPClient/0.1 (+launcher)")
        .build()
        .map_err(AuthError::from)
}

/// Run the chain from a set of Microsoft tokens through to a Minecraft profile.
/// Persists the refresh token on success so future launches can go silent.
async fn complete_chain(
    http: &reqwest::Client,
    ms: oauth::MsTokens,
) -> AuthResult<MinecraftProfile> {
    // Save the refresh token first thing (if we got one) so a later failure in
    // the chain doesn't force a fresh interactive login next time.
    if let Some(rt) = &ms.refresh_token {
        cache::save_refresh_token(rt)?;
    }

    let xbl = xbox::authenticate(http, &ms.access_token).await?;
    let xsts = xbox::authorize(http, &xbl).await?;
    let mc_token = minecraft::login_with_xbox(http, &xsts.user_hash, &xsts.xsts_token).await?;
    minecraft::fetch_profile(http, &mc_token).await
}

/// Interactive sign-in: opens the browser, runs the full chain.
pub async fn login() -> AuthResult<MinecraftProfile> {
    let http = http_client()?;
    let ms = oauth::interactive_login(&http).await?;
    let profile = complete_chain(&http, ms).await?;
    cache_session(&profile);
    Ok(profile)
}

/// Silent sign-in. Returns the in-memory session if one exists (survives a
/// Ctrl+R), otherwise refreshes from the stored token. `Ok(None)` if there's
/// nothing stored (caller should then do interactive).
pub async fn login_silent() -> AuthResult<Option<MinecraftProfile>> {
    // Fast path: reuse this run's session (no network → no rate limiting).
    if let Ok(guard) = SESSION.lock() {
        if let Some(profile) = guard.as_ref() {
            return Ok(Some(profile.clone()));
        }
    }
    let Some(refresh_token) = cache::load_refresh_token()? else {
        return Ok(None);
    };
    let http = http_client()?;
    let ms = oauth::refresh(&http, &refresh_token).await?;
    let profile = complete_chain(&http, ms).await?;
    cache_session(&profile);
    Ok(Some(profile))
}

/// Forget the stored refresh token and the in-memory session.
pub fn logout() -> AuthResult<()> {
    if let Ok(mut guard) = SESSION.lock() {
        *guard = None;
    }
    cache::clear_refresh_token()
}

#[derive(Serialize)]
pub struct AuthStatus {
    /// Whether a real Azure client ID has been configured.
    pub client_id_configured: bool,
    /// Whether a refresh token is cached (a previous successful login exists).
    pub has_cached_session: bool,
}

pub fn status() -> AuthResult<AuthStatus> {
    Ok(AuthStatus {
        client_id_configured: !config::client_id_is_placeholder(),
        has_cached_session: cache::load_refresh_token()?.is_some(),
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
pub fn auth_logout() -> AuthResult<()> {
    logout()
}

#[tauri::command]
pub fn auth_status() -> AuthResult<AuthStatus> {
    status()
}
