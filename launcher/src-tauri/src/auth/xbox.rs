//! Steps 2 & 3: trade the Microsoft access token for an Xbox Live token, then
//! for an XSTS token scoped to Minecraft services.
//!
//! Both calls also return a "user hash" (uhs); Minecraft needs the uhs from the
//! XSTS step paired with the XSTS token.

use serde_json::json;

use super::config;
use super::error::{AuthError, AuthResult};

pub struct XstsResult {
    pub xsts_token: String,
    pub user_hash: String,
}

#[derive(serde::Deserialize)]
struct XboxResponse {
    #[serde(rename = "Token")]
    token: String,
    #[serde(rename = "DisplayClaims")]
    display_claims: DisplayClaims,
}

#[derive(serde::Deserialize)]
struct DisplayClaims {
    xui: Vec<Xui>,
}

#[derive(serde::Deserialize)]
struct Xui {
    uhs: String,
}

#[derive(serde::Deserialize)]
struct XstsError {
    #[serde(rename = "XErr")]
    xerr: i64,
}

/// Step 2: Microsoft access token -> Xbox Live token.
pub async fn authenticate(http: &reqwest::Client, ms_access_token: &str) -> AuthResult<String> {
    let body = json!({
        "Properties": {
            "AuthMethod": "RPS",
            "SiteName": "user.auth.xboxlive.com",
            "RpsTicket": format!("d={ms_access_token}")
        },
        "RelyingParty": "http://auth.xboxlive.com",
        "TokenType": "JWT"
    });

    let resp = http
        .post(config::XBL_AUTH_URL)
        .header("Accept", "application/json")
        .json(&body)
        .send()
        .await?;

    let status = resp.status();
    let text = resp.text().await?;
    if !status.is_success() {
        return Err(AuthError::Xbox(format!("HTTP {status}: {text}")));
    }

    let parsed: XboxResponse = serde_json::from_str(&text).map_err(|e| {
        AuthError::UnexpectedResponse {
            context: "Xbox Live authenticate".into(),
            detail: e.to_string(),
        }
    })?;
    Ok(parsed.token)
}

/// Step 3: Xbox Live token -> XSTS token (+ user hash) for Minecraft.
pub async fn authorize(http: &reqwest::Client, xbl_token: &str) -> AuthResult<XstsResult> {
    let body = json!({
        "Properties": {
            "SandboxId": "RETAIL",
            "UserTokens": [xbl_token]
        },
        "RelyingParty": "rp://api.minecraftservices.com/",
        "TokenType": "JWT"
    });

    let resp = http
        .post(config::XSTS_AUTH_URL)
        .header("Accept", "application/json")
        .json(&body)
        .send()
        .await?;

    let status = resp.status();
    let text = resp.text().await?;

    if !status.is_success() {
        // XSTS encodes specific failures in an XErr code; map the well-known ones.
        if let Ok(err) = serde_json::from_str::<XstsError>(&text) {
            return Err(match err.xerr {
                2148916233 => AuthError::NoXboxAccount,
                2148916238 => AuthError::ChildAccount,
                other => AuthError::Xbox(format!("XSTS denied (XErr {other})")),
            });
        }
        return Err(AuthError::Xbox(format!("HTTP {status}: {text}")));
    }

    let parsed: XboxResponse = serde_json::from_str(&text).map_err(|e| {
        AuthError::UnexpectedResponse {
            context: "XSTS authorize".into(),
            detail: e.to_string(),
        }
    })?;

    let user_hash = parsed
        .display_claims
        .xui
        .into_iter()
        .next()
        .map(|x| x.uhs)
        .ok_or_else(|| AuthError::UnexpectedResponse {
            context: "XSTS authorize".into(),
            detail: "no user hash in response".into(),
        })?;

    Ok(XstsResult {
        xsts_token: parsed.token,
        user_hash,
    })
}
