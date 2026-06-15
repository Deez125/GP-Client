// Version list from the mods manifest (newest first), for the version picker.
import { invoke } from "@tauri-apps/api/core";

export const listModVersions = () => invoke<string[]>("list_mod_versions");
