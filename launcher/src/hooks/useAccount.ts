// Centralized account state. Supports multiple signed-in accounts: tries a
// silent sign-in of the active account on startup, lists all stored accounts,
// and exposes add / switch / per-account sign-out. Shared by the sidebar
// account box and the Play button.
import { useCallback, useEffect, useState } from "react";
import {
  asAuthError,
  authListAccounts,
  authLogin,
  authLoginSilent,
  authLogout,
  authSwitchAccount,
  type AccountRecord,
  type AuthError,
  type MinecraftProfile,
} from "../lib/auth";

export interface Account {
  /** The active account's full profile (token + identity), or null. */
  profile: MinecraftProfile | null;
  /** Every stored account (for the switcher list). */
  accounts: AccountRecord[];
  /** UUID of the active account, or null. */
  activeUuid: string | null;
  busy: boolean;
  /** True until the startup silent sign-in resolves (avoids a signed-out flash). */
  initializing: boolean;
  error: AuthError | null;
  /** Interactive sign-in that ADDS a new account and makes it active. */
  addAccount: () => Promise<void>;
  /** Make an already-stored account active. */
  switchAccount: (uuid: string) => Promise<void>;
  /** Remove one account by uuid (falls back to another, or signed-out). */
  signOut: (uuid: string) => Promise<void>;
}

export function useAccount(): Account {
  const [profile, setProfile] = useState<MinecraftProfile | null>(null);
  const [accounts, setAccounts] = useState<AccountRecord[]>([]);
  const [activeUuid, setActiveUuid] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [initializing, setInitializing] = useState(true);
  const [error, setError] = useState<AuthError | null>(null);

  const refreshList = useCallback(async () => {
    try {
      const list = await authListAccounts();
      setAccounts(list.accounts);
      setActiveUuid(list.active);
    } catch {
      // Non-fatal: just leave the list as-is.
    }
  }, []);

  useEffect(() => {
    (async () => {
      try {
        const p = await authLoginSilent();
        if (p) setProfile(p);
      } catch {
        // Silent failure is fine — user just stays signed out.
      }
      await refreshList();
      setInitializing(false);
    })();
  }, [refreshList]);

  const addAccount = useCallback(async () => {
    setBusy(true);
    setError(null);
    try {
      const p = await authLogin();
      setProfile(p);
      await refreshList();
    } catch (e) {
      setError(asAuthError(e));
    } finally {
      setBusy(false);
    }
  }, [refreshList]);

  const switchAccount = useCallback(
    async (uuid: string) => {
      if (uuid === activeUuid) return;
      setBusy(true);
      setError(null);
      try {
        const p = await authSwitchAccount(uuid);
        setProfile(p);
        setActiveUuid(uuid);
        await refreshList();
      } catch (e) {
        setError(asAuthError(e));
      } finally {
        setBusy(false);
      }
    },
    [activeUuid, refreshList],
  );

  const signOut = useCallback(
    async (uuid: string) => {
      await authLogout(uuid).catch(() => {});
      // Reload whatever account is active now (or clear if none remain).
      try {
        const p = await authLoginSilent();
        setProfile(p ?? null);
      } catch {
        setProfile(null);
      }
      await refreshList();
      setError(null);
    },
    [refreshList],
  );

  return {
    profile,
    accounts,
    activeUuid,
    busy,
    initializing,
    error,
    addAccount,
    switchAccount,
    signOut,
  };
}
