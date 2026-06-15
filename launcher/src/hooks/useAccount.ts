// Centralized account state: tries a silent sign-in on startup, exposes
// sign-in / sign-out. Shared by the sidebar account box and the Play button.
import { useCallback, useEffect, useState } from "react";
import {
  asAuthError,
  authLogin,
  authLoginSilent,
  authLogout,
  type AuthError,
  type MinecraftProfile,
} from "../lib/auth";

export interface Account {
  profile: MinecraftProfile | null;
  busy: boolean;
  error: AuthError | null;
  signIn: () => Promise<void>;
  signOut: () => Promise<void>;
}

export function useAccount(): Account {
  const [profile, setProfile] = useState<MinecraftProfile | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<AuthError | null>(null);

  useEffect(() => {
    (async () => {
      try {
        const p = await authLoginSilent();
        if (p) setProfile(p);
      } catch {
        // Silent failure is fine — user just stays signed out.
      }
    })();
  }, []);

  const signIn = useCallback(async () => {
    setBusy(true);
    setError(null);
    try {
      setProfile(await authLogin());
    } catch (e) {
      setError(asAuthError(e));
    } finally {
      setBusy(false);
    }
  }, []);

  const signOut = useCallback(async () => {
    await authLogout().catch(() => {});
    setProfile(null);
    setError(null);
  }, []);

  return { profile, busy, error, signIn, signOut };
}
