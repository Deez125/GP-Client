// Launcher-wide settings, persisted by the Rust backend in the shared
// `<.minecraft>/GP Client/settings.json`. The shape mirrors `settings.rs`.
import { invoke } from "@tauri-apps/api/core";

export interface Settings {
  // General
  launch_behavior: "keep" | "minimize" | "close";
  reopen_on_close: boolean;
  close_to_tray: boolean;
  // Updates
  check_updates_on_startup: boolean;
  prerelease_updates: boolean;
  // Game
  default_memory_gb: number;
  // Appearance
  animated_background: boolean;
}

export const getSettings = () => invoke<Settings>("get_settings");

export const setSettings = (settings: Settings) =>
  invoke<void>("set_settings", { settings });
