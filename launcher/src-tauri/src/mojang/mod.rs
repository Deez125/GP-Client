//! Fetch Mojang metadata and download everything needed to launch a version:
//! the client jar, libraries (with OS rules), natives, and assets — all SHA-1
//! verified and cached under the install root. Progress is emitted to the
//! frontend as `launch://progress` events.

pub mod download;
pub mod meta;
pub mod rules;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use futures::StreamExt;
use tauri::{AppHandle, Emitter};

use meta::{AssetIndex, VersionDetails, VersionManifest};

const RESOURCES_BASE: &str = "https://resources.download.minecraft.net";

/// Progress payload sent to the UI.
#[derive(Clone, serde::Serialize)]
pub struct LaunchProgress {
    pub phase: String,
    pub current: u64,
    pub total: u64,
    pub message: String,
}

/// Emit a `launch://progress` event to the UI. Public so the mods/fabric steps
/// can report progress on the same channel.
pub fn emit_progress(app: &AppHandle, phase: &str, current: u64, total: u64, message: &str) {
    let _ = app.emit(
        "launch://progress",
        LaunchProgress {
            phase: phase.to_string(),
            current,
            total,
            message: message.to_string(),
        },
    );
}

fn emit(app: &AppHandle, phase: &str, current: u64, total: u64, message: &str) {
    emit_progress(app, phase, current, total, message);
}

/// Everything the launch step needs after files are on disk.
pub struct PreparedGame {
    pub details: VersionDetails,
    /// Vanilla library jars (NOT including the client jar — see `client_jar`).
    pub classpath: Vec<PathBuf>,
    /// The Minecraft client jar (goes last on the final classpath).
    pub client_jar: PathBuf,
    pub natives_dir: PathBuf,
    pub game_dir: PathBuf,
    pub assets_dir: PathBuf,
    /// Where library jars live (so loaders like Fabric can place theirs too).
    pub libraries_dir: PathBuf,
}

// --- paths -------------------------------------------------------------------
// assets + libraries are shared/de-duplicated under `<.minecraft>/GP Client`;
// each version instance lives directly under the installations root.
fn libraries_dir() -> PathBuf {
    crate::installations::shared_root().join("libraries")
}
fn assets_dir() -> PathBuf {
    crate::installations::shared_root().join("assets")
}
fn version_dir(id: &str) -> PathBuf {
    crate::installations::install_root().join(id)
}

fn http() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent("GPClient/0.1 (+launcher)")
        .build()
        .map_err(|e| e.to_string())
}

/// Download + verify all game files for `version_id`, returning launch inputs.
/// If `version_id` isn't in the manifest, falls back to the latest release.
pub async fn prepare(app: &AppHandle, version_id: &str) -> Result<PreparedGame, String> {
    let client = http()?;

    // Make sure the instance folder + shared junctions exist first.
    crate::installations::ensure_version(version_id).map_err(|e| e.to_string())?;

    emit(app, "manifest", 0, 0, "Fetching version manifest…");
    let manifest: VersionManifest = client
        .get(meta::VERSION_MANIFEST_URL)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let target_id = if manifest.find(version_id).is_some() {
        version_id.to_string()
    } else {
        manifest.latest.release.clone()
    };
    let mv = manifest
        .find(&target_id)
        .ok_or_else(|| format!("version {target_id} not found in manifest"))?;

    // Version JSON (cache the raw text under the version folder).
    emit(app, "version", 0, 0, &format!("Loading {target_id}…"));
    let version_json_text = client
        .get(&mv.url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())?;
    let game_dir = version_dir(&target_id);
    std::fs::create_dir_all(&game_dir).map_err(|e| e.to_string())?;
    let _ = std::fs::write(game_dir.join(format!("{target_id}.json")), &version_json_text);
    let details: VersionDetails =
        serde_json::from_str(&version_json_text).map_err(|e| format!("parse version json: {e}"))?;

    let natives_dir = game_dir.join("natives");
    std::fs::create_dir_all(&natives_dir).map_err(|e| e.to_string())?;

    // --- client jar ---------------------------------------------------------
    emit(app, "client", 0, 1, "Downloading client jar…");
    let client_jar = game_dir.join(format!("{target_id}.jar"));
    download::fetch(
        &client,
        &details.downloads.client.url,
        &client_jar,
        &details.downloads.client.sha1,
    )
    .await?;
    emit(app, "client", 1, 1, "Client jar ready");

    // --- libraries ----------------------------------------------------------
    let features: HashMap<String, bool> = HashMap::new();
    let mut classpath: Vec<PathBuf> = Vec::new();
    let mut lib_downloads: Vec<(String, PathBuf, String)> = Vec::new();
    let mut natives_jars: Vec<PathBuf> = Vec::new();

    for lib in &details.libraries {
        if !rules::allowed(&lib.rules, &features) {
            continue;
        }
        let Some(dl) = &lib.downloads else { continue };

        if let Some(art) = &dl.artifact {
            if let Some(path) = &art.path {
                let dest = libraries_dir().join(path);
                lib_downloads.push((art.url.clone(), dest.clone(), art.sha1.clone()));
                classpath.push(dest);
            }
        }

        // Old-style natives (classifier per OS) — extracted, not on classpath.
        if !lib.natives.is_empty() {
            if let Some(classifier_tmpl) = lib.natives.get(rules::os_name()) {
                let classifier =
                    classifier_tmpl.replace("${arch}", if cfg!(target_pointer_width = "64") {
                        "64"
                    } else {
                        "32"
                    });
                if let Some(art) = dl.classifiers.get(&classifier) {
                    if let Some(path) = &art.path {
                        let dest = libraries_dir().join(path);
                        lib_downloads.push((art.url.clone(), dest.clone(), art.sha1.clone()));
                        natives_jars.push(dest);
                    }
                }
            }
        }
    }

    download_all(app, &client, lib_downloads, "libraries", "Downloading libraries").await?;

    // Extract any old-style native jars into the natives dir.
    for jar in &natives_jars {
        extract_natives(jar, &natives_dir)?;
    }

    // --- assets -------------------------------------------------------------
    emit(app, "assets", 0, 0, "Loading asset index…");
    let index_path = assets_dir()
        .join("indexes")
        .join(format!("{}.json", details.asset_index.id));
    download::fetch(
        &client,
        &details.asset_index.url,
        &index_path,
        &details.asset_index.sha1,
    )
    .await?;

    let index_text = std::fs::read_to_string(&index_path).map_err(|e| e.to_string())?;
    let index: AssetIndex =
        serde_json::from_str(&index_text).map_err(|e| format!("parse asset index: {e}"))?;

    let mut asset_downloads: Vec<(String, PathBuf, String)> = Vec::new();
    for obj in index.objects.values() {
        let sub = &obj.hash[0..2];
        let dest = assets_dir().join("objects").join(sub).join(&obj.hash);
        let url = format!("{RESOURCES_BASE}/{sub}/{}", obj.hash);
        asset_downloads.push((url, dest, obj.hash.clone()));
    }
    download_all(app, &client, asset_downloads, "assets", "Downloading assets").await?;

    emit(app, "done", 1, 1, "All files ready");

    Ok(PreparedGame {
        details,
        classpath,
        client_jar,
        natives_dir,
        game_dir,
        assets_dir: assets_dir(),
        libraries_dir: libraries_dir(),
    })
}

/// Download a batch of (url, dest, sha1) with bounded concurrency + progress.
async fn download_all(
    app: &AppHandle,
    client: &reqwest::Client,
    items: Vec<(String, PathBuf, String)>,
    phase: &str,
    label: &str,
) -> Result<(), String> {
    let total = items.len() as u64;
    if total == 0 {
        return Ok(());
    }
    emit(app, phase, 0, total, label);
    let done = Arc::new(AtomicU64::new(0));
    let step = (total / 100).max(1);

    let results = futures::stream::iter(items.into_iter().map(|(url, dest, sha)| {
        let client = client.clone();
        let app = app.clone();
        let done = done.clone();
        let phase = phase.to_string();
        let label = label.to_string();
        async move {
            let r = download::fetch(&client, &url, &dest, &sha).await;
            let n = done.fetch_add(1, Ordering::Relaxed) + 1;
            if n % step == 0 || n == total {
                emit(&app, &phase, n, total, &format!("{label} ({n}/{total})"));
            }
            r
        }
    }))
    .buffer_unordered(16)
    .collect::<Vec<_>>()
    .await;

    for r in results {
        r?;
    }
    Ok(())
}

/// Extract a native jar's contents (dll/so/dylib) into `dest`, skipping
/// metadata and directories.
fn extract_natives(jar: &Path, dest: &Path) -> Result<(), String> {
    let file = std::fs::File::open(jar).map_err(|e| format!("open {jar:?}: {e}"))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("zip {jar:?}: {e}"))?;
    std::fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        if entry.is_dir() {
            continue;
        }
        let Some(enclosed) = entry.enclosed_name() else {
            continue;
        };
        if enclosed.starts_with("META-INF") {
            continue;
        }
        let Some(file_name) = enclosed.file_name() else {
            continue;
        };
        let out = dest.join(file_name);
        let mut writer = std::fs::File::create(&out).map_err(|e| e.to_string())?;
        std::io::copy(&mut entry, &mut writer).map_err(|e| e.to_string())?;
    }
    Ok(())
}
