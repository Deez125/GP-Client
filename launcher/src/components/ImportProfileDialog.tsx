import { useEffect, useState } from "react";
import { createPortal } from "react-dom";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { Dropdown } from "./Dropdown";
import {
  importProfileSettings,
  listSourceProfiles,
  type ImportItems,
  type ImportReport,
  type SourceProfile,
} from "../lib/profileImport";
import { listInstallations, type InstallationInfo } from "../lib/installations";

const ITEM_FIELDS: { key: keyof ImportItems; label: string; hint: string }[] = [
  { key: "options", label: "Game options & keybinds", hint: "Volume, keybinds, video, GUI scale" },
  { key: "shaders", label: "Shader selection", hint: "Your active Iris shader" },
  { key: "servers", label: "Server list", hint: "Saved multiplayer servers" },
  { key: "schematics", label: "Litematica schematics", hint: "Your saved build schematics" },
  { key: "xaero", label: "Xaero's world map", hint: "Explored world map data" },
];

// Folder name (last path segment) for display, handling both \ and /.
function baseName(p: string) {
  return p.split(/[\\/]/).filter(Boolean).pop() ?? p;
}

// Popup to import settings from an existing Minecraft profile into a GP Client
// installation. Opened from the profile sidebar.
export function ImportProfileDialog({
  defaultVersion,
  onClose,
}: {
  defaultVersion: string;
  onClose: () => void;
}) {
  const [profiles, setProfiles] = useState<SourceProfile[]>([]);
  const [installs, setInstalls] = useState<InstallationInfo[]>([]);
  const [source, setSource] = useState("");
  const [target, setTarget] = useState("");
  const [items, setItems] = useState<ImportItems>({
    options: true,
    shaders: true,
    servers: true,
    schematics: true,
    xaero: true,
  });
  const [busy, setBusy] = useState(false);
  const [report, setReport] = useState<ImportReport | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    listSourceProfiles()
      .then((ps) => {
        setProfiles(ps);
        if (ps.length > 0) setSource(ps[0].path);
      })
      .catch((e) => setError(String(e)));
    listInstallations()
      .then((list) => {
        setInstalls(list);
        const def = list.find((i) => i.version === defaultVersion);
        setTarget(def?.version ?? list[0]?.version ?? defaultVersion);
      })
      .catch((e) => setError(String(e)));
  }, [defaultVersion]);

  async function onBrowse() {
    const picked = await openDialog({ directory: true, multiple: false });
    if (typeof picked === "string") {
      // Add the picked folder as a source option and select it.
      setProfiles((prev) =>
        prev.some((p) => p.path === picked)
          ? prev
          : [...prev, { name: baseName(picked), path: picked }],
      );
      setSource(picked);
    }
  }

  function toggle(key: keyof ImportItems) {
    setItems((prev) => ({ ...prev, [key]: !prev[key] }));
  }

  const anyItem = Object.values(items).some(Boolean);

  async function onImport() {
    setBusy(true);
    setError(null);
    setReport(null);
    try {
      const r = await importProfileSettings(target, source, items);
      setReport(r);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  const sourceOptions = profiles.map((p) => ({ value: p.path, label: p.name }));
  const targetOptions = installs.map((i) => ({
    value: i.version,
    label: i.version === defaultVersion ? `${i.version} (current)` : i.version,
  }));

  return createPortal(
    <div className="modal-backdrop" onClick={onClose}>
      <div className="modal import-modal" onClick={(e) => e.stopPropagation()}>
        <div className="mods-modal-head">
          <h3>Import settings</h3>
          <p className="muted small">
            Copy your game settings from another Minecraft profile into a GP Client
            installation. Worlds and resource packs are already shared, so they're
            left out.
          </p>
        </div>

        {error && <p className="warn">{error}</p>}

        {report ? (
          <div className="import-result">
            {report.imported.length > 0 && (
              <>
                <strong>Imported</strong>
                <ul>
                  {report.imported.map((x) => (
                    <li key={x}>✓ {x}</li>
                  ))}
                </ul>
              </>
            )}
            {report.skipped.length > 0 && (
              <>
                <strong className="muted">Skipped</strong>
                <ul className="muted">
                  {report.skipped.map((x) => (
                    <li key={x}>– {x}</li>
                  ))}
                </ul>
              </>
            )}
            {report.imported.length === 0 && (
              <p className="muted">Nothing was imported.</p>
            )}
          </div>
        ) : (
          <div className="import-body">
            <label className="import-field">
              <span>Import from</span>
              <div className="import-source-row">
                <Dropdown
                  value={source}
                  onChange={setSource}
                  disabled={sourceOptions.length === 0}
                  placeholder="No profiles found"
                  options={sourceOptions}
                />
                <button className="btn" onClick={onBrowse}>
                  Browse…
                </button>
              </div>
            </label>

            <label className="import-field">
              <span>Import into</span>
              <Dropdown
                value={target}
                onChange={setTarget}
                disabled={targetOptions.length === 0}
                placeholder="No installations"
                options={targetOptions}
              />
            </label>

            <div className="import-field">
              <span>What to import</span>
              <div className="import-checks">
                {ITEM_FIELDS.map((f) => (
                  <label key={f.key} className="import-check">
                    <input
                      type="checkbox"
                      checked={items[f.key]}
                      onChange={() => toggle(f.key)}
                    />
                    <span className="import-check-text">
                      <strong>{f.label}</strong>
                      <span className="muted small">{f.hint}</span>
                    </span>
                  </label>
                ))}
              </div>
            </div>
          </div>
        )}

        <div className="modal-actions">
          {report ? (
            <button className="btn primary" onClick={onClose}>
              Done
            </button>
          ) : (
            <>
              <button className="btn" onClick={onClose}>
                Cancel
              </button>
              <button
                className="btn primary"
                onClick={onImport}
                disabled={busy || !source || !target || !anyItem}
              >
                {busy ? "Importing…" : "Import"}
              </button>
            </>
          )}
        </div>
      </div>
    </div>,
    document.body,
  );
}
