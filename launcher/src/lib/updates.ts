// Update checking. The app's current version comes straight from Tauri
// (tauri.conf.json `version`); the latest version + installer URL come from the
// GitHub releases API. No hosted manifest needed.
import { invoke } from "@tauri-apps/api/core";
import { getVersion } from "@tauri-apps/api/app";

/** The currently-running app version, e.g. "0.1.2". */
export const getCurrentVersion = (): Promise<string> => getVersion();

export interface UpdateInfo {
  current: string;
  latest: string;
  available: boolean;
  url: string | null;
}

/** Check GitHub for a newer release. */
export const checkForUpdate = () => invoke<UpdateInfo>("check_for_update");

/** Download + run the new installer (the app exits so it can update itself). */
export const installUpdate = (url: string) =>
  invoke<void>("install_update", { url });

export interface ReleaseNotes {
  version: string;
  notes: string | null;
  url: string | null;
  date: string | null;
  prerelease: boolean | null;
}

/** Fetch the GitHub release notes (Markdown) for this app's version. */
export const getReleaseNotes = () => invoke<ReleaseNotes>("release_notes");
