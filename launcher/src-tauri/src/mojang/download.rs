//! Download helper: fetch files only when missing or hash-mismatched, verifying
//! Mojang's SHA-1 hashes. Used for the client jar, libraries, assets, and later
//! reused for mod sync.
//!
//! Mojang's CDNs occasionally drop a connection mid-transfer; rather than fail
//! the whole launch over one flaky request, downloads retry transient errors
//! (dropped connections, timeouts, 5xx/429) with a short exponential backoff.

use std::path::Path;
use std::time::Duration;

use sha1::{Digest, Sha1};

/// How many times to attempt a single download before giving up.
const MAX_ATTEMPTS: u32 = 4;

/// GET `url` and return the body bytes, retrying transient failures with a short
/// backoff. Transport errors (dropped connection, timeout), 5xx, 408 and 429 are
/// retried; other non-success statuses (e.g. 404) fail immediately.
async fn get_bytes(client: &reqwest::Client, url: &str) -> Result<Vec<u8>, String> {
    let mut attempt = 0u32;
    loop {
        attempt += 1;
        // (error message, retryable?)
        let outcome: Result<Vec<u8>, (String, bool)> = async {
            let resp = client
                .get(url)
                .send()
                .await
                .map_err(|e| (format!("GET {url}: {e}"), true))?;
            let status = resp.status();
            if !status.is_success() {
                let retryable = status.is_server_error()
                    || status.as_u16() == 408
                    || status.as_u16() == 429;
                return Err((format!("GET {url}: HTTP {status}"), retryable));
            }
            let bytes = resp
                .bytes()
                .await
                .map_err(|e| (format!("read body {url}: {e}"), true))?;
            Ok(bytes.to_vec())
        }
        .await;

        match outcome {
            Ok(bytes) => return Ok(bytes),
            Err((msg, retryable)) => {
                if retryable && attempt < MAX_ATTEMPTS {
                    // 250ms, 500ms, 1000ms…
                    let delay = Duration::from_millis(250u64 * (1u64 << (attempt - 1)));
                    tokio::time::sleep(delay).await;
                    continue;
                }
                return Err(msg);
            }
        }
    }
}

/// Compute the SHA-1 of a file as a lowercase hex string, if it exists.
pub fn file_sha1(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    let mut hasher = Sha1::new();
    hasher.update(&bytes);
    Some(hex(&hasher.finalize()))
}

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// True if `path` exists and its SHA-1 matches `expected` (case-insensitive).
pub fn verified(path: &Path, expected: &str) -> bool {
    match file_sha1(path) {
        Some(actual) => actual.eq_ignore_ascii_case(expected),
        None => false,
    }
}

/// Download `url` to `dest` unless it already exists with the expected SHA-1.
/// Verifies the hash after downloading.
pub async fn fetch(
    client: &reqwest::Client,
    url: &str,
    dest: &Path,
    expected_sha1: &str,
) -> Result<(), String> {
    if verified(dest, expected_sha1) {
        return Ok(());
    }
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir {parent:?}: {e}"))?;
    }

    // A truncated transfer yields a hash mismatch; re-download a few times
    // before giving up (each attempt's transport errors are retried inside).
    let mut last_err = String::new();
    for attempt in 1..=MAX_ATTEMPTS {
        let bytes = get_bytes(client, url).await?;

        // Verify before writing into place.
        let mut hasher = Sha1::new();
        hasher.update(&bytes);
        let actual = hex(&hasher.finalize());
        if actual.eq_ignore_ascii_case(expected_sha1) {
            return std::fs::write(dest, &bytes).map_err(|e| format!("write {dest:?}: {e}"));
        }

        last_err = format!("hash mismatch for {url}: expected {expected_sha1}, got {actual}");
        if attempt < MAX_ATTEMPTS {
            tokio::time::sleep(Duration::from_millis(250u64 * (1u64 << (attempt - 1)))).await;
        }
    }
    Err(last_err)
}

/// Like [`fetch`], but the SHA-1 is optional. When `None` (e.g. Fabric maven
/// artifacts that don't publish a hash inline), the file is downloaded if
/// missing and accepted without verification.
pub async fn fetch_maybe(
    client: &reqwest::Client,
    url: &str,
    dest: &Path,
    expected_sha1: Option<&str>,
) -> Result<(), String> {
    if let Some(sha1) = expected_sha1 {
        return fetch(client, url, dest, sha1).await;
    }
    if dest.exists() {
        return Ok(());
    }
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir {parent:?}: {e}"))?;
    }
    let bytes = get_bytes(client, url).await?;
    std::fs::write(dest, &bytes).map_err(|e| format!("write {dest:?}: {e}"))?;
    Ok(())
}
