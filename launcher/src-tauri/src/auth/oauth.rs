//! Step 1 of the chain: Microsoft OAuth (authorization code + PKCE).
//!
//! Flow in plain terms:
//!   1. Start a tiny web server on `http://localhost:<random port>`.
//!   2. Open the user's browser at Microsoft's login page, telling it to send
//!      the result back to that local address.
//!   3. The user signs in; Microsoft redirects the browser to our local server
//!      with a one-time `code`.
//!   4. We swap that `code` (plus the PKCE verifier) for an access token and a
//!      long-lived refresh token.

use std::time::{Duration, Instant};

use url::Url;

use super::config;
use super::error::{AuthError, AuthResult};
use super::pkce::{random_state, Pkce};

/// Tokens returned by Microsoft's token endpoint.
#[derive(Debug, Clone)]
pub struct MsTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
}

#[derive(serde::Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
}

#[derive(serde::Deserialize)]
struct TokenErrorResponse {
    error: String,
    #[serde(default)]
    error_description: Option<String>,
}

/// Run the interactive browser login and return Microsoft tokens.
pub async fn interactive_login(http: &reqwest::Client) -> AuthResult<MsTokens> {
    if config::client_id_is_placeholder() {
        return Err(AuthError::MissingClientId);
    }

    let pkce = Pkce::generate();
    let state = random_state();

    // 1. Bind the loopback listener first so we know which port to redirect to.
    //    tiny_http is blocking, so the whole capture runs on a blocking thread.
    let expected_state = state.clone();
    let (redirect_uri_tx, redirect_uri_rx) = tokio::sync::oneshot::channel::<String>();
    let capture = tokio::task::spawn_blocking(move || capture_code(expected_state, redirect_uri_tx));

    // 2. Wait for the server thread to tell us its chosen redirect URI, then
    //    build the authorize URL and open the browser.
    let redirect_uri = redirect_uri_rx
        .await
        .map_err(|_| AuthError::Other("loopback server failed to start".into()))?;

    let authorize_url = build_authorize_url(&redirect_uri, &pkce.challenge, &state);
    if let Err(e) = open::that(&authorize_url) {
        return Err(AuthError::Other(format!(
            "couldn't open the browser for login: {e}"
        )));
    }

    // 3. Wait for the redirect to deliver the code (the blocking task enforces
    //    its own timeout, but guard here too).
    let code = capture
        .await
        .map_err(|e| AuthError::Other(format!("login task panicked: {e}")))??;

    // 4. Exchange the code for tokens.
    exchange_code(http, &code, &redirect_uri, &pkce.verifier).await
}

/// Refresh tokens silently using a stored refresh token (no browser).
pub async fn refresh(http: &reqwest::Client, refresh_token: &str) -> AuthResult<MsTokens> {
    if config::client_id_is_placeholder() {
        return Err(AuthError::MissingClientId);
    }

    let params = [
        ("client_id", config::client_id()),
        ("grant_type", "refresh_token".to_string()),
        ("refresh_token", refresh_token.to_string()),
        ("scope", config::SCOPES.to_string()),
    ];
    post_token(http, &params).await
}

async fn exchange_code(
    http: &reqwest::Client,
    code: &str,
    redirect_uri: &str,
    verifier: &str,
) -> AuthResult<MsTokens> {
    let params = [
        ("client_id", config::client_id()),
        ("grant_type", "authorization_code".to_string()),
        ("code", code.to_string()),
        ("redirect_uri", redirect_uri.to_string()),
        ("code_verifier", verifier.to_string()),
    ];
    post_token(http, &params).await
}

async fn post_token(http: &reqwest::Client, params: &[(&str, String)]) -> AuthResult<MsTokens> {
    let resp = http.post(config::MS_TOKEN_URL).form(params).send().await?;
    let status = resp.status();
    let body = resp.text().await?;

    if !status.is_success() {
        // Microsoft returns a JSON error object; surface its description.
        let msg = serde_json::from_str::<TokenErrorResponse>(&body)
            .map(|e| e.error_description.unwrap_or(e.error))
            .unwrap_or_else(|_| body.clone());
        return Err(AuthError::OAuthDenied(msg));
    }

    let parsed: TokenResponse = serde_json::from_str(&body).map_err(|e| {
        AuthError::UnexpectedResponse {
            context: "Microsoft token endpoint".into(),
            detail: e.to_string(),
        }
    })?;

    Ok(MsTokens {
        access_token: parsed.access_token,
        refresh_token: parsed.refresh_token,
    })
}

fn build_authorize_url(redirect_uri: &str, challenge: &str, state: &str) -> String {
    let mut url = Url::parse(config::MS_AUTHORIZE_URL).expect("authorize URL is valid");
    url.query_pairs_mut()
        .append_pair("client_id", &config::client_id())
        .append_pair("response_type", "code")
        .append_pair("redirect_uri", redirect_uri)
        .append_pair("response_mode", "query")
        .append_pair("scope", config::SCOPES)
        .append_pair("state", state)
        .append_pair("code_challenge", challenge)
        .append_pair("code_challenge_method", "S256")
        // Always show the account picker so users can switch accounts.
        .append_pair("prompt", "select_account");
    url.to_string()
}

/// Blocking: bind a localhost port, report the redirect URI back, then wait for
/// the browser redirect carrying `?code=...&state=...`. Returns the code.
fn capture_code(
    expected_state: String,
    redirect_uri_tx: tokio::sync::oneshot::Sender<String>,
) -> AuthResult<String> {
    let server = tiny_http::Server::http("127.0.0.1:0")
        .map_err(|e| AuthError::Other(format!("couldn't start loopback server: {e}")))?;

    let port = server
        .server_addr()
        .to_ip()
        .map(|a| a.port())
        .ok_or_else(|| AuthError::Other("loopback server has no port".into()))?;
    let redirect_uri = format!("http://localhost:{port}");

    // Hand the redirect URI to the async side so it can build the authorize URL.
    let _ = redirect_uri_tx.send(redirect_uri.clone());

    let deadline = Instant::now() + Duration::from_secs(config::LOGIN_TIMEOUT_SECS);
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(AuthError::LoginTimeout);
        }

        match server.recv_timeout(remaining) {
            Ok(Some(request)) => {
                // The redirect path looks like `/?code=...&state=...`.
                let full = format!("http://localhost{}", request.url());
                let parsed = Url::parse(&full).ok();

                let mut code: Option<String> = None;
                let mut state: Option<String> = None;
                let mut oauth_error: Option<String> = None;
                if let Some(u) = &parsed {
                    for (k, v) in u.query_pairs() {
                        match k.as_ref() {
                            "code" => code = Some(v.into_owned()),
                            "state" => state = Some(v.into_owned()),
                            "error_description" => oauth_error = Some(v.into_owned()),
                            "error" if oauth_error.is_none() => {
                                oauth_error = Some(v.into_owned())
                            }
                            _ => {}
                        }
                    }
                }

                // Respond to the browser so the user sees a friendly page.
                let (page, result): (&str, AuthResult<String>) = if let Some(err) = oauth_error {
                    ("Sign-in failed. You can close this tab.", Err(AuthError::OAuthDenied(err)))
                } else if state.as_deref() != Some(expected_state.as_str()) {
                    (
                        "Sign-in could not be verified. You can close this tab.",
                        Err(AuthError::OAuthDenied("state mismatch (possible CSRF)".into())),
                    )
                } else if let Some(c) = code {
                    ("Signed in to GP Client. You can close this tab and return to the launcher.", Ok(c))
                } else {
                    // Not the redirect we care about (e.g. favicon) — keep waiting.
                    let _ = request.respond(tiny_http::Response::from_string("GP Client"));
                    continue;
                };

                let html = format!(
                    "<!doctype html><html><head><meta charset=utf-8><title>GP Client</title></head>\
                     <body style=\"font-family:sans-serif;text-align:center;padding-top:3em\">\
                     <h2>{page}</h2></body></html>"
                );
                let response = tiny_http::Response::from_string(html).with_header(
                    "Content-Type: text/html; charset=utf-8"
                        .parse::<tiny_http::Header>()
                        .unwrap(),
                );
                let _ = request.respond(response);
                return result;
            }
            Ok(None) => return Err(AuthError::LoginTimeout),
            Err(e) => return Err(AuthError::Other(format!("loopback server error: {e}"))),
        }
    }
}
