// Single source of truth for every user-facing brand string in the frontend.
//
// The actual values live in /brand.json at the launcher root so that BOTH the
// React frontend (this file) and the Rust backend (src-tauri/src/brand.rs) read
// the exact same data. To rename the product, edit brand.json only.
import brandJson from "../../brand.json";

export interface Brand {
  /** Full product name shown in headings and the window title. */
  appName: string;
  /** Short form for tight spaces (e.g. badges). */
  shortName: string;
  /** OS window title. */
  windowTitle: string;
  /** One-line marketing line. */
  tagline: string;
  /** Reverse-DNS bundle id (must mirror tauri.conf.json `identifier`). */
  bundleIdentifier: string;
  /** Name of the top-level installations folder on disk. */
  installationsFolderName: string;
  /** Name of the shared-support folder (assets/libraries), sibling to installations. */
  sharedFolderName: string;
  /** UI label for mods the launcher manages. */
  managedModsLabel: string;
  /** UI label for mods the user added themselves. */
  userModsLabel: string;
}

export const brand: Brand = brandJson;
