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

const crafatarFace = (uuid: string) =>
  `https://crafatar.com/avatars/${uuid}?size=64&overlay`;

// Top account area: the active signed-in user (with their Minecraft skin face)
// and a switcher listing every stored account, or a sign-in button.
export function AccountBox({ account }: { account: Account }) {
  const {
    profile,
    accounts,
    activeUuid,
    busy,
    initializing,
    error,
    addAccount,
    switchAccount,
    signOut,
  } = account;
  // Skin faces keyed by uuid, fetched for every stored account.
  const [faces, setFaces] = useState<Record<string, string>>({});
  const [menuOpen, setMenuOpen] = useState(false);

  useEffect(() => {
    let active = true;
    for (const a of accounts) {
      if (faces[a.uuid]) continue;
      getSkinFace(a.uuid)
        .then((f) => active && setFaces((m) => ({ ...m, [a.uuid]: f })))
        .catch(
          () => active && setFaces((m) => ({ ...m, [a.uuid]: crafatarFace(a.uuid) })),
        );
    }
    return () => {
      active = false;
    };
  }, [accounts, faces]);

  if (profile) {
    const triggerFace = faces[profile.uuid];
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
              {triggerFace ? (
                <img className="account-avatar" src={triggerFace} alt="" />
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
                  {accounts.map((a) => {
                    const isActive = a.uuid === activeUuid;
                    const face = faces[a.uuid];
                    return (
                      <div
                        className="account-row"
                        key={a.uuid}
                        role="button"
                        onClick={() => {
                          if (!isActive) switchAccount(a.uuid);
                          setMenuOpen(false);
                        }}
                      >
                        <span
                          className={`account-dot${isActive ? "" : " inactive"}`}
                        />
                        {face ? (
                          <img className="account-row-avatar" src={face} alt="" />
                        ) : (
                          <div className="account-row-avatar placeholder" />
                        )}
                        <span className="account-row-name" title={a.username}>
                          {a.username}
                        </span>
                        <Tooltip text="Sign out">
                          <button
                            className="account-signout"
                            onClick={(e) => {
                              e.stopPropagation();
                              signOut(a.uuid);
                            }}
                          >
                            <LuDoorOpen />
                          </button>
                        </Tooltip>
                      </div>
                    );
                  })}
                  <button
                    className="account-add"
                    onClick={() => {
                      setMenuOpen(false);
                      addAccount();
                    }}
                    disabled={busy}
                  >
                    <LuPlus />
                    {busy ? "Signing in…" : "Add account"}
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
        <button className="link-btn" onClick={addAccount} disabled={busy}>
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
