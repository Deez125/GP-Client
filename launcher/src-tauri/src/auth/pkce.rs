//! PKCE (Proof Key for Code Exchange) — the mechanism that lets a public
//! desktop client do the OAuth code flow safely without a client secret.
//!
//! We make a random `verifier`, send only its SHA-256 hash (`challenge`) when
//! asking for the login, then prove ownership by sending the raw `verifier`
//! when exchanging the returned code for tokens.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::Rng;
use sha2::{Digest, Sha256};

pub struct Pkce {
    pub verifier: String,
    pub challenge: String,
}

impl Pkce {
    pub fn generate() -> Self {
        // 32 random bytes -> 43-char base64url string (within the 43..128 spec range).
        let mut bytes = [0u8; 32];
        rand::thread_rng().fill(&mut bytes);
        let verifier = URL_SAFE_NO_PAD.encode(bytes);

        let digest = Sha256::digest(verifier.as_bytes());
        let challenge = URL_SAFE_NO_PAD.encode(digest);

        Pkce {
            verifier,
            challenge,
        }
    }
}

/// A random URL-safe string, used for the OAuth `state` parameter (CSRF guard).
pub fn random_state() -> String {
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}
