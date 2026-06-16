//! Launcher-wide settings, persisted as `settings.json` in the shared
//! `<.minecraft>/GP Client` folder. Loaded on demand (cheap) rather than held in
//! global state, so every read sees the latest on-disk values.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::installations::shared_root;

/// All user-configurable launcher settings. `#[serde(default)]` on the struct
/// means any field missing from an older `settings.json` falls back to its
/// `Default` value, so adding new settings never breaks existing files.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    // --- General ---
    /// What the launcher window does once the game starts:
    /// "keep" | "minimize" | "close".
    pub launch_behavior: String,
    /// Restore/focus the launcher window when the game exits (ignored when the
    /// launcher was closed on launch).
    pub reopen_on_close: bool,
    /// Keep running in the tray instead of fully quitting on window close.
    pub close_to_tray: bool,

    // --- Updates ---
    pub check_updates_on_startup: bool,
    /// Whether to receive pre-release updates. `None` means the user hasn't
    /// chosen, so the updater falls back to a version-based default (on for
    /// pre-release builds, off for full releases). A user choice persists.
    pub prerelease_updates: Option<bool>,

    // --- Game ---
    /// Default heap (GB) suggested for new installations.
    pub default_memory_gb: u32,

    // --- Appearance ---
    pub animated_background: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            launch_behavior: "keep".to_string(),
            reopen_on_close: false,
            close_to_tray: false,
            check_updates_on_startup: true,
            // Unset → the updater picks a default from the running build's
            // channel (pre-release build = on, full release = off).
            prerelease_updates: None,
            default_memory_gb: 6,
            animated_background: true,
        }
    }
}

fn settings_path() -> PathBuf {
    shared_root().join("settings.json")
}

/// Read settings from disk, falling back to defaults if the file is missing or
/// unreadable/corrupt (never fails — a bad file just yields defaults).
pub fn load() -> Settings {
    match std::fs::read_to_string(settings_path()) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
        Err(_) => Settings::default(),
    }
}

/// Persist settings, creating the shared folder if needed.
pub fn store(settings: &Settings) -> Result<(), String> {
    let path = settings_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("create settings dir: {e}"))?;
    }
    let text = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    std::fs::write(&path, text).map_err(|e| format!("write settings: {e}"))
}

// --- Tauri commands ---------------------------------------------------------

#[tauri::command]
pub fn get_settings() -> Settings {
    load()
}

#[tauri::command]
pub fn set_settings(settings: Settings) -> Result<(), String> {
    store(&settings)
}
