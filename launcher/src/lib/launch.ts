// Wrapper around the launch (step 6) command + progress events.
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface LaunchProgress {
  phase: string;
  current: number;
  total: number;
  message: string;
}

/** Prepare files and launch the given Minecraft version. Resolves when the
 *  game process has been spawned (downloads happen during the call). Pass
 *  `server` (host or host:port) to quick-join straight into a server. */
export const launchVersion = (version: string, server?: string) =>
  invoke<void>("launch_version", { version, server });

/** Subscribe to download/launch progress. Returns an unlisten function. */
export const onLaunchProgress = (
  cb: (p: LaunchProgress) => void,
): Promise<UnlistenFn> =>
  listen<LaunchProgress>("launch://progress", (e) => cb(e.payload));
