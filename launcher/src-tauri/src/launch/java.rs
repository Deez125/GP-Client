//! Locate a Java runtime matching the version Minecraft requires.
//!
//! For now we use a Java already on the system (the spec's "detect/require it"
//! option). Auto-downloading Mojang's bundled runtime can be added later.
//!
//! Search order:
//!   1. `GP_CLIENT_JAVA_PATH` env var (explicit path to `java`/`java.exe`)
//!   2. `JAVA_HOME`
//!   3. Common JDK install locations (Adoptium/Temurin, Microsoft, Java)
//!   4. `java` on `PATH`
//! The first candidate whose major version matches is used.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use futures::StreamExt;
use serde::Deserialize;
use tauri::AppHandle;

use crate::mojang::{download, emit_progress};

const JAVA_BIN: &str = if cfg!(windows) { "java.exe" } else { "java" };

const JAVA_RUNTIME_ALL: &str =
    "https://piston-meta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json";

/// Resolve a Java executable for the version: prefer a matching system JDK,
/// otherwise download Mojang's bundled runtime for `component` (so users don't
/// need Java installed). Returns the path to the `java` executable.
pub async fn ensure_java(
    app: &AppHandle,
    component: &str,
    required_major: u32,
) -> Result<PathBuf, String> {
    // 1. Use a matching system Java if we can find one (no download needed).
    if let Ok(p) = find_java(required_major) {
        return Ok(p);
    }

    // 2. Otherwise download Mojang's runtime for this component.
    download_runtime(app, component).await
}

#[derive(Deserialize)]
struct RuntimeEntry {
    manifest: ManifestRef,
    version: VersionName,
}
#[derive(Deserialize)]
struct ManifestRef {
    url: String,
}
#[derive(Deserialize)]
struct VersionName {
    name: String,
}
#[derive(Deserialize)]
struct RuntimeFiles {
    files: HashMap<String, RuntimeFile>,
}
#[derive(Deserialize)]
struct RuntimeFile {
    #[serde(rename = "type")]
    kind: String,
    downloads: Option<RuntimeDownloads>,
}
#[derive(Deserialize)]
struct RuntimeDownloads {
    raw: RawDownload,
}
#[derive(Deserialize)]
struct RawDownload {
    url: String,
    sha1: String,
}

fn platform_key() -> &'static str {
    if cfg!(target_os = "windows") {
        if cfg!(target_arch = "aarch64") {
            "windows-arm64"
        } else if cfg!(target_arch = "x86") {
            "windows-x86"
        } else {
            "windows-x64"
        }
    } else if cfg!(target_os = "macos") {
        if cfg!(target_arch = "aarch64") {
            "mac-os-arm64"
        } else {
            "mac-os"
        }
    } else if cfg!(target_arch = "x86") {
        "linux-i386"
    } else {
        "linux"
    }
}

async fn download_runtime(app: &AppHandle, component: &str) -> Result<PathBuf, String> {
    let runtime_dir = crate::installations::shared_root()
        .join("runtimes")
        .join(component);
    let java_exe = runtime_dir.join("bin").join(JAVA_BIN);
    if java_exe.exists() {
        return Ok(java_exe); // already downloaded
    }

    emit_progress(app, "java", 0, 0, "Fetching Java runtime…");
    let client = reqwest::Client::builder()
        .user_agent("GPClient/0.1 (+launcher)")
        .build()
        .map_err(|e| e.to_string())?;

    // all.json: platform -> component -> [entry]
    let all: HashMap<String, HashMap<String, Vec<RuntimeEntry>>> = client
        .get(JAVA_RUNTIME_ALL)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| format!("java runtime list: {e}"))?;

    let platform = platform_key();
    let entry = all
        .get(platform)
        .and_then(|m| m.get(component))
        .and_then(|v| v.first())
        .ok_or_else(|| format!("no Java runtime '{component}' available for {platform}"))?;

    let files: RuntimeFiles = client
        .get(&entry.manifest.url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| format!("java runtime manifest: {e}"))?;

    // Build the download list (and create directories).
    let mut items: Vec<(String, PathBuf, String)> = Vec::new();
    for (rel, f) in &files.files {
        let dest = runtime_dir.join(rel);
        if f.kind == "directory" {
            let _ = std::fs::create_dir_all(&dest);
            continue;
        }
        if let Some(dl) = &f.downloads {
            items.push((dl.raw.url.clone(), dest, dl.raw.sha1.clone()));
        }
    }

    let total = items.len() as u64;
    emit_progress(
        app,
        "java",
        0,
        total,
        &format!("Downloading Java {}…", entry.version.name),
    );
    let done = Arc::new(AtomicU64::new(0));
    let results = futures::stream::iter(items.into_iter().map(|(url, dest, sha)| {
        let client = client.clone();
        let app = app.clone();
        let done = done.clone();
        async move {
            let r = download::fetch(&client, &url, &dest, &sha).await;
            let n = done.fetch_add(1, Ordering::Relaxed) + 1;
            if n % 20 == 0 || n == total {
                emit_progress(&app, "java", n, total, &format!("Downloading Java ({n}/{total})"));
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

    if !java_exe.exists() {
        return Err("Java runtime downloaded but the executable is missing".into());
    }
    Ok(java_exe)
}

/// Find a `java` executable whose major version equals `required_major`.
pub fn find_java(required_major: u32) -> Result<PathBuf, String> {
    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Ok(p) = std::env::var("GP_CLIENT_JAVA_PATH") {
        if !p.trim().is_empty() {
            candidates.push(PathBuf::from(p));
        }
    }
    if let Ok(home) = std::env::var("JAVA_HOME") {
        if !home.trim().is_empty() {
            candidates.push(Path::new(&home).join("bin").join(JAVA_BIN));
        }
    }
    candidates.extend(scan_common_locations());
    if let Some(p) = which_java() {
        candidates.push(p);
    }

    let mut seen_versions = Vec::new();
    for cand in &candidates {
        if !cand.exists() {
            continue;
        }
        match java_major_version(cand) {
            Some(major) => {
                if major == required_major {
                    return Ok(cand.clone());
                }
                seen_versions.push(format!("{} (Java {major})", cand.display()));
            }
            None => {}
        }
    }

    Err(format!(
        "No Java {required_major} runtime found. Install JDK {required_major} or set GP_CLIENT_JAVA_PATH. Detected: {}",
        if seen_versions.is_empty() {
            "none".to_string()
        } else {
            seen_versions.join("; ")
        }
    ))
}

/// Scan common Windows JDK install roots for `*/bin/java.exe`.
fn scan_common_locations() -> Vec<PathBuf> {
    let mut out = Vec::new();
    let roots = [
        r"C:\Program Files\Eclipse Adoptium",
        r"C:\Program Files\Microsoft",
        r"C:\Program Files\Java",
        r"C:\Program Files\Zulu",
        r"C:\Program Files\Amazon Corretto",
    ];
    for root in roots {
        let Ok(entries) = std::fs::read_dir(root) else {
            continue;
        };
        for entry in entries.flatten() {
            let candidate = entry.path().join("bin").join(JAVA_BIN);
            if candidate.exists() {
                out.push(candidate);
            }
        }
    }
    out
}

/// Resolve `java` via the OS PATH (best effort).
fn which_java() -> Option<PathBuf> {
    let cmd = if cfg!(windows) { "where" } else { "which" };
    let output = Command::new(cmd).arg("java").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines().next().map(|l| PathBuf::from(l.trim()))
}

/// Run `java -version` and parse the major version (e.g. 25 from "25.0.3",
/// or 8 from the legacy "1.8.0"). `-version` prints to stderr.
fn java_major_version(java: &Path) -> Option<u32> {
    let output = Command::new(java).arg("-version").output().ok()?;
    let text = String::from_utf8_lossy(&output.stderr);
    // First quoted token is the version string.
    let start = text.find('"')? + 1;
    let end = text[start..].find('"')? + start;
    let version = &text[start..end];
    let mut parts = version.split('.');
    let first = parts.next()?;
    if first == "1" {
        parts.next()?.parse().ok()
    } else {
        first.parse().ok()
    }
}
