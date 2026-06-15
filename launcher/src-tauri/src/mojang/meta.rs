//! Mojang "piston-meta" data model: the version manifest and per-version JSON.
//!
//! We model only the fields the launcher actually uses. The `arguments` block
//! is the modern (1.13+) format — a list whose entries are either a plain
//! string or a `{ rules, value }` object.

// Some fields mirror Mojang's schema for completeness even though the launcher
// doesn't read them all yet, so dead-code is allowed module-wide here.
#![allow(dead_code)]

use serde::Deserialize;
use std::collections::HashMap;

pub const VERSION_MANIFEST_URL: &str =
    "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

#[derive(Debug, Deserialize)]
pub struct VersionManifest {
    pub latest: LatestVersions,
    pub versions: Vec<ManifestVersion>,
}

#[derive(Debug, Deserialize)]
pub struct LatestVersions {
    pub release: String,
    #[allow(dead_code)]
    pub snapshot: String,
}

#[derive(Debug, Deserialize)]
pub struct ManifestVersion {
    pub id: String,
    pub url: String,
}

impl VersionManifest {
    pub fn find(&self, id: &str) -> Option<&ManifestVersion> {
        self.versions.iter().find(|v| v.id == id)
    }
}

#[derive(Debug, Deserialize)]
pub struct VersionDetails {
    pub id: String,
    #[serde(rename = "mainClass")]
    pub main_class: String,
    #[serde(rename = "type")]
    pub version_type: String,
    #[serde(rename = "javaVersion")]
    pub java_version: JavaVersion,
    #[serde(rename = "assetIndex")]
    pub asset_index: AssetIndexRef,
    pub assets: String,
    pub downloads: Downloads,
    pub libraries: Vec<Library>,
    pub arguments: Arguments,
}

#[derive(Debug, Deserialize)]
pub struct JavaVersion {
    #[allow(dead_code)]
    pub component: String,
    #[serde(rename = "majorVersion")]
    pub major_version: u32,
}

#[derive(Debug, Deserialize)]
pub struct AssetIndexRef {
    pub id: String,
    pub sha1: String,
    #[allow(dead_code)]
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct Downloads {
    pub client: DownloadEntry,
}

#[derive(Debug, Deserialize)]
pub struct DownloadEntry {
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct Library {
    pub name: String,
    pub downloads: Option<LibraryDownloads>,
    #[serde(default)]
    pub rules: Vec<Rule>,
    /// Old-style natives map (classifier key per OS), e.g. {"windows":"natives-windows"}.
    #[serde(default)]
    pub natives: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct LibraryDownloads {
    pub artifact: Option<Artifact>,
    #[serde(default)]
    pub classifiers: HashMap<String, Artifact>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Artifact {
    pub path: Option<String>,
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Rule {
    pub action: String,
    pub os: Option<OsRule>,
    pub features: Option<HashMap<String, bool>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct OsRule {
    pub name: Option<String>,
    pub arch: Option<String>,
    #[allow(dead_code)]
    pub version: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Arguments {
    #[serde(default)]
    pub game: Vec<ArgEntry>,
    #[serde(default)]
    pub jvm: Vec<ArgEntry>,
}

/// An argument is either a literal string or a conditional `{ rules, value }`.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum ArgEntry {
    Literal(String),
    Conditional {
        rules: Vec<Rule>,
        value: ArgValue,
    },
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum ArgValue {
    Single(String),
    Many(Vec<String>),
}

impl ArgValue {
    pub fn into_vec(self) -> Vec<String> {
        match self {
            ArgValue::Single(s) => vec![s],
            ArgValue::Many(v) => v,
        }
    }
}

/// The asset index file (list of content-addressed objects).
#[derive(Debug, Deserialize)]
pub struct AssetIndex {
    pub objects: HashMap<String, AssetObject>,
}

#[derive(Debug, Deserialize)]
pub struct AssetObject {
    pub hash: String,
    pub size: u64,
}
