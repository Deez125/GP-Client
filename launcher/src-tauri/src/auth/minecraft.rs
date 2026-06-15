//! Steps 4 & 5: XSTS token -> Minecraft access token, then fetch the player
//! profile (UUID + username).
//!
//! The profile call is also our ownership check. A 404 here is the classic
//! Game Pass situation the spec warns about: the account is valid but has no
//! Java profile yet — we report that as a distinct, actionable error rather
//! than a generic failure.

use serde_json::json;

use super::config;
use super::error::{AuthError, AuthResult};

/// The Minecraft session: token to launch with, plus who you are.
#[derive(Debug, Clone, serde::Serialize)]
pub struct MinecraftProfile {
    /// Minecraft access token (used at launch and for profile APIs).
    pub access_token: String,
    /// Player UUID (no dashes, as Mojang returns it).
    pub uuid: String,
    /// Player username.
    pub username: String,
}

#[derive(serde::Deserialize)]
struct McLoginResponse {
    access_token: String,
}

#[derive(serde::Deserialize)]
struct McProfileResponse {
    id: String,
    name: String,
}

/// Step 4: XSTS token (+ user hash) -> Minecraft access token.
pub async fn login_with_xbox(
    http: &reqwest::Client,
    user_hash: &str,
    xsts_token: &str,
) -> AuthResult<String> {
    let body = json!({
        "identityToken": format!("XBL3.0 x={user_hash};{xsts_token}")
    });

    let resp = http
        .post(config::MC_LOGIN_URL)
        .header("Accept", "application/json")
        .json(&body)
        .send()
        .await?;

    let status = resp.status();
    let text = resp.text().await?;
    if !status.is_success() {
        // 403 here almost always means the Azure app hasn't been approved for
        // the Minecraft API yet (everything upstream succeeded).
        if status.as_u16() == 403 {
            return Err(AuthError::MinecraftApiNotApproved);
        }
        return Err(AuthError::Minecraft(format!("HTTP {status}: {text}")));
    }

    let parsed: McLoginResponse = serde_json::from_str(&text).map_err(|e| {
        AuthError::UnexpectedResponse {
            context: "Minecraft login_with_xbox".into(),
            detail: e.to_string(),
        }
    })?;
    Ok(parsed.access_token)
}

/// Step 5: fetch the player's Minecraft profile. Doubles as the ownership check.
pub async fn fetch_profile(
    http: &reqwest::Client,
    mc_access_token: &str,
) -> AuthResult<MinecraftProfile> {
    let resp = http
        .get(config::MC_PROFILE_URL)
        .bearer_auth(mc_access_token)
        .send()
        .await?;

    let status = resp.status();

    // 404 (and sometimes 401) here means: authenticated, but no Java profile —
    // the Game Pass first-time-setup snag. Report it distinctly.
    if status.as_u16() == 404 {
        return Err(AuthError::NoMinecraftProfile);
    }
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        // A missing-profile body can also come back with other codes; detect it.
        if text.contains("NOT_FOUND") {
            return Err(AuthError::NoMinecraftProfile);
        }
        return Err(AuthError::Minecraft(format!("profile HTTP {status}: {text}")));
    }

    let parsed: McProfileResponse = resp.json().await.map_err(|e| {
        AuthError::UnexpectedResponse {
            context: "Minecraft profile".into(),
            detail: e.to_string(),
        }
    })?;

    Ok(MinecraftProfile {
        access_token: mc_access_token.to_string(),
        uuid: parsed.id,
        username: parsed.name,
    })
}
