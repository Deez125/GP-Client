import { useState } from "react";
import type { Account } from "../hooks/useAccount";
import { AccountBox } from "./AccountBox";
import { SkinRender } from "./SkinRender";
import { ChangeSkinDialog } from "./ChangeSkinDialog";

// Right sidebar: account at top, then a static (paused, angled) 3D render of the
// player's skin. "Change skin" opens the skin library popup.
export function SkinSidebar({ account }: { account: Account }) {
  const profile = account.profile;
  const [open, setOpen] = useState(false);

  return (
    <aside className="sidebar-right">
      <AccountBox account={account} />

      <div className="skin-preview">
        {profile ? (
          <SkinRender uuid={profile.uuid} width={190} height={300} animated={false} />
        ) : (
          <div className="skin-placeholder" />
        )}
      </div>
      <button className="btn" onClick={() => setOpen(true)} disabled>
        Change skin
      </button>
      <p className="muted small">[WIP]</p>

      {open && profile && (
        <ChangeSkinDialog
          uuid={profile.uuid}
          username={profile.username}
          onClose={() => setOpen(false)}
        />
      )}
    </aside>
  );
}
