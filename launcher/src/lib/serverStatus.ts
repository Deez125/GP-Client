// Wrapper around the server-list-ping command. Never rejects from the backend's
// side (an unreachable server resolves to `online: false`).
import { invoke } from "@tauri-apps/api/core";

export interface ServerStatus {
  online: boolean;
  players_online: number;
  players_max: number;
  /** Player names from the status "sample" (often partial; may be empty). */
  sample: string[];
  /** MOTD flattened to plain text. */
  motd: string;
  /** Round-trip latency in ms. */
  ping_ms: number;
}

/** Ping a Minecraft server (host or host:port) for its live status. */
export const serverStatus = (address: string) =>
  invoke<ServerStatus>("server_status", { address });
