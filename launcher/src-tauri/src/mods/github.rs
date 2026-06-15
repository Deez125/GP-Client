//! List files in a GitHub repo folder via the contents API. Each file object
//! carries a ready-to-use `download_url` (a raw URL), so we never build raw
//! jar URLs by hand.

use serde::Deserialize;

#[derive(Deserialize)]
struct Entry {
    name: String,
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    download_url: Option<String>,
}

/// Return `(filename, download_url)` for every `.jar` file in the folder.
pub async fn list_jars(
    client: &reqwest::Client,
    api_url: &str,
) -> Result<Vec<(String, String)>, String> {
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
                out.push((e.name, url));
            }
        }
    }
    Ok(out)
}
