import { useEffect, useState } from "react";
import type { Account } from "../hooks/useAccount";
import type { AuthError } from "../lib/auth";
import { getSkinFace } from "../lib/skin";

// Turn a raw auth error into short, friendly text for the account box.
function authMessage(error: AuthError): string {
  const m = (error.message || "").toLowerCase();
  if (m.includes("429") || m.includes("too many requests"))
    return "Too many tries. Wait a bit and sign in again.";
  if (error.kind === "cancelled" || m.includes("cancel"))
    return "Sign-in canceled.";
  if (m.includes("network") || m.includes("sending request") || m.includes("timed out"))
    return "Network problem. Try again.";
  return "Sign-in failed. Try again.";
}

// Top account area: signed-in user (with their Minecraft skin face), or a
// sign-in button.
export function AccountBox({ account }: { account: Account }) {
  const { profile, busy, initializing, error, signIn, signOut } = account;
  const [face, setFace] = useState<string | null>(null);

  useEffect(() => {
    setFace(null);
    if (!profile) return;
    getSkinFace(profile.uuid)
      .then(setFace)
      // Fall back to Crafatar's face render if the Mojang fetch fails
      // (e.g. default skin / network hiccup).
      .catch(() =>
        setFace(`https://crafatar.com/avatars/${profile.uuid}?size=64&overlay`),
      );
  }, [profile]);

  if (profile) {
    return (
      <div className="account">
        {face ? (
          <img className="account-avatar" src={face} alt="" />
        ) : (
          <div className="account-avatar placeholder" />
        )}
        <div className="account-meta">
          <strong title={profile.username}>{profile.username}</strong>
          <button className="link-btn" onClick={signOut}>
            Sign out
          </button>
        </div>
      </div>
    );
  }

  // While restoring the session on startup/refresh, show a neutral loading
  // state instead of flashing "Not signed in".
  if (initializing) {
    return (
      <div className="account">
        <div className="account-avatar placeholder" />
        <div className="account-meta">
          <strong className="muted">Loading…</strong>
        </div>
      </div>
    );
  }

  return (
    <div className="account">
      <div className="account-avatar placeholder" />
      <div className="account-meta">
        <strong>Not signed in</strong>
        <button className="link-btn" onClick={signIn} disabled={busy}>
          {busy ? "Signing in…" : "Sign in"}
        </button>
        {error && (
          <span className="account-error" title={error.message}>
            {authMessage(error)}
          </span>
        )}
      </div>
    </div>
  );
}
