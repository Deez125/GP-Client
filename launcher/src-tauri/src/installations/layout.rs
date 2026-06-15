//! Pure path math + directory/junction creation for the installations tree.
//!
//! `Layout` knows where everything lives relative to the install root and the
//! vanilla `.minecraft` folder. Each version gets real, isolated `mods/` and
//! `config/` folders, plus `resourcepacks`/`shaderpacks`/`saves` that are
//! **junctions pointing at the vanilla `.minecraft` folders** — so the user
//! sees their existing vanilla packs and worlds. GP Client never keeps its own
//! copies of those.

use std::path::{Path, PathBuf};

use super::links;

pub struct Layout {
    root: PathBuf,
    /// The vanilla `.minecraft` directory (junction targets live here).
    minecraft_dir: PathBuf,
}

impl Layout {
    pub fn new(root: PathBuf, minecraft_dir: PathBuf) -> Self {
        Layout {
            root,
            minecraft_dir,
        }
    }

    // Used by step 4+ and tooling.
    #[allow(dead_code)]
    pub fn root(&self) -> &Path {
        &self.root
    }

    // --- vanilla .minecraft folders (the real junction targets) -------------
    pub fn vanilla_resourcepacks(&self) -> PathBuf {
        self.minecraft_dir.join("resourcepacks")
    }
    pub fn vanilla_shaderpacks(&self) -> PathBuf {
        self.minecraft_dir.join("shaderpacks")
    }
    pub fn vanilla_saves(&self) -> PathBuf {
        self.minecraft_dir.join("saves")
    }

    // --- versions ------------------------------------------------------------
    /// A version installation lives directly under the root (e.g. `<root>/26.1.2`).
    pub fn version_dir(&self, version: &str) -> PathBuf {
        self.root.join(version)
    }
    /// The real, per-instance mods folder — never shared, never a junction.
    pub fn version_mods(&self, version: &str) -> PathBuf {
        self.version_dir(version).join("mods")
    }
    pub fn version_config(&self, version: &str) -> PathBuf {
        self.version_dir(version).join("config")
    }
    // Per-version junction points (link -> vanilla folder).
    pub fn version_resourcepacks(&self, version: &str) -> PathBuf {
        self.version_dir(version).join("resourcepacks")
    }
    pub fn version_shaderpacks(&self, version: &str) -> PathBuf {
        self.version_dir(version).join("shaderpacks")
    }
    pub fn version_saves(&self, version: &str) -> PathBuf {
        self.version_dir(version).join("saves")
    }

    // --- creation ------------------------------------------------------------

    /// Create the installations root. We do NOT create any `shared/` library of
    /// our own — the shared resourcepacks/shaderpacks/saves are the vanilla
    /// `.minecraft` ones (junction targets).
    pub fn ensure_base(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.root)?;
        Ok(())
    }

    /// Create a version: real `mods/` + `config/`, and junctions for
    /// `resourcepacks`/`shaderpacks`/`saves` pointing at the vanilla folders.
    /// Idempotent — existing folders/junctions are left alone.
    pub fn ensure_version(&self, version: &str) -> std::io::Result<()> {
        self.ensure_base()?;
        std::fs::create_dir_all(self.version_mods(version))?;
        std::fs::create_dir_all(self.version_config(version))?;

        links::ensure_dir_link(
            &self.version_resourcepacks(version),
            &self.vanilla_resourcepacks(),
        )?;
        links::ensure_dir_link(
            &self.version_shaderpacks(version),
            &self.vanilla_shaderpacks(),
        )?;
        links::ensure_dir_link(&self.version_saves(version), &self.vanilla_saves())?;
        Ok(())
    }
}
