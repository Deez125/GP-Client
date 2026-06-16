//! The mods manifest (GitHub-hosted system, per mod-system-spec.md).
//!
//! `source` describes where mods live; `versions.<v>.required` and `.optional`
//! list them. Actual jar files are discovered by listing the GitHub folders
//! (see `github.rs`) — the manifest only stores friendly names + filename
//! prefixes, so updating a mod is just replacing the file in the repo.
//!
//! Some fields here are only used by the (upcoming) optional-mods UI, hence the
//! module-wide dead-code allowance.
#![allow(dead_code)]

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// The only hardcoded URL in the whole system (per the spec).
pub const MANIFEST_URL: &str =
    "https://raw.githubusercontent.com/Deez125/GP-Client/main/manifest.json";

#[derive(Debug, Deserialize)]
pub struct Manifest {
    pub source: Source,
    pub versions: HashMap<String, VersionEntry>,
}

#[derive(Debug, Deserialize)]
pub struct Source {
    pub raw_base: String,
    pub api_base: String,
    pub branch: String,
    pub required_dir: String,
    pub optional_dir: String,
    pub images_dir: String,
}

#[derive(Debug, Deserialize)]
pub struct VersionEntry {
    #[serde(default)]
    pub required: Vec<RequiredMod>,
    #[serde(default)]
    pub optional: Vec<OptionalCategory>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RequiredMod {
    pub name: String,
    #[serde(default)]
    pub jar: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OptionalCategory {
    pub category: String,
    #[serde(default)]
    pub mods: Vec<OptionalMod>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OptionalMod {
    pub name: String,
    #[serde(default)]
    pub jar: Option<String>,
    #[serde(default)]
    pub image: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    /// Explicit download URL override (externally-hosted big mods).
    #[serde(default)]
    pub url: Option<String>,
    /// Shown but not installable (e.g. too big to host yet).
    #[serde(default)]
    pub inactive: bool,
}

impl Manifest {
    fn expand(&self, template: &str, version: &str) -> String {
        template.replace("{version}", version)
    }

    /// GitHub contents-API URL for listing the version's `required/` folder.
    pub fn required_api_url(&self, version: &str) -> String {
        format!(
            "{}{}?ref={}",
            self.source.api_base,
            self.expand(&self.source.required_dir, version),
            self.source.branch
        )
    }

    /// GitHub contents-API URL for listing the version's `optional/` folder.
    pub fn optional_api_url(&self, version: &str) -> String {
        format!(
            "{}{}?ref={}",
            self.source.api_base,
            self.expand(&self.source.optional_dir, version),
            self.source.branch
        )
    }

    /// Raw URL for an optional mod's preview image.
    pub fn image_url(&self, version: &str, image: &str) -> String {
        format!(
            "{}{}/{}",
            self.source.raw_base,
            self.expand(&self.source.images_dir, version),
            image
        )
    }
}

pub async fn fetch(client: &reqwest::Client) -> Result<Manifest, String> {
    let resp = client
        .get(MANIFEST_URL)
        .send()
        .await
        .map_err(|e| format!("fetch manifest: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("fetch manifest: HTTP {}", resp.status()));
    }
    resp.json().await.map_err(|e| format!("parse manifest: {e}"))
}
