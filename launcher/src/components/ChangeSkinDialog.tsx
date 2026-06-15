import { useEffect, useState } from "react";
import { createPortal } from "react-dom";
import { SkinRender } from "./SkinRender";
import { listSkins, renameSkin, type SkinLibEntry } from "../lib/skin";

// Skin manager: "Current" (animated, shows the selected skin) on the left;
// "Library" grid (paused tiles) on the right — seeded with Steve, Alex, and the
// player's current skin, newest first. Names are editable; library is stored in
// .minecraft/GP Client/skins/.
export function ChangeSkinDialog({
  uuid,
  onClose,
}: {
  uuid: string;
  username: string;
  onClose: () => void;
}) {
  const [entries, setEntries] = useState<SkinLibEntry[]>([]);
  const [selected, setSelected] = useState("");
  const [editingId, setEditingId] = useState<string | null>(null);
  const [draft, setDraft] = useState("");

  useEffect(() => {
    listSkins(uuid)
      .then((es) => {
        setEntries(es);
        setSelected((prev) => prev || es[0]?.id || "");
      })
      .catch(() => {});
  }, [uuid]);

  const current = entries.find((e) => e.id === selected) ?? entries[0];

  function startEdit(e: SkinLibEntry) {
    setEditingId(e.id);
    setDraft(e.name);
  }

  async function commitEdit() {
    const id = editingId;
    setEditingId(null);
    if (!id) return;
    const name = draft.trim();
    if (!name) return;
    setEntries((prev) => prev.map((e) => (e.id === id ? { ...e, name } : e)));
    await renameSkin(id, name).catch(() => {});
  }

  return createPortal(
    <div className="modal-backdrop" onClick={onClose}>
      <div className="modal skin-modal" onClick={(e) => e.stopPropagation()}>
        <div className="skin-panes">
          <div className="skin-current">
            <p className="skin-col-title">Current</p>
            {current && (
              <SkinRender
                key={current.id}
                src={current.data_url}
                model={current.model}
                width={230}
                height={380}
                animated
              />
            )}
            <span className="skin-current-name">{current?.name}</span>
          </div>

          <div className="skin-vline" />

          <div className="skin-library">
            <p className="skin-col-title">Library</p>
            <div className="skin-grid">
              <button className="skin-tile add" title="Coming soon">
                <span className="skin-add-circle">+</span>
                <span>New skin</span>
              </button>

              {entries.map((e) => (
                <div
                  key={e.id}
                  className={`skin-tile${selected === e.id ? " selected" : ""}`}
                >
                  <div
                    className="skin-tile-render"
                    onClick={() => setSelected(e.id)}
                  >
                    <SkinRender
                      src={e.data_url}
                      model={e.model}
                      width={110}
                      height={170}
                      animated={false}
                    />
                  </div>
                  {editingId === e.id ? (
                    <input
                      className="skin-name-input"
                      value={draft}
                      autoFocus
                      onChange={(ev) => setDraft(ev.currentTarget.value)}
                      onBlur={commitEdit}
                      onKeyDown={(ev) => {
                        if (ev.key === "Enter") ev.currentTarget.blur();
                        if (ev.key === "Escape") setEditingId(null);
                      }}
                    />
                  ) : (
                    <span
                      className="skin-name"
                      title="Click to rename"
                      onClick={() => startEdit(e)}
                    >
                      {e.name}
                    </span>
                  )}
                </div>
              ))}
            </div>
          </div>
        </div>

        <div className="modal-actions">
          <button className="btn" onClick={onClose}>
            Close
          </button>
        </div>
      </div>
    </div>,
    document.body,
  );
}
