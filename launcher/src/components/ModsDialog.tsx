import { useEffect, useMemo, useState } from "react";
import {
  getVersionMods,
  setOptionalMods,
  type OptionalCategoryView,
  type VersionMods,
} from "../lib/mods";
import { LoadingSpinner } from "./LoadingSpinner";

// The Mods picker. Required mods collapse to a single locked summary; optional
// mods are grouped into collapsible categories, each with a select-all box.
export function ModsDialog({
  version,
  onClose,
  saveLabel = "Save",
  onSaved,
}: {
  version: string;
  onClose: () => void;
  /** Label for the confirm button (e.g. "Continue" during install). */
  saveLabel?: string;
  /** Called after the selection is saved (e.g. to start the install). */
  onSaved?: () => void;
}) {
  const [data, setData] = useState<VersionMods | null>(null);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    getVersionMods(version)
      .then((d) => {
        setData(d);
        const init = new Set<string>();
        for (const cat of d.optional) {
          for (const m of cat.mods) if (m.selected) init.add(m.name);
        }
        setSelected(init);
        setExpanded(new Set(d.optional.map((c) => c.category)));
      })
      .catch((e) => setError(String(e)));
  }, [version]);

  // Names of every installable optional mod (available + not inactive).
  const allAvailable = useMemo(() => {
    const names: string[] = [];
    data?.optional.forEach((c) =>
      c.mods.forEach((m) => {
        if (m.available && !m.inactive) names.push(m.name);
      }),
    );
    return names;
  }, [data]);

  const allSelected =
    allAvailable.length > 0 && allAvailable.every((n) => selected.has(n));

  function catAvailable(cat: OptionalCategoryView) {
    return cat.mods.filter((m) => m.available && !m.inactive).map((m) => m.name);
  }
  function catState(cat: OptionalCategoryView): "all" | "some" | "none" {
    const a = catAvailable(cat);
    const n = a.filter((name) => selected.has(name)).length;
    if (a.length > 0 && n === a.length) return "all";
    return n > 0 ? "some" : "none";
  }

  function toggle(name: string) {
    setSelected((prev) => {
      const next = new Set(prev);
      next.has(name) ? next.delete(name) : next.add(name);
      return next;
    });
  }
  function toggleCategory(cat: OptionalCategoryView) {
    const a = catAvailable(cat);
    setSelected((prev) => {
      const next = new Set(prev);
      const all = a.every((n) => next.has(n));
      a.forEach((n) => (all ? next.delete(n) : next.add(n)));
      return next;
    });
  }
  function toggleExpanded(name: string) {
    setExpanded((prev) => {
      const next = new Set(prev);
      next.has(name) ? next.delete(name) : next.add(name);
      return next;
    });
  }
  function toggleAll() {
    setSelected(allSelected ? new Set() : new Set(allAvailable));
  }

  async function onSave() {
    setSaving(true);
    setError(null);
    try {
      await setOptionalMods(version, [...selected]);
      onClose();
      onSaved?.();
    } catch (e) {
      setError(String(e));
      setSaving(false);
    }
  }

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div className="modal mods-modal" onClick={(e) => e.stopPropagation()}>
        <div className="mods-modal-head">
          <h3>{version} Mods</h3>
        </div>

        {!data && !error && <LoadingSpinner />}
        {error && <p className="warn">{error}</p>}

        {data && (
          <div className="mods-scroll">
            {/* Required: locked, always-on summary. */}
            <div className="required-row">
              <div>
                <strong>Required Mods</strong>{" "}
                <span className="muted small">
                  {data.required.length} mods Installed
                </span>
              </div>
              <span className="mod-check on locked" title="Always installed">
                ✓
              </span>
            </div>

            {data.optional.map((cat) => {
              const state = catState(cat);
              const isOpen = expanded.has(cat.category);
              return (
                <section className="mod-group" key={cat.category}>
                  <div
                    className="mod-group-head"
                    onClick={() => toggleExpanded(cat.category)}
                  >
                    <span className={`chevron${isOpen ? " open" : ""}`}>▸</span>
                    <h4>{cat.category}</h4>
                    <span className="muted small grow">
                      {cat.mods.filter((m) => selected.has(m.name)).length}/
                      {cat.mods.length}
                    </span>
                    <span
                      className={`mod-check section ${state}`}
                      onClick={(e) => {
                        e.stopPropagation();
                        toggleCategory(cat);
                      }}
                      title="Select all in category"
                    >
                      {state === "all" ? "✓" : state === "some" ? "–" : ""}
                    </span>
                  </div>

                  {isOpen && (
                    <div className="mod-cards">
                      {cat.mods.map((m) => {
                        const disabled = m.inactive || !m.available;
                        const on = selected.has(m.name);
                        return (
                          <button
                            key={m.name}
                            className={`mod-card${on ? " on" : ""}${disabled ? " disabled" : ""}`}
                            onClick={() => !disabled && toggle(m.name)}
                            disabled={disabled}
                            title={
                              m.inactive
                                ? "Unavailable (too big to host yet)"
                                : !m.available
                                  ? "Not available on the server yet"
                                  : ""
                            }
                          >
                            {m.image_url ? (
                              <img
                                className="mod-img"
                                src={m.image_url}
                                alt=""
                                onError={(e) =>
                                  (e.currentTarget.style.visibility = "hidden")
                                }
                              />
                            ) : (
                              <div className="mod-img placeholder" />
                            )}
                            <div className="mod-card-body">
                              <strong>{m.name}</strong>
                              {m.description && <p>{m.description}</p>}
                              {m.inactive && (
                                <span className="mod-flag">Unavailable</span>
                              )}
                            </div>
                            <span className={`mod-check${on ? " on" : ""}`}>
                              {on ? "✓" : ""}
                            </span>
                          </button>
                        );
                      })}
                    </div>
                  )}
                </section>
              );
            })}
          </div>
        )}

        <div className="modal-actions">
          <button
            className="btn select-all"
            onClick={toggleAll}
            disabled={!data || allAvailable.length === 0}
          >
            {allSelected ? "Deselect all" : "Select all"}
          </button>
          <button className="btn" onClick={onClose}>
            Cancel
          </button>
          <button className="btn primary" onClick={onSave} disabled={saving || !data}>
            {saving ? "Saving…" : saveLabel}
          </button>
        </div>
      </div>
    </div>
  );
}
