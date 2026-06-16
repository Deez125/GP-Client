//! Step 3: the isolated installations folder tree.
//!
//! Layout (per the launcher spec §5):
//!
//! ```text
//! <root>/                         e.g. %LOCALAPPDATA%\GP Client installations
//!   shared/
//!     resourcepacks/
//!     shaderpacks/
//!     saves/
//!   versions/
//!     <version>/
//!       mods/        <- real, per-instance, isolated (NEVER shared/symlinked)
//!       config/
//!       (resourcepacks/shaderpacks/saves junctions are added in step 4)
//! ```
//!
//! This module only creates *real* directories. The shared-folder shortcuts
//! (junctions on Windows) are a separate, deliberately isolated step (step 4).

mod layout;
mod links;
mod meta;

pub use layout::Layout;
pub use meta::InstanceMeta;

use serde::Serialize;
use std::path::PathBuf;

use crate::brand;

/// Resolve the vanilla `.minecraft` directory (where shared resourcepacks,
/// shaderpacks, and saves live — the junction targets).
///
/// Order: `GP_CLIENT_MINECRAFT_DIR` env override → Roaming AppData `.minecraft`
/// on Windows (sensible fallbacks elsewhere).
pub fn minecraft_dir() -> PathBuf {
    if let Ok(custom) = std::env::var("GP_CLIENT_MINECRAFT_DIR") {
        if !custom.trim().is_empty() {
            return PathBuf::from(custom);
        }
    }
    // On Windows, config_dir() is %APPDATA% (Roaming) — exactly where the
    // vanilla launcher keeps .minecraft.
    let roaming = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    roaming.join(".minecraft")
}

/// Resolve the installations root: `<.minecraft>/GP Client installations`.
/// Each version installation lives directly under here (e.g. `<root>/26.1.2`).
///
/// `GP_CLIENT_INSTALL_DIR` overrides the whole path if set.
pub fn install_root() -> PathBuf {
    if let Ok(custom) = std::env::var("GP_CLIENT_INSTALL_DIR") {
        if !custom.trim().is_empty() {
            return PathBuf::from(custom);
        }
    }
    minecraft_dir().join(brand::brand().installations_folder_name)
}

/// Resolve the shared-support root: `<.minecraft>/GP Client`. Holds the shared,
/// de-duplicated `assets/` and `libraries/` folders (downloaded once, used by
/// every installation). Sibling to the installations root.
pub fn shared_root() -> PathBuf {
    minecraft_dir().join(brand::brand().shared_folder_name)
}

fn layout() -> Layout {
    Layout::new(install_root(), minecraft_dir())
}

/// Info about a single version installation, for the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct InstallationInfo {
    /// The version id (also the folder name) — stable identity.
    pub version: String,
    /// Launcher-only display name (defaults to the version).
    pub name: String,
    /// Absolute path to the version's folder.
    pub path: String,
    /// Absolute path to the version's real `mods/` folder.
    pub mods_path: String,
    /// Max heap in MB (`-Xmx`).
    pub ram_mb: u32,
    /// Extra JVM arguments.
    pub jvm_args: String,
}

impl InstallationInfo {
    fn from_layout(layout: &Layout, version: &str) -> Self {
        let dir = layout.version_dir(version);
        let m = meta::read(&dir, version);
        InstallationInfo {
            version: version.to_string(),
            name: m.name,
            path: dir.to_string_lossy().into_owned(),
            mods_path: layout.version_mods(version).to_string_lossy().into_owned(),
            ram_mb: m.ram_mb,
            jvm_args: m.jvm_args,
        }
    }
}

/// Read the launcher-only settings for a version (used by the launch step).
pub fn instance_meta(version: &str) -> InstanceMeta {
    meta::read(&layout().version_dir(version), version)
}

/// Create the base tree (`versions/`) if it doesn't exist. Idempotent — safe to
/// call every launch.
pub fn ensure_base() -> std::io::Result<()> {
    layout().ensure_base()
}

/// Create a version installation: real `mods/` + `config/`, plus junctions for
/// resourcepacks/shaderpacks/saves pointing at the vanilla `.minecraft`
/// folders. Idempotent.
pub fn ensure_version(version: &str) -> std::io::Result<InstallationInfo> {
    let layout = layout();
    layout.ensure_version(version)?;
    // Register it as a real installation with default settings the first time,
    // so launching from a fresh state (no installations yet) produces a proper
    // entry in the Installations tab.
    let dir = layout.version_dir(version);
    if !dir.join(meta::META_FILENAME).exists() {
        let mut m = meta::InstanceMeta::default_for(version);
        // Seed new installations with the user's default memory setting.
        let gb = crate::settings::load().default_memory_gb.max(1);
        m.ram_mb = (gb * 1024).clamp(512, 65536);
        let _ = meta::write(&dir, &m);
    }
    Ok(InstallationInfo::from_layout(&layout, version))
}

/// List the version installations that currently exist on disk (each is a
/// directory directly under the installations root).
pub fn list_installations() -> std::io::Result<Vec<InstallationInfo>> {
    let layout = layout();
    let root = install_root();
    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut out = Vec::new();
    for entry in std::fs::read_dir(&root)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            let version = entry.file_name().to_string_lossy().into_owned();
            out.push(InstallationInfo::from_layout(&layout, &version));
        }
    }
    out.sort_by(|a, b| a.version.cmp(&b.version));
    Ok(out)
}

// --- Tauri commands ---------------------------------------------------------

/// The resolved installations root path (for display / "open folder").
#[tauri::command]
pub fn installations_root() -> String {
    install_root().to_string_lossy().into_owned()
}

#[tauri::command]
pub fn create_installation(version: String) -> Result<InstallationInfo, String> {
    ensure_version(&version).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_installations_cmd() -> Result<Vec<InstallationInfo>, String> {
    list_installations().map_err(|e| e.to_string())
}

/// Open the installations root in the OS file manager (for inspection).
#[tauri::command]
pub fn open_installations_folder() -> Result<(), String> {
    let root = install_root();
    std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    open::that(&root).map_err(|e| e.to_string())
}

/// Open a specific installation's folder.
#[tauri::command]
pub fn open_installation_folder(version: String) -> Result<(), String> {
    let dir = layout().version_dir(&version);
    if !dir.exists() {
        return Err(format!("installation {version} doesn't exist"));
    }
    open::that(&dir).map_err(|e| e.to_string())
}

/// Update a single installation's launcher-only settings.
#[tauri::command]
pub fn update_installation(
    version: String,
    name: String,
    ram_mb: u32,
    jvm_args: String,
) -> Result<InstallationInfo, String> {
    let dir = layout().version_dir(&version);
    if !dir.exists() {
        return Err(format!("installation {version} doesn't exist"));
    }
    let m = InstanceMeta {
        name: if name.trim().is_empty() {
            version.clone()
        } else {
            name
        },
        ram_mb: ram_mb.clamp(512, 65536),
        jvm_args,
    };
    meta::write(&dir, &m)?;
    Ok(InstallationInfo::from_layout(&layout(), &version))
}

/// Delete an installation and its folder entirely — as if it never existed.
///
/// The shared resourcepacks/shaderpacks/saves are **junctions**, so we remove
/// those links first (never following them) to guarantee the real vanilla
/// folders they point at are untouched. The GP installations root and all other
/// installations are left intact.
#[tauri::command]
pub fn delete_installation(version: String) -> Result<(), String> {
    let dir = layout().version_dir(&version);
    if !dir.exists() {
        return Ok(());
    }

    // Safety: unlink the junctions before recursively deleting, so a vanilla
    // saves/packs folder can never be followed into and wiped.
    for name in ["resourcepacks", "shaderpacks", "saves"] {
        let _ = links::remove_link(&dir.join(name));
    }

    std::fs::remove_dir_all(&dir).map_err(|e| format!("delete {dir:?}: {e}"))
}
