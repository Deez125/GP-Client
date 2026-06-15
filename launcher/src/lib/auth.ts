// Typed wrappers around the Rust auth commands. Each command returns a
// Result on the Rust side; `invoke` resolves on Ok and rejects with the
// serialized AuthError ({ kind, message }) on Err.
import { invoke } from "@tauri-apps/api/core";

export interface MinecraftProfile {
  access_token: string;
  uuid: string;
  username: string;
}

export interface AuthStatus {
  client_id_configured: boolean;
  has_cached_session: boolean;
}

export interface AuthError {
  kind: string;
  message: string;
}

/** Narrow an unknown thrown value into an AuthError shape. */
export function asAuthError(e: unknown): AuthError {
  if (e && typeof e === "object" && "message" in e && "kind" in e) {
    return e as AuthError;
  }
  return { kind: "other", message: String(e) };
}

export const authStatus = () => invoke<AuthStatus>("auth_status");

/** Interactive browser sign-in (full chain). */
export const authLogin = () => invoke<MinecraftProfile>("auth_login");

/** Silent sign-in from the stored refresh token; null if none stored. */
export const authLoginSilent = () =>
  invoke<MinecraftProfile | null>("auth_login_silent");

export const authLogout = () => invoke<void>("auth_logout");
