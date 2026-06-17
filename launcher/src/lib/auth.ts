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

/** One stored account (non-secret). The refresh token stays in the keychain. */
export interface AccountRecord {
  uuid: string;
  username: string;
}

/** All stored accounts plus the active one's uuid (null if none). */
export interface AccountList {
  accounts: AccountRecord[];
  active: string | null;
}

export const authStatus = () => invoke<AuthStatus>("auth_status");

/** Interactive browser sign-in (full chain). Adds an account, makes it active. */
export const authLogin = () => invoke<MinecraftProfile>("auth_login");

/** Silent sign-in for the active account; null if no account is active. */
export const authLoginSilent = () =>
  invoke<MinecraftProfile | null>("auth_login_silent");

/** List all stored accounts + which is active. */
export const authListAccounts = () => invoke<AccountList>("auth_list_accounts");

/** Make a stored account active and return its profile (refreshes if needed). */
export const authSwitchAccount = (uuid: string) =>
  invoke<MinecraftProfile>("auth_switch_account", { uuid });

/** Remove one account by uuid. */
export const authLogout = (uuid: string) =>
  invoke<void>("auth_logout", { uuid });
