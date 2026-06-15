//! Per-installation, launcher-only settings stored alongside each version:
//! a display name (does NOT rename the folder), a RAM allocation that drives
//! the `-Xmx` launch argument, and extra JVM arguments.
//!
//! Stored at `versions/<version>/gpclient-instance.json`.

use std::path::Path;

use serde::{Deserialize, Serialize};

pub const META_FILENAME: &str = "gpclient-instance.json";
pub const DEFAULT_RAM_MB: u32 = 4096;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceMeta {
    /// Launcher-only display name (folder stays the version id).
    pub name: String,
    /// Max heap in MB → `-Xmx<ram>m`.
    pub ram_mb: u32,
    /// Extra JVM arguments (whitespace-separated), appended at launch.
    pub jvm_args: String,
}

impl InstanceMeta {
    pub fn default_for(version: &str) -> Self {
        InstanceMeta {
            name: version.to_string(),
            ram_mb: DEFAULT_RAM_MB,
            jvm_args: String::new(),
        }
    }
}

fn meta_path(version_dir: &Path) -> std::path::PathBuf {
    version_dir.join(META_FILENAME)
}

/// Read the instance meta, falling back to defaults (named after `version`).
pub fn read(version_dir: &Path, version: &str) -> InstanceMeta {
    match std::fs::read_to_string(meta_path(version_dir)) {
        Ok(text) => serde_json::from_str(&text)
            .unwrap_or_else(|_| InstanceMeta::default_for(version)),
        Err(_) => InstanceMeta::default_for(version),
    }
}

pub fn write(version_dir: &Path, meta: &InstanceMeta) -> Result<(), String> {
    let text = serde_json::to_string_pretty(meta).map_err(|e| e.to_string())?;
    std::fs::write(meta_path(version_dir), text).map_err(|e| e.to_string())
}
