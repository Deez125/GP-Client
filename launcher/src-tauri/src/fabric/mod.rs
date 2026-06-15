//! Step 7: add the Fabric mod loader on top of a prepared vanilla version.
//!
//! Fabric's meta API gives us a "profile" that inherits from the vanilla
//! version: a set of extra libraries (from Fabric's maven), a replacement main
//! class (`KnotClient`), and some extra JVM args. We download the libraries and
//! hand those pieces back so the launch step can merge them.

use std::path::{Path, PathBuf};

use serde::Deserialize;
use tauri::AppHandle;

use crate::mojang::{download, emit_progress};

const META_BASE: &str = "https://meta.fabricmc.net/v2/versions/loader";
const DEFAULT_MAVEN: &str = "https://maven.fabricmc.net/";

#[derive(Deserialize)]
struct LoaderEntry {
    loader: LoaderInfo,
}

#[derive(Deserialize)]
struct LoaderInfo {
    version: String,
    #[serde(default)]
    stable: bool,
}

#[derive(Deserialize)]
struct Profile {
    #[serde(rename = "mainClass")]
    main_class: serde_json::Value,
    #[serde(default)]
    libraries: Vec<FabricLib>,
    arguments: Option<FabricArgs>,
}

#[derive(Deserialize)]
struct FabricLib {
    name: String,
    url: Option<String>,
    sha1: Option<String>,
}

#[derive(Deserialize, Default)]
struct FabricArgs {
    #[serde(default)]
    jvm: Vec<String>,
    #[serde(default)]
    game: Vec<String>,
}

/// The Fabric pieces to merge into the launch.
pub struct FabricPrepared {
    pub loader_version: String,
    pub main_class: String,
    /// Fabric library jars (go on the classpath ahead of the vanilla ones).
    pub libraries: Vec<PathBuf>,
    pub jvm_args: Vec<String>,
    pub game_args: Vec<String>,
}

/// Fetch the latest stable Fabric loader for `game_version`, download its
/// libraries into `libraries_dir`, and return the launch pieces.
pub async fn prepare(
    app: &AppHandle,
    client: &reqwest::Client,
    libraries_dir: &Path,
    game_version: &str,
) -> Result<FabricPrepared, String> {
    emit_progress(app, "fabric", 0, 0, "Resolving Fabric loader…");

    let loaders: Vec<LoaderEntry> = client
        .get(format!("{META_BASE}/{game_version}"))
        .send()
        .await
        .map_err(|e| format!("fabric loader list: {e}"))?
        .json()
        .await
        .map_err(|e| format!("parse fabric loader list: {e}"))?;

    let loader_version = loaders
        .iter()
        .find(|l| l.loader.stable)
        .or_else(|| loaders.first())
        .map(|l| l.loader.version.clone())
        .ok_or_else(|| format!("no Fabric loader available for {game_version}"))?;

    let profile: Profile = client
        .get(format!(
            "{META_BASE}/{game_version}/{loader_version}/profile/json"
        ))
        .send()
        .await
        .map_err(|e| format!("fabric profile: {e}"))?
        .json()
        .await
        .map_err(|e| format!("parse fabric profile: {e}"))?;

    let main_class = match &profile.main_class {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Object(o) => o
            .get("client")
            .and_then(|v| v.as_str())
            .unwrap_or("net.fabricmc.loader.impl.launch.knot.KnotClient")
            .to_string(),
        _ => "net.fabricmc.loader.impl.launch.knot.KnotClient".to_string(),
    };

    emit_progress(
        app,
        "fabric",
        0,
        profile.libraries.len() as u64,
        "Downloading Fabric libraries…",
    );

    let mut libraries = Vec::new();
    for (i, lib) in profile.libraries.iter().enumerate() {
        let path = maven_path(&lib.name)
            .ok_or_else(|| format!("bad fabric library name: {}", lib.name))?;
        let base = lib.url.clone().unwrap_or_else(|| DEFAULT_MAVEN.to_string());
        let url = format!("{}{}", ensure_trailing_slash(&base), path);
        let dest = libraries_dir.join(&path);
        download::fetch_maybe(client, &url, &dest, lib.sha1.as_deref()).await?;
        libraries.push(dest);
        emit_progress(
            app,
            "fabric",
            (i + 1) as u64,
            profile.libraries.len() as u64,
            "Downloading Fabric libraries…",
        );
    }

    let args = profile.arguments.unwrap_or_default();

    Ok(FabricPrepared {
        loader_version,
        main_class,
        libraries,
        jvm_args: args.jvm,
        game_args: args.game,
    })
}

fn ensure_trailing_slash(s: &str) -> String {
    if s.ends_with('/') {
        s.to_string()
    } else {
        format!("{s}/")
    }
}

/// Convert a maven coordinate ("group:artifact:version[:classifier]") into its
/// repository path.
fn maven_path(name: &str) -> Option<String> {
    let parts: Vec<&str> = name.split(':').collect();
    if parts.len() < 3 {
        return None;
    }
    let group = parts[0].replace('.', "/");
    let artifact = parts[1];
    let version = parts[2];
    let classifier = parts.get(3);
    let file = match classifier {
        Some(c) => format!("{artifact}-{version}-{c}.jar"),
        None => format!("{artifact}-{version}.jar"),
    };
    Some(format!("{group}/{artifact}/{version}/{file}"))
}
