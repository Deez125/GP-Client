//! Import game settings from an existing Minecraft profile into a GP Client
//! installation. The user's `options.txt` (volume/keybinds/video), server list,
//! shader selection, and a couple of per-instance data folders (Litematica
//! schematics, Xaero's world map) can be copied across.
//!
//! Source profiles are discovered from the vanilla launcher's
//! `launcher_profiles.json` so the UI can list them by name (e.g. "Essential").
//! Resource packs / shaderpacks / saves are intentionally NOT copied — GP Client
//! already junctions those to the vanilla `.minecraft` folders.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::installations::{install_root, minecraft_dir};

/// A Minecraft directory the user can import from, shown in the UI.
#[derive(Debug, Clone, Serialize)]
pub struct SourceProfile {
    /// Display name (profile name, or "Default (.minecraft)").
    pub name: String,
    /// Absolute path to that profile's game directory.
    pub path: String,
}

/// Which pieces to import. Each maps to one or more files/folders.
#[derive(Debug, Clone, Deserialize)]
pub struct ImportItems {
    #[serde(default)]
    pub options: bool,
    #[serde(default)]
    pub shaders: bool,
    #[serde(default)]
    pub servers: bool,
    #[serde(default)]
    pub schematics: bool,
    #[serde(default)]
    pub xaero: bool,
}

/// Summary returned to the UI after an import.
#[derive(Debug, Default, Serialize)]
pub struct ImportReport {
    /// Human-readable lines for things that were copied.
    pub imported: Vec<String>,
    /// Things that were requested but not found in the source.
    pub skipped: Vec<String>,
}

// --- launcher_profiles.json parsing -----------------------------------------

#[derive(Deserialize)]
struct LauncherProfiles {
    #[serde(default)]
    profiles: std::collections::HashMap<String, RawProfile>,
}

#[derive(Deserialize)]
struct RawProfile {
    #[serde(default)]
    name: Option<String>,
    #[serde(rename = "gameDir", default)]
    game_dir: Option<String>,
    #[serde(rename = "type", default)]
    kind: Option<String>,
}

/// List importable source profiles: every launcher profile whose game directory
/// exists, plus the default `.minecraft` itself. Deduplicated by (name, path).
#[tauri::command]
pub fn list_source_profiles() -> Vec<SourceProfile> {
    let mc = minecraft_dir();
    let mut out: Vec<SourceProfile> = Vec::new();

    // Always offer the default .minecraft (if it exists).
    if mc.is_dir() {
        out.push(SourceProfile {
            name: "Default (.minecraft)".to_string(),
            path: mc.to_string_lossy().into_owned(),
        });
    }

    // Parse the vanilla launcher's profile list, if present.
    let profiles_path = mc.join("launcher_profiles.json");
    if let Ok(text) = std::fs::read_to_string(&profiles_path) {
        if let Ok(parsed) = serde_json::from_str::<LauncherProfiles>(&text) {
            for raw in parsed.profiles.values() {
                // A profile with no gameDir uses the default .minecraft.
                let dir = match &raw.game_dir {
                    Some(d) if !d.trim().is_empty() => PathBuf::from(d),
                    _ => mc.clone(),
                };
                if !dir.is_dir() {
                    continue;
                }
                let name = raw
                    .name
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .or_else(|| raw.kind.clone())
                    .unwrap_or_else(|| "Unnamed".to_string());
                let path = dir.to_string_lossy().into_owned();
                if !out.iter().any(|p| p.name == name && p.path == path) {
                    out.push(SourceProfile { name, path });
                }
            }
        }
    }

    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    out
}

// --- import ------------------------------------------------------------------

/// Copy the selected items from `source` into the GP Client `version` install.
#[tauri::command]
pub fn import_profile_settings(
    version: String,
    source: String,
    items: ImportItems,
) -> Result<ImportReport, String> {
    let source = PathBuf::from(&source);
    if !source.is_dir() {
        return Err(format!("source folder not found: {}", source.display()));
    }

    let game_dir = install_root().join(&version);
    std::fs::create_dir_all(&game_dir).map_err(|e| format!("create install dir: {e}"))?;

    let mut report = ImportReport::default();

    if items.options {
        copy_file(&source, &game_dir, "options.txt", "Game options & keybinds", &mut report)?;
    }
    if items.servers {
        copy_file(&source, &game_dir, "servers.dat", "Server list", &mut report)?;
    }
    if items.shaders {
        // The active shader (and Iris settings) live in config/iris.properties.
        let cfg = game_dir.join("config");
        std::fs::create_dir_all(&cfg).map_err(|e| format!("create config dir: {e}"))?;
        let src = source.join("config").join("iris.properties");
        if src.is_file() {
            std::fs::copy(&src, cfg.join("iris.properties"))
                .map_err(|e| format!("copy iris.properties: {e}"))?;
            report.imported.push("Shader selection".to_string());
        } else {
            report.skipped.push("Shader selection (no Iris config in source)".to_string());
        }
    }
    if items.schematics {
        copy_dir_item(&source, &game_dir, "schematics", "Litematica schematics", &mut report)?;
    }
    if items.xaero {
        // Copy every Xaero* data folder in full (World Map + Waypoints + any
        // variant), recursively, so per-world map tiles come across too.
        let mut copied = 0u32;
        if let Ok(entries) = std::fs::read_dir(&source) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                if name.to_string_lossy().to_lowercase().starts_with("xaero")
                    && entry.path().is_dir()
                {
                    copy_dir_all(&entry.path(), &game_dir.join(&name))
                        .map_err(|e| format!("copy {}: {e}", name.to_string_lossy()))?;
                    copied += 1;
                }
            }
        }
        if copied > 0 {
            report.imported.push("Xaero's world map".to_string());
        } else {
            report.skipped.push("Xaero's world map (not found in source)".to_string());
        }
    }

    Ok(report)
}

/// Copy a single file from `src_dir/name` to `dst_dir/name`, recording the result.
fn copy_file(
    src_dir: &Path,
    dst_dir: &Path,
    name: &str,
    label: &str,
    report: &mut ImportReport,
) -> Result<(), String> {
    let src = src_dir.join(name);
    if src.is_file() {
        std::fs::copy(&src, dst_dir.join(name)).map_err(|e| format!("copy {name}: {e}"))?;
        report.imported.push(label.to_string());
    } else {
        report.skipped.push(format!("{label} (not found in source)"));
    }
    Ok(())
}

/// Copy a directory `src_dir/name` into `dst_dir/name` (recursive, merging).
fn copy_dir_item(
    src_dir: &Path,
    dst_dir: &Path,
    name: &str,
    label: &str,
    report: &mut ImportReport,
) -> Result<(), String> {
    let src = src_dir.join(name);
    if src.is_dir() {
        copy_dir_all(&src, &dst_dir.join(name)).map_err(|e| format!("copy {name}: {e}"))?;
        report.imported.push(label.to_string());
    } else {
        report.skipped.push(format!("{label} (not found in source)"));
    }
    Ok(())
}

/// Recursively copy `src` into `dst`, creating directories and overwriting files.
fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_all(&from, &to)?;
        } else {
            std::fs::copy(&from, &to)?;
        }
    }
    Ok(())
}
