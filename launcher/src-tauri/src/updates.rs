//! Update check: compare the running version against the newest GitHub release.
//! No hosted manifest — GitHub's releases API provides the version (tag) and the
//! installer download URL. Includes prereleases (newest wins).

use serde::Deserialize;

const RELEASES_API: &str = "https://api.github.com/repos/Deez125/GP-Client/releases";

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

/// Result of an update check, for the UI.
#[derive(serde::Serialize)]
pub struct UpdateInfo {
    /// The running app version (e.g. "0.1.2").
    pub current: String,
    /// The newest release version (tag without a leading "v").
    pub latest: String,
    /// True when `latest` is newer than `current`.
    pub available: bool,
    /// Direct download URL for the newest installer (.exe), if found.
    pub url: Option<String>,
}

/// Parse a version/tag into comparable numeric parts ("v0.1.3" -> [0,1,3]).
fn version_key(v: &str) -> Vec<u32> {
    v.trim_start_matches(['v', 'V'])
        .split(['.', '-', '+'])
        .map(|chunk| {
            let digits: String = chunk.chars().take_while(|c| c.is_ascii_digit()).collect();
            digits.parse().unwrap_or(0)
        })
        .collect()
}

#[tauri::command]
pub async fn check_for_update() -> Result<UpdateInfo, String> {
    let current = env!("CARGO_PKG_VERSION").to_string();

    let client = reqwest::Client::builder()
        .user_agent("GPClient/0.1 (+launcher)")
        .build()
        .map_err(|e| e.to_string())?;

    let releases: Vec<Release> = client
        .get(RELEASES_API)
        .send()
        .await
        .map_err(|e| format!("fetch releases: {e}"))?
        .json()
        .await
        .map_err(|e| format!("parse releases: {e}"))?;

    // Pick the newest non-draft release (prereleases included).
    let mut best: Option<(&Release, Vec<u32>)> = None;
    for r in &releases {
        if r.draft {
            continue;
        }
        let key = version_key(&r.tag_name);
        if best.as_ref().map(|(_, k)| key > *k).unwrap_or(true) {
            best = Some((r, key));
        }
    }

    let Some((rel, latest_key)) = best else {
        // No releases yet — nothing to update to.
        return Ok(UpdateInfo {
            current: current.clone(),
            latest: current,
            available: false,
            url: None,
        });
    };

    let latest = rel.tag_name.trim_start_matches(['v', 'V']).to_string();
    let available = latest_key > version_key(&current);
    let url = rel
        .assets
        .iter()
        .find(|a| a.name.to_lowercase().ends_with(".exe"))
        .map(|a| a.browser_download_url.clone());

    Ok(UpdateInfo {
        current,
        latest,
        available,
        url,
    })
}

/// Download the new installer and launch it, then exit so it can replace the
/// running app. (The NSIS installer reinstalls over the existing copy.)
#[tauri::command]
pub async fn install_update(app: tauri::AppHandle, url: String) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .user_agent("GPClient/0.1 (+launcher)")
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("download update: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("download update: HTTP {}", resp.status()));
    }
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("read update: {e}"))?;

    let path = std::env::temp_dir().join("GPClient-update-setup.exe");
    std::fs::write(&path, &bytes).map_err(|e| format!("save update: {e}"))?;

    // Launch the installer (detached), then quit so it can overwrite our files.
    std::process::Command::new(&path)
        .spawn()
        .map_err(|e| format!("launch installer: {e}"))?;
    app.exit(0);
    Ok(())
}
