//! The `.modsync.json` state file — the record of which jars GP installed, so
//! sync is non-destructive (we only ever remove our own previous jars, never
//! user-added mods). Ported from the Python tool's `sync.py`.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

pub const STATE_FILENAME: &str = ".modsync.json";

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct State {
    #[serde(default)]
    pub version: String,
    /// Filenames of the jars GP installed last sync.
    #[serde(default)]
    pub managed: Vec<String>,
    #[serde(default)]
    pub updated: u64,
}

fn state_path(mods_dir: &Path) -> PathBuf {
    mods_dir.join(STATE_FILENAME)
}

/// Read the previous state, or a default if absent/unreadable.
pub fn read(mods_dir: &Path) -> State {
    let path = state_path(mods_dir);
    match std::fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
        Err(_) => State::default(),
    }
}

/// Write the current state (the new list of GP-managed jars).
pub fn write(mods_dir: &Path, version: &str, mut managed: Vec<String>) -> Result<(), String> {
    managed.sort();
    let state = State {
        version: version.to_string(),
        managed,
        updated: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
    };
    let text = serde_json::to_string_pretty(&state).map_err(|e| e.to_string())?;
    std::fs::write(state_path(mods_dir), text).map_err(|e| e.to_string())
}
