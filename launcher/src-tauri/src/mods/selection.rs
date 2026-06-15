//! Per-installation record of which OPTIONAL mods the user has ticked.
//! Stored at `versions/<version>/gpclient-optional.json`. Selection is by the
//! optional mod's display `name` (stable identity in the manifest).

use std::path::Path;

use serde::{Deserialize, Serialize};

pub const SELECTION_FILE: &str = "gpclient-optional.json";

#[derive(Default, Serialize, Deserialize)]
struct Selection {
    #[serde(default)]
    selected: Vec<String>,
}

/// Names of the optional mods the user has enabled (empty if none/never set).
pub fn read(version_dir: &Path) -> Vec<String> {
    match std::fs::read_to_string(version_dir.join(SELECTION_FILE)) {
        Ok(text) => serde_json::from_str::<Selection>(&text)
            .map(|s| s.selected)
            .unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

pub fn write(version_dir: &Path, selected: &[String]) -> Result<(), String> {
    let s = Selection {
        selected: selected.to_vec(),
    };
    let text = serde_json::to_string_pretty(&s).map_err(|e| e.to_string())?;
    std::fs::write(version_dir.join(SELECTION_FILE), text).map_err(|e| e.to_string())
}
