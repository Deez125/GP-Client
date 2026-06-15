// Fetch a player's face (from the Mojang skin) as a PNG data URL.
import { invoke } from "@tauri-apps/api/core";

export const getSkinFace = (uuid: string) =>
  invoke<string>("get_skin_face", { uuid });

/** Full skin PNG as a data URL (for the 3D viewer). */
export const getSkin = (uuid: string) => invoke<string>("get_skin", { uuid });

export interface SkinLibEntry {
  id: string;
  name: string;
  model: "auto-detect" | "default" | "slim";
  added: number;
  data_url: string;
}

/** Saved skin library (seeds Steve/Alex/Current), newest first. */
export const listSkins = (uuid: string) =>
  invoke<SkinLibEntry[]>("list_skins", { uuid });

export const renameSkin = (id: string, name: string) =>
  invoke<void>("rename_skin", { id, name });
