//! List files in a GitHub repo folder via the contents API. Each file object
//! carries a ready-to-use `download_url` (a raw URL), so we never build raw
//! jar URLs by hand.

use serde::Deserialize;

/// A `.jar` in a GitHub repo folder. `sha` is the git blob SHA-1 (what the
/// contents API reports) — used to detect content changes even when the
/// filename stays the same.
#[derive(Clone)]
pub struct RemoteJar {
    pub name: String,
    pub url: String,
    pub sha: String,
}

#[derive(Deserialize)]
struct Entry {
    name: String,
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    sha: String,
    #[serde(default)]
    download_url: Option<String>,
}

/// Return a `RemoteJar` for every `.jar` file in the folder.
pub async fn list_jars(
    client: &reqwest::Client,
    api_url: &str,
) -> Result<Vec<RemoteJar>, String> {
    let resp = client
        .get(api_url)
        .send()
        .await
        .map_err(|e| format!("GitHub listing: {e}"))?;
    let status = resp.status();
    match status.as_u16() {
        404 => return Err("mods folder not found on GitHub for this version".into()),
        403 => {
            return Err(
                "GitHub API rate limit reached (60 requests/hour). Try again shortly.".into(),
            )
        }
        _ if !status.is_success() => {
            return Err(format!("GitHub listing failed (HTTP {status})"))
        }
        _ => {}
    }

    let entries: Vec<Entry> = resp
        .json()
        .await
        .map_err(|e| format!("parse GitHub listing: {e}"))?;

    let mut out = Vec::new();
    for e in entries {
        if e.kind == "file" && e.name.to_lowercase().ends_with(".jar") {
            if let Some(url) = e.download_url {
                out.push(RemoteJar {
                    name: e.name,
                    url,
                    sha: e.sha,
                });
            }
        }
    }
    Ok(out)
}
