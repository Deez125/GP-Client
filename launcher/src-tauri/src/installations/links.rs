//! Directory shortcut creation (the spec's "junctions on Windows" requirement).
//!
//! On Windows we create NTFS **directory junctions**, which (unlike symlinks)
//! need no administrator rights or Developer Mode. On other platforms we fall
//! back to a normal directory symlink.

use std::path::Path;

/// Ensure a directory shortcut exists at `link` pointing to `target`.
///
/// - Creates `target` if it doesn't exist (so the junction is never dangling).
/// - No-op if something already exists at `link` (real dir or existing link).
pub fn ensure_dir_link(link: &Path, target: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(target)?;

    // `symlink_metadata` does not follow the link, so this is true even for an
    // already-present (possibly dangling) junction — don't try to recreate it.
    if std::fs::symlink_metadata(link).is_ok() {
        return Ok(());
    }
    if let Some(parent) = link.parent() {
        std::fs::create_dir_all(parent)?;
    }
    create_link(link, target)
}

#[cfg(windows)]
fn create_link(link: &Path, target: &Path) -> std::io::Result<()> {
    // junction::create(target, junction_point)
    junction::create(target, link)
}

#[cfg(not(windows))]
fn create_link(link: &Path, target: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

/// Remove a directory junction/symlink at `link` **without** touching its
/// target. Critical for safe deletion — a vanilla saves/packs folder must never
/// be followed into. No-op if nothing is there.
pub fn remove_link(link: &Path) -> std::io::Result<()> {
    if std::fs::symlink_metadata(link).is_err() {
        return Ok(());
    }
    // On Windows a junction is a reparse-point directory: RemoveDirectory
    // removes the junction itself, never the target's contents.
    #[cfg(windows)]
    {
        std::fs::remove_dir(link)
    }
    // On Unix a directory symlink is unlinked with remove_file.
    #[cfg(not(windows))]
    {
        std::fs::remove_file(link)
    }
}
