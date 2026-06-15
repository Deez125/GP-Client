//! Static configuration for the Microsoft auth chain.
//!
//! The only value you must supply is an Azure **application (client) ID** for a
//! public client registered as "Mobile and desktop applications" with the
//! redirect URI `http://localhost` (the loopback exception lets us use any
//! port). Scope must include `XboxLive.signin offline_access`.
//!
//! The client ID is read from the `GP_CLIENT_AZURE_CLIENT_ID` environment
//! variable at runtime if set, otherwise it falls back to `DEFAULT_CLIENT_ID`
//! below. A public-client ID is not a secret (PKCE replaces the secret), so it
//! is fine to keep here. To use a different app, change this one line or set
//! the env var.

/// The launcher's Azure application (client) ID. Public client, not a secret.
const DEFAULT_CLIENT_ID: &str = "5649f31d-b217-4011-b6c6-3543e4e6baaf";

/// Sentinel meaning "no client ID configured" — used to disable login in the UI
/// if the default is ever blanked out.
const UNSET_CLIENT_ID: &str = "PUT_YOUR_AZURE_CLIENT_ID_HERE";

/// Returns the configured Azure client ID (env override wins).
pub fn client_id() -> String {
    std::env::var("GP_CLIENT_AZURE_CLIENT_ID")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_CLIENT_ID.to_string())
}

/// True when no real client ID has been provided yet.
pub fn client_id_is_placeholder() -> bool {
    let id = client_id();
    id == UNSET_CLIENT_ID || id.trim().is_empty()
}

/// OAuth scopes. `offline_access` is what gets us a refresh token.
pub const SCOPES: &str = "XboxLive.signin offline_access";

// Microsoft identity platform (personal / "consumers" accounts — this covers
// both purchased and Game Pass Minecraft accounts).
pub const MS_AUTHORIZE_URL: &str =
    "https://login.microsoftonline.com/consumers/oauth2/v2.0/authorize";
pub const MS_TOKEN_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";

// Xbox Live / XSTS.
pub const XBL_AUTH_URL: &str = "https://user.auth.xboxlive.com/user/authenticate";
pub const XSTS_AUTH_URL: &str = "https://xsts.auth.xboxlive.com/xsts/authorize";

// Minecraft services.
pub const MC_LOGIN_URL: &str = "https://api.minecraftservices.com/authentication/login_with_xbox";
pub const MC_PROFILE_URL: &str = "https://api.minecraftservices.com/minecraft/profile";
// Reserved for an explicit entitlements check if the profile call proves
// insufficient for some Game Pass accounts.
#[allow(dead_code)]
pub const MC_ENTITLEMENTS_URL: &str = "https://api.minecraftservices.com/entitlements/mcstore";

/// How long to wait for the user to finish the browser login before giving up.
pub const LOGIN_TIMEOUT_SECS: u64 = 300;

// Keychain identifiers for the cached refresh token.
pub const KEYRING_SERVICE: &str = "gg.gpclient.launcher";
pub const KEYRING_ACCOUNT: &str = "microsoft-refresh-token";
