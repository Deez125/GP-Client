import { useEffect, useState } from "react";
import { getSettings, setSettings, type Settings } from "../lib/settings";

// Loads settings once on mount and persists every change immediately. `settings`
// is null until the first load resolves.
export function useSettings() {
  const [settings, setLocal] = useState<Settings | null>(null);

  useEffect(() => {
    getSettings()
      .then(setLocal)
      .catch(() => {});
  }, []);

  // Apply a partial change optimistically and write the whole object back.
  function update(patch: Partial<Settings>) {
    setLocal((prev) => {
      if (!prev) return prev;
      const next = { ...prev, ...patch };
      setSettings(next).catch(() => {});
      return next;
    });
  }

  return { settings, update };
}
