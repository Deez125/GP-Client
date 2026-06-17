//! Step 8 (GitHub mod system, per mod-system-spec.md).
//!
//! On launch we sync the version's `mods/` folder so it contains exactly:
//!   * every jar in the repo's `required/` folder, plus
//!   * the optional mods the user ticked in the Mods UI.
//!
//! Sync is INCREMENTAL and NON-DESTRUCTIVE:
//!   * only jars GP installed before (recorded in `.modsync.json`) are ever
//!     removed — user-added mods are invisible to it and never touched;
//!   * a jar already present is re-downloaded only if the hosted file's git SHA
//!     changed (so same-filename updates are still picked up);
//!   * a removed/unticked/updated mod's old jar is cleaned up.

mod github;
mod manifest;
mod selection;
mod state;

use std::collections::HashSet;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use futures::StreamExt;
use serde::Serialize;
use tauri::AppHandle;

use sha1::{Digest, Sha1};

use crate::mojang::{download, emit_progress};

fn http() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent("GPClient/0.1 (+launcher)")
        .build()
        .map_err(|e| e.to_string())
}

fn version_dir(version: &str) -> std::path::PathBuf {
    crate::installations::install_root().join(version)
}

/// Git blob SHA-1 of a file — the same value the GitHub contents API reports
/// for a file — so we can tell whether an on-disk jar still matches the hosted
/// one (even when the filename is unchanged).
fn git_blob_sha1(path: &Path) -> Option<String> {
    let data = std::fs::read(path).ok()?;
    let mut h = Sha1::new();
    h.update(format!("blob {}\0", data.len()).as_bytes());
    h.update(&data);
    Some(h.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

#[derive(Debug, Default, serde::Serialize)]
pub struct SyncResult {
    pub installed: Vec<String>,
    pub removed: Vec<String>,
}

/// Sync the required + ticked-optional mods for `version` into its `mods/` folder.
pub async fn sync(app: &AppHandle, version: &str) -> Result<SyncResult, String> {
    let dir = version_dir(version);
    let mods_dir = dir.join("mods");
    std::fs::create_dir_all(&mods_dir).map_err(|e| e.to_string())?;

    let client = http()?;

    emit_progress(app, "mods", 0, 0, "Reading mod manifest…");
    let manifest = manifest::fetch(&client).await?;
    let ventry = manifest
        .versions
        .get(version)
        .ok_or_else(|| format!("version {version} is not in the mod manifest"))?;

    emit_progress(app, "mods", 0, 0, "Finding required mods…");
    let required = github::list_jars(&client, &manifest.required_api_url(version)).await?;
    if required.is_empty() {
        return Err(format!("no required mods found on GitHub for {version}"));
    }

    // Resolve the user's ticked optional mods to real files.
    let selected = selection::read(&dir);
    let optional = resolve_optional(&client, &manifest, version, ventry, &selected).await?;

    // The full target set.
    let mut targets: Vec<github::RemoteJar> = required;
    targets.extend(optional);
    let target_names: HashSet<String> = targets.iter().map(|j| j.name.clone()).collect();

    // (Re)download anything missing, or whose on-disk content no longer matches
    // the hosted file's git SHA — this catches updates that reuse the same
    // filename. A stale jar is deleted first so it actually re-downloads.
    let mut needed: Vec<(String, String)> = Vec::new();
    for j in &targets {
        let dest = mods_dir.join(&j.name);
        if !dest.exists() {
            needed.push((j.name.clone(), j.url.clone()));
        } else if !j.sha.is_empty() && git_blob_sha1(&dest).as_deref() != Some(j.sha.as_str()) {
            let _ = std::fs::remove_file(&dest);
            needed.push((j.name.clone(), j.url.clone()));
        }
    }
    if !needed.is_empty() {
        download_all(app, &client, &needed, &mods_dir).await?;
    }

    // Remove only OUR previously-managed jars that are no longer wanted
    // (unticked optional, or an old filename of an updated mod). User mods are
    // never in the managed list, so they're untouched.
    let previous = state::read(&mods_dir);
    let mut removed = Vec::new();
    for name in &previous.managed {
        if !target_names.contains(name) {
            let p = mods_dir.join(name);
            if p.exists() && std::fs::remove_file(&p).is_ok() {
                removed.push(name.clone());
            }
        }
    }

    // Record the new managed set.
    let mut managed: Vec<String> = target_names.into_iter().collect();
    managed.sort();
    state::write(&mods_dir, version, managed)?;

    let installed: Vec<String> = needed.into_iter().map(|(n, _)| n).collect();
    emit_progress(
        app,
        "mods",
        targets.len() as u64,
        targets.len() as u64,
        &format!("{} mod(s) ready", targets.len()),
    );

    Ok(SyncResult { installed, removed })
}

/// Resolve the ticked optional mods to `RemoteJar`s.
async fn resolve_optional(
    client: &reqwest::Client,
    manifest: &manifest::Manifest,
    version: &str,
    ventry: &manifest::VersionEntry,
    selected: &[String],
) -> Result<Vec<github::RemoteJar>, String> {
    if selected.is_empty() {
        return Ok(Vec::new());
    }
    // List the optional folder so we can prefix-match (best-effort).
    let listing = github::list_jars(client, &manifest.optional_api_url(version))
        .await
        .unwrap_or_default();

    let mut out = Vec::new();
    for cat in &ventry.optional {
        for m in &cat.mods {
            if m.inactive || !selected.iter().any(|s| s == &m.name) {
                continue;
            }
            if let Some(url) = &m.url {
                let filename = url
                    .rsplit('/')
                    .next()
                    .filter(|s| !s.is_empty())
                    .unwrap_or("mod.jar")
                    .to_string();
                // Externally hosted: no git sha, so it's existence-only checked.
                out.push(github::RemoteJar {
                    name: filename,
                    url: url.clone(),
                    sha: String::new(),
                });
            } else if let Some(prefix) = &m.jar {
                let pl = prefix.to_lowercase();
                if let Some(j) = listing
                    .iter()
                    .find(|j| j.name.to_lowercase().starts_with(&pl))
                {
                    out.push(j.clone());
                }
            }
        }
    }
    Ok(out)
}

/// Download a batch of (filename, url) into `dir` with bounded concurrency.
async fn download_all(
    app: &AppHandle,
    client: &reqwest::Client,
    items: &[(String, String)],
    dir: &Path,
) -> Result<(), String> {
    let total = items.len() as u64;
    emit_progress(app, "mods", 0, total, "Downloading mods…");
    let done = Arc::new(AtomicU64::new(0));

    let results = futures::stream::iter(items.iter().cloned().map(|(name, url)| {
        let client = client.clone();
        let app = app.clone();
        let done = done.clone();
        let dir = dir.to_path_buf();
        async move {
            let dest = dir.join(&name);
            let r = download::fetch_maybe(&client, &url, &dest, None).await;
            let n = done.fetch_add(1, Ordering::Relaxed) + 1;
            emit_progress(&app, "mods", n, total, &format!("Downloading mods ({n}/{total})"));
            r
        }
    }))
    .buffer_unordered(8)
    .collect::<Vec<_>>()
    .await;

    for r in results {
        r?;
    }
    Ok(())
}

fn version_key(v: &str) -> Vec<u32> {
    v.replace(['-', '_'], ".")
        .split('.')
        .map(|chunk| {
            chunk.parse::<u32>().unwrap_or_else(|_| {
                let digits: String = chunk.chars().filter(|c| c.is_ascii_digit()).collect();
                digits.parse().unwrap_or(0)
            })
        })
        .collect()
}

// --- Views for the Mods UI --------------------------------------------------

#[derive(Serialize)]
pub struct VersionMods {
    pub required: Vec<String>,
    pub optional: Vec<OptionalCategoryView>,
}

#[derive(Serialize)]
pub struct OptionalCategoryView {
    pub category: String,
    pub mods: Vec<OptionalModView>,
}

#[derive(Serialize)]
pub struct OptionalModView {
    pub name: String,
    pub description: Option<String>,
    pub image_url: Option<String>,
    /// Installable (has a matching file or a url override, and not inactive).
    pub available: bool,
    pub inactive: bool,
    pub selected: bool,
}

// --- Tauri commands ---------------------------------------------------------

#[tauri::command]
pub async fn sync_mods(app: AppHandle, version: String) -> Result<SyncResult, String> {
    sync(&app, &version).await
}

/// The versions available in the manifest, newest first (for the version picker).
#[tauri::command]
pub async fn list_mod_versions() -> Result<Vec<String>, String> {
    let client = http()?;
    let manifest = manifest::fetch(&client).await?;
    let mut versions: Vec<String> = manifest.versions.into_keys().collect();
    versions.sort_by(|a, b| version_key(b).cmp(&version_key(a)));
    Ok(versions)
}

/// The required + optional mod catalog for a version, resolved for the Mods UI.
#[tauri::command]
pub async fn get_version_mods(version: String) -> Result<VersionMods, String> {
    let client = http()?;
    let manifest = manifest::fetch(&client).await?;
    let ventry = manifest
        .versions
        .get(&version)
        .ok_or_else(|| format!("version {version} is not in the mod manifest"))?;

    // List the optional folder to know which mods are actually downloadable.
    let listing = github::list_jars(&client, &manifest.optional_api_url(&version))
        .await
        .unwrap_or_default();
    let selected = selection::read(&version_dir(&version));

    let required = ventry.required.iter().map(|r| r.name.clone()).collect();

    let mut optional = Vec::new();
    for cat in &ventry.optional {
        let mut mods = Vec::new();
        for m in &cat.mods {
            let available = if m.inactive {
                false
            } else if m.url.is_some() {
                true
            } else if let Some(prefix) = &m.jar {
                let pl = prefix.to_lowercase();
                listing.iter().any(|j| j.name.to_lowercase().starts_with(&pl))
            } else {
                false
            };
            optional_push(&mut mods, &manifest, &version, m, available, &selected);
        }
        optional.push(OptionalCategoryView {
            category: cat.category.clone(),
            mods,
        });
    }

    Ok(VersionMods { required, optional })
}

fn optional_push(
    mods: &mut Vec<OptionalModView>,
    manifest: &manifest::Manifest,
    version: &str,
    m: &manifest::OptionalMod,
    available: bool,
    selected: &[String],
) {
    let image_url = m.image.as_ref().map(|img| manifest.image_url(version, img));
    mods.push(OptionalModView {
        name: m.name.clone(),
        description: m.description.clone(),
        image_url,
        available,
        inactive: m.inactive,
        selected: selected.iter().any(|s| s == &m.name),
    });
}

/// Save the user's optional-mod selection for a version (applied next launch).
#[tauri::command]
pub fn set_optional_mods(version: String, selected: Vec<String>) -> Result<(), String> {
    let dir = version_dir(&version);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    selection::write(&dir, &selected)
}
