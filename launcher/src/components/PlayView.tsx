import { useEffect, useRef, useState } from "react";
import {
  launchVersion,
  onLaunchProgress,
  type LaunchProgress,
} from "../lib/launch";
import { listModVersions } from "../lib/versions";
import { listInstallations } from "../lib/installations";
import { ModsDialog } from "./ModsDialog";

// The Play tab: hero (selected server's image) + bottom bar (version dropdown +
// Mods + Play).
export function PlayView({
  signedIn,
  heroImage,
}: {
  signedIn: boolean;
  heroImage: string;
}) {
  const [versions, setVersions] = useState<string[]>([]);
  const [version, setVersion] = useState("");
  const [installed, setInstalled] = useState<Set<string>>(new Set());
  const [busy, setBusy] = useState(false);
  const [progress, setProgress] = useState<LaunchProgress | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [modsOpen, setModsOpen] = useState(false);
  // True when the Mods popup was opened as the first step of an install.
  const [installMode, setInstallMode] = useState(false);
  const unlisten = useRef<(() => void) | null>(null);

  // Hero crossfade: stack image layers; the newest fades in over the old, then
  // older layers are removed.
  const [heroLayers, setHeroLayers] = useState<{ key: number; src: string }[]>([]);
  const heroKey = useRef(0);
  useEffect(() => {
    heroKey.current += 1;
    const key = heroKey.current;
    setHeroLayers((prev) => [...prev, { key, src: heroImage }]);
  }, [heroImage]);

  function refreshInstalled() {
    listInstallations()
      .then((list) => setInstalled(new Set(list.map((i) => i.version))))
      .catch(() => {});
  }

  useEffect(() => {
    onLaunchProgress(setProgress).then((fn) => (unlisten.current = fn));
    return () => unlisten.current?.();
  }, []);

  useEffect(() => {
    listModVersions()
      .then((vs) => {
        setVersions(vs);
        if (vs.length > 0) setVersion(vs[0]);
      })
      .catch((e) => setError(`Couldn't load versions: ${e}`));
    refreshInstalled();
  }, []);

  async function onPlay() {
    setBusy(true);
    setError(null);
    setProgress(null);
    try {
      await launchVersion(version);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
      refreshInstalled();
    }
  }

  const pct =
    progress && progress.total > 0
      ? Math.round((progress.current / progress.total) * 100)
      : null;

  const running = busy && progress?.phase === "running";
  const isInstalled = installed.has(version);
  const playLabel = busy
    ? running
      ? "RUNNING…"
      : "WORKING…"
    : !isInstalled
      ? "INSTALL"
      : signedIn
        ? "PLAY"
        : "PLAY (OFFLINE)";

  return (
    <div className="play-view">
      <div className="hero">
        {heroLayers.map((l) => (
          <div
            key={l.key}
            className="hero-layer"
            style={{ backgroundImage: `url(${l.src})` }}
            onAnimationEnd={() =>
              setHeroLayers((prev) => prev.filter((x) => x.key >= l.key))
            }
          />
        ))}
      </div>

      {error && (
        <div className="error">
          <strong>Launch problem</strong>
          <p>{error}</p>
        </div>
      )}

      <div className="play-dock">
        {/* Progress panel slides up from behind the bar while busy. */}
        <div className={`play-progress-panel${busy ? " show" : ""}`}>
          <div className="pp-head">
            <span className="pp-msg">{progress?.message ?? "Working…"}</span>
            <span className="pp-pct">{pct !== null ? `${pct}%` : ""}</span>
          </div>
          <div className="progress-bar">
            <div
              className="progress-fill"
              style={{ width: pct !== null ? `${pct}%` : "100%" }}
            />
          </div>
        </div>

        <div className="play-bar">
          <div className="version-pick">
            <select
              value={version}
              onChange={(e) => setVersion(e.currentTarget.value)}
              disabled={busy || versions.length === 0}
            >
              {versions.length === 0 && <option value="">Loading…</option>}
              {versions.map((v, i) => (
                <option key={v} value={v}>
                  {v}
                  {i === 0 ? " (latest)" : ""}
                </option>
              ))}
            </select>
          </div>

          <button
            className="play-btn"
            onClick={() => {
              // First-time install: pick optional mods first, then continue.
              if (isInstalled) onPlay();
              else {
                setInstallMode(true);
                setModsOpen(true);
              }
            }}
            disabled={busy || !version}
          >
            {playLabel}
          </button>

          <div className="play-right">
            <button
              className="btn mods-btn"
              onClick={() => {
                setInstallMode(false);
                setModsOpen(true);
              }}
              disabled={busy || !version}
            >
              Mods
            </button>
          </div>
        </div>
      </div>

      {modsOpen && version && (
        <ModsDialog
          version={version}
          saveLabel={installMode ? "Continue" : "Save"}
          onSaved={installMode ? () => onPlay() : undefined}
          onClose={() => {
            setModsOpen(false);
            setInstallMode(false);
          }}
        />
      )}
    </div>
  );
}
