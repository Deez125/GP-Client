// Import game settings from an existing Minecraft profile into a GP Client
// installation. Mirrors `profile_import.rs`.
import { invoke } from "@tauri-apps/api/core";

export interface SourceProfile {
  name: string;
  path: string;
}

export interface ImportItems {
  options: boolean;
  shaders: boolean;
  servers: boolean;
  schematics: boolean;
  xaero: boolean;
}

export interface ImportReport {
  imported: string[];
  skipped: string[];
}

/** Profiles discovered from the vanilla launcher (plus the default .minecraft). */
export const listSourceProfiles = () =>
  invoke<SourceProfile[]>("list_source_profiles");

/** Copy the selected items from `source` into the GP Client `version` install. */
export const importProfileSettings = (
  version: string,
  source: string,
  items: ImportItems,
) => invoke<ImportReport>("import_profile_settings", { version, source, items });
