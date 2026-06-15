//! Download helper: fetch files only when missing or hash-mismatched, verifying
//! Mojang's SHA-1 hashes. Used for the client jar, libraries, assets, and later
//! reused for mod sync.

use std::path::Path;

use sha1::{Digest, Sha1};

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

    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("GET {url}: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("GET {url}: HTTP {}", resp.status()));
    }
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("read body {url}: {e}"))?;

    // Verify before writing into place.
    let mut hasher = Sha1::new();
    hasher.update(&bytes);
    let actual = hex(&hasher.finalize());
    if !actual.eq_ignore_ascii_case(expected_sha1) {
        return Err(format!(
            "hash mismatch for {url}: expected {expected_sha1}, got {actual}"
        ));
    }

    std::fs::write(dest, &bytes).map_err(|e| format!("write {dest:?}: {e}"))?;
    Ok(())
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
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("GET {url}: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("GET {url}: HTTP {}", resp.status()));
    }
    let bytes = resp.bytes().await.map_err(|e| format!("read {url}: {e}"))?;
    std::fs::write(dest, &bytes).map_err(|e| format!("write {dest:?}: {e}"))?;
    Ok(())
}
