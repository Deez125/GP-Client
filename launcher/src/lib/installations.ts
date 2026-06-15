// Typed wrappers around the installations Rust commands.
import { invoke } from "@tauri-apps/api/core";

export interface InstallationInfo {
  version: string;
  name: string;
  path: string;
  mods_path: string;
  ram_mb: number;
  jvm_args: string;
}

export const installationsRoot = () => invoke<string>("installations_root");

export const createInstallation = (version: string) =>
  invoke<InstallationInfo>("create_installation", { version });

export const listInstallations = () =>
  invoke<InstallationInfo[]>("list_installations_cmd");

export const openInstallationsFolder = () =>
  invoke<void>("open_installations_folder");

export const openInstallationFolder = (version: string) =>
  invoke<void>("open_installation_folder", { version });

export const updateInstallation = (
  version: string,
  name: string,
  ramMb: number,
  jvmArgs: string,
) =>
  invoke<InstallationInfo>("update_installation", {
    version,
    name,
    ramMb,
    jvmArgs,
  });

export const deleteInstallation = (version: string) =>
  invoke<void>("delete_installation", { version });
