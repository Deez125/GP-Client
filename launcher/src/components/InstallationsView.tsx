import { useEffect, useState } from "react";
import { LuPlay, LuSearch } from "react-icons/lu";
import {
  createInstallation,
  deleteInstallation,
  listInstallations,
  openInstallationFolder,
  openInstallationsFolder,
  updateInstallation,
  type InstallationInfo,
} from "../lib/installations";
import { launchVersion } from "../lib/launch";
import { listModVersions } from "../lib/versions";
import { EditInstallationDialog } from "./EditInstallationDialog";
import { Tooltip } from "./Tooltip";

// Folder / pencil / trash icons (small inline SVGs for crisp rendering).
const FolderIcon = () => (
  <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor">
    <path d="M10 4H4a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-8l-2-2z" />
  </svg>
);
const PencilIcon = () => (
  <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor">
    <path d="M3 17.25V21h3.75L17.81 9.94l-3.75-3.75L3 17.25zM20.71 7.04a1 1 0 0 0 0-1.41l-2.34-2.34a1 1 0 0 0-1.41 0l-1.83 1.83 3.75 3.75 1.83-1.83z" />
  </svg>
);
const TrashIcon = () => (
  <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor">
    <path d="M6 19a2 2 0 0 0 2 2h8a2 2 0 0 0 2-2V7H6v12zM19 4h-3.5l-1-1h-5l-1 1H5v2h14V4z" />
  </svg>
);

export function InstallationsView() {
  const [installs, setInstalls] = useState<InstallationInfo[]>([]);
  const [query, setQuery] = useState("");
  // Latest available version; used when creating a new installation.
  const [version, setVersion] = useState("");
  const [busy, setBusy] = useState(false);
  const [launching, setLaunching] = useState<string | null>(null);
  const [editing, setEditing] = useState<InstallationInfo | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function refresh() {
    try {
      setInstalls(await listInstallations());
    } catch (e) {
      setError(String(e));
    }
  }

  useEffect(() => {
    refresh();
    listModVersions()
      .then((vs) => {
        if (vs.length > 0) setVersion(vs[0]);
      })
      .catch(() => {});
  }, []);

  async function onCreate() {
    setBusy(true);
    setError(null);
    try {
      await createInstallation(version);
      await refresh();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function onPlay(i: InstallationInfo) {
    setLaunching(i.version);
    setError(null);
    try {
      await launchVersion(i.version);
    } catch (e) {
      setError(String(e));
    } finally {
      setLaunching(null);
    }
  }

  async function onDelete(i: InstallationInfo) {
    const ok = window.confirm(
      `Delete installation "${i.name}"?\n\nThis removes its folder and mods entirely. Your shared vanilla saves/resourcepacks/shaderpacks are NOT affected.`,
    );
    if (!ok) return;
    try {
      await deleteInstallation(i.version);
      await refresh();
    } catch (e) {
      setError(String(e));
    }
  }

  async function onSaveEdit(name: string, ramMb: number, jvmArgs: string) {
    if (!editing) return;
    try {
      await updateInstallation(editing.version, name, ramMb, jvmArgs);
      setEditing(null);
      await refresh();
    } catch (e) {
      setError(String(e));
    }
  }

  const q = query.trim().toLowerCase();
  const filtered = q
    ? installs.filter(
        (i) =>
          i.name.toLowerCase().includes(q) ||
          i.version.toLowerCase().includes(q),
      )
    : installs;

  return (
    <div className="installs-view">
      <div className="installs-header">
        <div className="search-box">
          <LuSearch className="search-icon" />
          <input
            className="search-input"
            placeholder="Search installations…"
            value={query}
            onChange={(e) => setQuery(e.currentTarget.value)}
          />
        </div>
        <div className="installs-header-actions">
          <button className="btn" onClick={() => openInstallationsFolder()}>
            Open folder
          </button>
          <button
            className="btn primary"
            onClick={onCreate}
            disabled={busy || !version}
          >
            {busy ? "Creating…" : "New installation"}
          </button>
        </div>
      </div>

      {installs.length === 0 ? (
        <p className="muted">No installations yet.</p>
      ) : filtered.length === 0 ? (
        <p className="muted">No installations match your search.</p>
      ) : (
        <ul className="install-rows">
          {filtered.map((i) => (
            <li key={i.version} className="install-row">
              <div className="install-row-info">
                <strong>{i.name}</strong>
                <span className="install-row-sub">
                  {i.version} · Fabric · {(i.ram_mb / 1024).toFixed(i.ram_mb % 1024 ? 1 : 0)} GB
                </span>
              </div>
              <div className="install-row-actions">
                <button
                  className="btn primary play-small"
                  onClick={() => onPlay(i)}
                  disabled={launching === i.version}
                >
                  {launching === i.version ? (
                    "…"
                  ) : (
                    <>
                      <LuPlay />
                      Play
                    </>
                  )}
                </button>
                <Tooltip text="Open folder">
                  <button
                    className="icon-btn"
                    onClick={() => openInstallationFolder(i.version)}
                  >
                    <FolderIcon />
                  </button>
                </Tooltip>
                <Tooltip text="Edit">
                  <button className="icon-btn" onClick={() => setEditing(i)}>
                    <PencilIcon />
                  </button>
                </Tooltip>
                <Tooltip text="Delete">
                  <button className="icon-btn danger" onClick={() => onDelete(i)}>
                    <TrashIcon />
                  </button>
                </Tooltip>
              </div>
            </li>
          ))}
        </ul>
      )}

      {error && <p className="warn">{error}</p>}

      {editing && (
        <EditInstallationDialog
          installation={editing}
          onSave={onSaveEdit}
          onCancel={() => setEditing(null)}
        />
      )}
    </div>
  );
}
