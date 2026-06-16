import { useEffect, useState } from "react";
import { createPortal } from "react-dom";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { LuSquarePen, LuTrash2 } from "react-icons/lu";
import { SkinRender } from "./SkinRender";
import {
  applySkin,
  deleteSkin,
  getCapes,
  importSkin,
  listSkins,
  replaceSkinFile,
  setCape,
  updateSkin,
  type CapeEntry,
  type SkinLibEntry,
} from "../lib/skin";

type Status =
  | { kind: "idle" }
  | { kind: "working" }
  | { kind: "error"; msg: string };

function baseName(p: string) {
  const file = p.split(/[\\/]/).filter(Boolean).pop() ?? p;
  return file.replace(/\.png$/i, "");
}

// Turn a raw backend error into short, friendly text.
function friendlyError(raw: string): string {
  const e = raw.toLowerCase();
  if (e.includes("429") || e.includes("too many requests"))
    return "Too many requests right now. Try again in a minute.";
  if (e.includes("expected 64") || e.includes("not a minecraft skin"))
    return "That file isn't a valid Minecraft skin (must be a 64×64 PNG).";
  if (e.includes("not a valid image"))
    return "That file isn't a valid image.";
  if (e.includes("signed in") || e.includes("sign-in") || e.includes("auth"))
    return "Couldn't reach your Microsoft account. Try again later.";
  if (e.includes("network") || e.includes("sending request") || e.includes("timed out"))
    return "Network problem. Check your connection and try again.";
  return "Something went wrong. Try again.";
}

// Skin manager. Library view: a big preview of the selected skin + a grid of
// library skins, each with Select / Edit / Delete on hover. Edit view: rename,
// pick the arm model, replace the PNG, and choose a cape from your account.
export function ChangeSkinDialog({
  uuid,
  onClose,
}: {
  uuid: string;
  username: string;
  onClose: () => void;
}) {
  const [entries, setEntries] = useState<SkinLibEntry[]>([]);
  const [capes, setCapes] = useState<CapeEntry[]>([]);
  const [selected, setSelected] = useState("");
  const [editing, setEditing] = useState<SkinLibEntry | null>(null);
  const [confirmId, setConfirmId] = useState<string | null>(null);
  const [status, setStatus] = useState<Status>({ kind: "idle" });

  // Edit-form state.
  const [fName, setFName] = useState("");
  const [fModel, setFModel] = useState<"default" | "slim">("default");
  const [fCape, setFCape] = useState<string>("none"); // cape id or "none"
  const [fFile, setFFile] = useState<string | null>(null); // new PNG to apply on save

  function refresh() {
    return listSkins(uuid).then((es) => {
      setEntries(es);
      return es;
    });
  }

  useEffect(() => {
    listSkins(uuid)
      .then((es) => {
        setEntries(es);
        setSelected((prev) => prev || es[0]?.id || "");
      })
      .catch(() => {});
    getCapes()
      .then(setCapes)
      .catch(() => {}); // best-effort; capes need a live account
  }, [uuid]);

  const current = entries.find((e) => e.id === selected) ?? entries[0];
  const capeTexture = (id: string | null) =>
    capes.find((c) => c.id === id)?.texture ?? null;
  const activeCapeTexture = capes.find((c) => c.active)?.texture ?? null;

  // Which cape texture to drape on an entry's preview.
  function entryCape(e: SkinLibEntry | undefined): string | null {
    if (!e) return null;
    if (e.cape === "none") return null;
    if (e.cape) return capeTexture(e.cape);
    return activeCapeTexture;
  }

  // ----- library actions -----
  async function onSelect(e: SkinLibEntry) {
    setStatus({ kind: "working" });
    try {
      await applySkin(uuid, e.id, e.model === "slim" ? "slim" : "classic");
      if (e.cape != null) await setCape(e.cape === "none" ? null : e.cape);
      setSelected(e.id);
      setStatus({ kind: "idle" });
    } catch (err) {
      setStatus({ kind: "error", msg: String(err) });
    }
  }

  async function onImport() {
    try {
      const picked = await openDialog({
        multiple: false,
        filters: [{ name: "Minecraft skin (PNG)", extensions: ["png"] }],
      });
      if (typeof picked !== "string") return;
      const id = await importSkin(uuid, picked, baseName(picked), "default");
      const es = await refresh();
      const created = es.find((x) => x.id === id);
      if (created) openEdit(created); // jump straight into editing the new skin
    } catch (err) {
      setStatus({ kind: "error", msg: String(err) });
    }
  }

  async function doDelete(id: string) {
    setConfirmId(null);
    try {
      await deleteSkin(uuid, id);
      const es = await refresh();
      if (selected === id) setSelected(es[0]?.id ?? "");
    } catch (err) {
      setStatus({ kind: "error", msg: String(err) });
    }
  }

  // ----- edit -----
  function openEdit(e: SkinLibEntry) {
    setEditing(e);
    setFName(e.name);
    setFModel(e.model === "slim" ? "slim" : "default");
    setFCape(e.cape ?? capes.find((c) => c.active)?.id ?? "none");
    setFFile(null);
    setStatus({ kind: "idle" });
  }

  async function onBrowseFile() {
    const picked = await openDialog({
      multiple: false,
      filters: [{ name: "Minecraft skin (PNG)", extensions: ["png"] }],
    });
    if (typeof picked === "string") setFFile(picked);
  }

  async function saveEdits() {
    if (!editing) return;
    await updateSkin(uuid, editing.id, fName, fModel, fCape);
    if (fFile) await replaceSkinFile(uuid, editing.id, fFile);
    await refresh();
  }

  async function onSave() {
    setStatus({ kind: "working" });
    try {
      await saveEdits();
      setEditing(null);
      setStatus({ kind: "idle" });
    } catch (err) {
      setStatus({ kind: "error", msg: String(err) });
    }
  }

  async function onSaveAndSelect() {
    if (!editing) return;
    setStatus({ kind: "working" });
    try {
      await saveEdits();
      await applySkin(uuid, editing.id, fModel === "slim" ? "slim" : "classic");
      await setCape(fCape === "none" ? null : fCape);
      setSelected(editing.id);
      setEditing(null);
      setStatus({ kind: "idle" });
    } catch (err) {
      setStatus({ kind: "error", msg: String(err) });
    }
  }

  const statusLine =
    status.kind === "working" ? (
      <p className="skin-status">Working…</p>
    ) : status.kind === "error" ? (
      <p className="skin-status err">{friendlyError(status.msg)}</p>
    ) : null;

  return createPortal(
    <div className="modal-backdrop" onClick={onClose}>
      <div className="modal skin-modal" onClick={(e) => e.stopPropagation()}>
        {editing ? (
          // ---------------- EDIT SCREEN ----------------
          <div className="skin-panes">
            <div className="skin-current">
              <p className="skin-col-title">Edit skin</p>
              <SkinRender
                key={`${editing.id}-${fModel}-${fCape}`}
                src={editing.data_url}
                model={fModel}
                cape={fCape === "none" ? null : capeTexture(fCape)}
                width={230}
                height={380}
                animated
              />
            </div>

            <div className="skin-vline" />

            <div className="skin-edit-form">
              <label className="edit-field">
                <span>NAME</span>
                <input
                  className="edit-name-input"
                  value={fName}
                  onChange={(e) => setFName(e.currentTarget.value)}
                />
              </label>

              <div className="edit-field">
                <span>PLAYER MODEL</span>
                <div className="model-radios">
                  <label>
                    <input
                      type="radio"
                      checked={fModel === "default"}
                      onChange={() => setFModel("default")}
                    />
                    Wide
                  </label>
                  <label>
                    <input
                      type="radio"
                      checked={fModel === "slim"}
                      onChange={() => setFModel("slim")}
                    />
                    Slim
                  </label>
                </div>
              </div>

              <div className="edit-field">
                <span>SKIN FILE</span>
                <div className="edit-file-row">
                  <button className="btn" onClick={onBrowseFile}>
                    Browse
                  </button>
                  {fFile && (
                    <span className="muted small">
                      {baseName(fFile)}.png — applied on save
                    </span>
                  )}
                </div>
              </div>

              <div className="edit-field">
                <span>CAPE</span>
                <div className="cape-grid">
                  <button
                    className={`cape-tile${fCape === "none" ? " on" : ""}`}
                    onClick={() => setFCape("none")}
                  >
                    <div className="cape-none">None</div>
                    <span>No cape</span>
                  </button>
                  {capes.map((c) => (
                    <button
                      key={c.id}
                      className={`cape-tile${fCape === c.id ? " on" : ""}`}
                      onClick={() => setFCape(c.id)}
                    >
                      <img src={c.data_url} alt={c.name} />
                      <span>{c.name}</span>
                    </button>
                  ))}
                </div>
              </div>
            </div>
          </div>
        ) : (
          // ---------------- LIBRARY SCREEN ----------------
          <div className="skin-panes">
            <div className="skin-current">
              {current && (
                <SkinRender
                  key={`${current.id}-${current.cape ?? "live"}`}
                  src={current.data_url}
                  model={current.model === "slim" ? "slim" : "default"}
                  cape={entryCape(current)}
                  width={230}
                  height={380}
                  animated
                />
              )}
              <span className="skin-current-name">{current?.name}</span>
            </div>

            <div className="skin-vline" />

            <div className="skin-library">
              <div className="skin-grid">
                <button className="skin-tile add" onClick={onImport} title="Import a PNG skin">
                  <span className="skin-add-circle">+</span>
                  <span>New skin</span>
                </button>

                {entries.map((e) => (
                  <div
                    key={e.id}
                    className={`skin-tile${selected === e.id ? " selected" : ""}`}
                  >
                    <div className="skin-tile-render">
                      <SkinRender
                        src={e.data_url}
                        model={e.model}
                        width={110}
                        height={170}
                        animated={false}
                      />
                    </div>
                    <span className="skin-name">{e.name}</span>

                    <div className="skin-tile-actions">
                      <button
                        className="tile-action select"
                        onClick={() => onSelect(e)}
                      >
                        Select
                      </button>
                      <div className="tile-action-row">
                        <button
                          className="tile-action icon"
                          title="Edit"
                          onClick={() => openEdit(e)}
                        >
                          <LuSquarePen />
                        </button>
                        <button
                          className="tile-action icon"
                          title="Delete"
                          onClick={() => setConfirmId(e.id)}
                        >
                          <LuTrash2 />
                        </button>
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </div>
        )}

        <div className="modal-actions">
          {statusLine}
          {editing ? (
            <>
              <button className="btn" onClick={() => setEditing(null)}>
                Cancel
              </button>
              <button className="btn" onClick={onSave} disabled={status.kind === "working"}>
                Save
              </button>
              <button
                className="btn primary"
                onClick={onSaveAndSelect}
                disabled={status.kind === "working"}
              >
                Save and select
              </button>
            </>
          ) : (
            <button className="btn" onClick={onClose}>
              Close
            </button>
          )}
        </div>

        {confirmId && (
          <div className="confirm-backdrop" onClick={() => setConfirmId(null)}>
            <div className="confirm-box" onClick={(e) => e.stopPropagation()}>
              <p className="confirm-title">Delete this skin?</p>
              <p className="muted small">
                "{entries.find((x) => x.id === confirmId)?.name}" will be removed
                from your library. This doesn't affect your account.
              </p>
              <div className="confirm-actions">
                <button className="btn" onClick={() => setConfirmId(null)}>
                  Cancel
                </button>
                <button className="btn danger" onClick={() => doDelete(confirmId)}>
                  Delete
                </button>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>,
    document.body,
  );
}
