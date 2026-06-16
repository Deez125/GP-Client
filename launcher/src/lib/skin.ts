// Fetch a player's face (from the Mojang skin) as a PNG data URL.
import { invoke } from "@tauri-apps/api/core";

export const getSkinFace = (uuid: string) =>
  invoke<string>("get_skin_face", { uuid });

/** Full skin PNG as a data URL (for the 3D viewer). */
export const getSkin = (uuid: string) => invoke<string>("get_skin", { uuid });

export interface PlayerTextures {
  skin: string;
  cape: string | null;
}

/** Skin + active cape (both data URLs) for the 3D viewer. */
export const getPlayerTextures = (uuid: string) =>
  invoke<PlayerTextures>("get_player_textures", { uuid });

export interface CapeEntry {
  id: string;
  name: string;
  active: boolean;
  /** Cropped front-face preview (for the grid). */
  data_url: string;
  /** Full cape PNG (for the 3D model). */
  texture: string;
}

/** Capes owned by the signed-in account, each with a preview. */
export const getCapes = () => invoke<CapeEntry[]>("get_capes");

/** Activate a cape by id, or pass null / "none" to hide the cape. */
export const setCape = (cape: string | null) =>
  invoke<void>("set_cape", { cape });

export interface SkinLibEntry {
  id: string;
  name: string;
  model: "auto-detect" | "default" | "slim";
  added: number;
  data_url: string;
  /** Preferred cape: a cape id, "none", or null (leave unchanged). */
  cape: string | null;
}

/** Saved skin library (seeds Steve/Alex/Current), newest first. */
export const listSkins = (uuid: string) =>
  invoke<SkinLibEntry[]>("list_skins", { uuid });

export const renameSkin = (uuid: string, id: string, name: string) =>
  invoke<void>("rename_skin", { uuid, id, name });

/** Upload a library skin to the signed-in account. variant: "classic" | "slim". */
export const applySkin = (
  uuid: string,
  id: string,
  variant: "classic" | "slim",
) => invoke<void>("apply_skin", { uuid, id, variant });

/** Import a PNG file (64x64 or 64x32) into the library; returns the new id. */
export const importSkin = (
  uuid: string,
  path: string,
  name: string,
  model: "default" | "slim",
) => invoke<string>("import_skin", { uuid, path, name, model });

/** Update a library skin's name, model, and cape preference. */
export const updateSkin = (
  uuid: string,
  id: string,
  name: string,
  model: "default" | "slim",
  cape: string | null,
) => invoke<void>("update_skin", { uuid, id, name, model, cape });

/** Replace a library skin's PNG with the file at `path`. */
export const replaceSkinFile = (uuid: string, id: string, path: string) =>
  invoke<void>("replace_skin_file", { uuid, id, path });

/** Delete a library skin. */
export const deleteSkin = (uuid: string, id: string) =>
  invoke<void>("delete_skin", { uuid, id });
