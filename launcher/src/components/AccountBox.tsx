import { useEffect, useState } from "react";
import type { Account } from "../hooks/useAccount";
import { getSkinFace } from "../lib/skin";

// Top account area: signed-in user (with their Minecraft skin face), or a
// sign-in button.
export function AccountBox({ account }: { account: Account }) {
  const { profile, busy, error, signIn, signOut } = account;
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
            {error.kind}
          </span>
        )}
      </div>
    </div>
  );
}
