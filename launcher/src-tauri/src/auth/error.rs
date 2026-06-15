//! Error type for the whole auth chain.
//!
//! It implements `Serialize` so Tauri commands can return it straight to the
//! frontend as `{ "kind": ..., "message": ... }` — the UI can branch on `kind`
//! (e.g. show specific guidance for a Game Pass account with no profile yet)
//! and always has a human-readable `message`.

use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("No Azure client ID is configured. Set GP_CLIENT_AZURE_CLIENT_ID or edit auth/config.rs.")]
    MissingClientId,

    #[error("The login window timed out before sign-in completed.")]
    LoginTimeout,

    #[error("The login was cancelled or returned an error: {0}")]
    OAuthDenied(String),

    #[error("Network/HTTP error: {0}")]
    Http(String),

    #[error("Xbox Live rejected the sign-in: {0}")]
    Xbox(String),

    // The two XSTS error cases worth handling specially. The raw XErr code is
    // kept so we can show tailored guidance.
    #[error("This Microsoft account has no Xbox profile. Sign in once at xbox.com to create one.")]
    NoXboxAccount,
    #[error("This account is a child account and must be added to a Microsoft Family to play.")]
    ChildAccount,

    #[error("Minecraft sign-in failed: {0}")]
    Minecraft(String),

    // Microsoft requires newly-created Azure apps to be approved for the
    // Minecraft API; until then login_with_xbox returns 403. This is an
    // app-registration issue, not a user problem — say so clearly.
    #[error("This launcher's Microsoft app isn't approved for the Minecraft API yet. The Azure app must apply for Minecraft API access and be approved by Microsoft before sign-in can complete.")]
    MinecraftApiNotApproved,

    // The key Game Pass snag the spec calls out: auth succeeds but there's no
    // Java profile yet. We surface this distinctly so the UI can guide the user.
    #[error("This account doesn't own Minecraft: Java Edition, or hasn't set up a Java profile yet. If you have Game Pass, launch Java once through the official launcher to create your profile, then try again.")]
    NoMinecraftProfile,

    #[error("Couldn't read or write the secure token store: {0}")]
    TokenStore(String),

    #[error("Unexpected response from {context}: {detail}")]
    UnexpectedResponse { context: String, detail: String },

    #[error("{0}")]
    Other(String),
}

impl AuthError {
    /// Short machine-readable tag the frontend can switch on.
    pub fn kind(&self) -> &'static str {
        match self {
            AuthError::MissingClientId => "missing_client_id",
            AuthError::LoginTimeout => "login_timeout",
            AuthError::OAuthDenied(_) => "oauth_denied",
            AuthError::Http(_) => "http",
            AuthError::Xbox(_) => "xbox",
            AuthError::NoXboxAccount => "no_xbox_account",
            AuthError::ChildAccount => "child_account",
            AuthError::Minecraft(_) => "minecraft",
            AuthError::MinecraftApiNotApproved => "minecraft_api_not_approved",
            AuthError::NoMinecraftProfile => "no_minecraft_profile",
            AuthError::TokenStore(_) => "token_store",
            AuthError::UnexpectedResponse { .. } => "unexpected_response",
            AuthError::Other(_) => "other",
        }
    }
}

impl Serialize for AuthError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("AuthError", 2)?;
        s.serialize_field("kind", self.kind())?;
        s.serialize_field("message", &self.to_string())?;
        s.end()
    }
}

impl From<reqwest::Error> for AuthError {
    fn from(e: reqwest::Error) -> Self {
        AuthError::Http(e.to_string())
    }
}

pub type AuthResult<T> = Result<T, AuthError>;
