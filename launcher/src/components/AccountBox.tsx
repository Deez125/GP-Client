import { useEffect, useState } from "react";
import { LuChevronDown, LuDoorOpen, LuPlus } from "react-icons/lu";
import { Tooltip } from "./Tooltip";
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
  const [menuOpen, setMenuOpen] = useState(false);

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
      <div className="account-block">
        <p className="account-label">Playing as</p>
        <div className="account-anchor">
          <div className={`account-box${menuOpen ? " open" : ""}`}>
            <div
              className="account"
              role="button"
              onClick={() => setMenuOpen((o) => !o)}
            >
              {face ? (
                <img className="account-avatar" src={face} alt="" />
              ) : (
                <div className="account-avatar placeholder" />
              )}
              <div className="account-meta">
                <strong title={profile.username}>{profile.username}</strong>
                <span className="account-sub">Minecraft account</span>
              </div>
              <LuChevronDown
                className={`dropdown-arrow${menuOpen ? " open" : ""}`}
              />
            </div>

            {/* Always rendered so the box can animate its height. */}
            <div className="account-menu">
              <div className="account-menu-clip">
                <div className="account-menu-content">
                  <div
                    className="account-row"
                    role="button"
                    onClick={() => setMenuOpen(false)}
                  >
                    <span className="account-dot" />
                    {face ? (
                      <img className="account-row-avatar" src={face} alt="" />
                    ) : (
                      <div className="account-row-avatar placeholder" />
                    )}
                    <span className="account-row-name" title={profile.username}>
                      {profile.username}
                    </span>
                    <Tooltip text="Sign out">
                      <button
                        className="account-signout"
                        onClick={(e) => {
                          e.stopPropagation();
                          signOut();
                        }}
                      >
                        <LuDoorOpen />
                      </button>
                    </Tooltip>
                  </div>
                  <button
                    className="account-add"
                    onClick={signIn}
                    disabled={busy}
                  >
                    <LuPlus />
                    Add account
                  </button>
                </div>
              </div>
            </div>
          </div>
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
