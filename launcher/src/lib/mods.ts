// Wrappers for the mods catalog + optional selection commands.
import { invoke } from "@tauri-apps/api/core";

export interface OptionalModView {
  name: string;
  description: string | null;
  image_url: string | null;
  available: boolean;
  inactive: boolean;
  selected: boolean;
}

export interface OptionalCategoryView {
  category: string;
  mods: OptionalModView[];
}

export interface VersionMods {
  required: string[];
  optional: OptionalCategoryView[];
}

export const getVersionMods = (version: string) =>
  invoke<VersionMods>("get_version_mods", { version });

export const setOptionalMods = (version: string, selected: string[]) =>
  invoke<void>("set_optional_mods", { version, selected });
