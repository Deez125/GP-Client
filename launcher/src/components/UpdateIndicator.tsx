import { useEffect, useState } from "react";
import {
  checkForUpdate,
  installUpdate,
  type UpdateInfo,
} from "../lib/updates";

// Update-available popup, bottom-right. Checks GitHub on startup; if there's a
// newer release, "Update now" downloads + runs its installer (the app exits so
// the installer can replace it).
export function UpdateIndicator() {
  const [info, setInfo] = useState<UpdateInfo | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    checkForUpdate()
      .then(setInfo)
      .catch(() => {});
  }, []);

  async function onUpdate() {
    if (!info?.url) return;
    setBusy(true);
    setError(null);
    try {
      await installUpdate(info.url);
      // On success the app exits, so we won't get here.
    } catch (e) {
      setError(String(e));
      setBusy(false);
    }
  }

  if (!info?.available) return null;

  return (
    <div className="update-toast">
      <div className="update-toast-title">Update available</div>
      <div className="update-toast-text">
        {error ?? "A new version of GP Client is ready to install."}
      </div>
      <button
        className="update-toast-btn"
        onClick={onUpdate}
        disabled={busy || !info.url}
      >
        {busy ? "Updating…" : "Update now"}
      </button>
    </div>
  );
}
