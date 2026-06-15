//! Single source of truth for user-facing brand strings on the Rust side.
//!
//! The values are NOT duplicated here — this module embeds `../brand.json`
//! (the same file the React frontend reads) at compile time and parses it into
//! a `Brand` struct. To rename the product, edit brand.json only; both the
//! frontend and this backend pick up the change.

use serde::{Deserialize, Serialize};

/// The contents of brand.json, baked into the binary at build time.
/// `CARGO_MANIFEST_DIR` is `.../launcher/src-tauri`, so `../brand.json`
/// resolves to the launcher-root brand file shared with the frontend.
const BRAND_JSON: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../brand.json"));

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Brand {
    /// Full product name shown in headings and the window title.
    pub app_name: String,
    /// Short form for tight spaces.
    pub short_name: String,
    /// OS window title.
    pub window_title: String,
    /// One-line marketing line.
    pub tagline: String,
    /// Reverse-DNS bundle id (mirrors tauri.conf.json `identifier`).
    pub bundle_identifier: String,
    /// Name of the top-level installations folder on disk.
    pub installations_folder_name: String,
    /// Name of the shared-support folder (assets/libraries), sibling to installations.
    pub shared_folder_name: String,
    /// UI label for mods the launcher manages.
    pub managed_mods_label: String,
    /// UI label for mods the user added themselves.
    pub user_mods_label: String,
}

/// Parse the embedded brand.json. Panics only if brand.json is malformed,
/// which is a build-time/authoring error rather than a runtime condition.
pub fn brand() -> Brand {
    serde_json::from_str(BRAND_JSON).expect("brand.json is present but malformed")
}
