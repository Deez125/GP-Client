import { useState } from "react";
import type { InstallationInfo } from "../lib/installations";

const MIN_RAM = 1024;
const MAX_RAM = 16384;
const STEP = 512;

// Edit dialog: launcher-only name, a RAM slider that drives -Xmx, and extra
// JVM arguments. The RAM slider sits above the arguments field, per request.
export function EditInstallationDialog({
  installation,
  onSave,
  onCancel,
}: {
  installation: InstallationInfo;
  onSave: (name: string, ramMb: number, jvmArgs: string) => void;
  onCancel: () => void;
}) {
  const [name, setName] = useState(installation.name);
  const [ram, setRam] = useState(installation.ram_mb);
  const [jvmArgs, setJvmArgs] = useState(installation.jvm_args);

  const gb = (ram / 1024).toFixed(ram % 1024 ? 1 : 0);

  return (
    <div className="modal-backdrop" onClick={onCancel}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <h3>Edit installation</h3>
        <p className="muted small">
          Version {installation.version} · launcher-only settings (the folder
          isn't renamed)
        </p>

        <label className="field">
          <span>Name</span>
          <input
            value={name}
            onChange={(e) => setName(e.currentTarget.value)}
            placeholder={installation.version}
          />
        </label>

        <label className="field">
          <span>
            Memory: <strong>{gb} GB</strong>{" "}
            <span className="muted small">→ -Xmx{ram}m</span>
          </span>
          <input
            type="range"
            min={MIN_RAM}
            max={MAX_RAM}
            step={STEP}
            value={ram}
            onChange={(e) => setRam(Number(e.currentTarget.value))}
          />
          <div className="range-ends muted small">
            <span>{MIN_RAM / 1024} GB</span>
            <span>{MAX_RAM / 1024} GB</span>
          </div>
        </label>

        <label className="field">
          <span>Extra JVM arguments</span>
          <textarea
            value={jvmArgs}
            onChange={(e) => setJvmArgs(e.currentTarget.value)}
            rows={3}
            placeholder="e.g. -XX:+UseG1GC -Dsomething=true"
          />
        </label>

        <div className="modal-actions">
          <button className="btn" onClick={onCancel}>
            Cancel
          </button>
          <button
            className="btn primary"
            onClick={() => onSave(name, ram, jvmArgs)}
          >
            Save
          </button>
        </div>
      </div>
    </div>
  );
}
