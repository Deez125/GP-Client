import { useEffect, useState } from "react";
import { createPortal } from "react-dom";
import ReactMarkdown from "react-markdown";
import { openUrl } from "@tauri-apps/plugin-opener";
import { getReleaseNotes, type ReleaseNotes } from "../lib/updates";

// "What's New" popup. Fetches the GitHub release whose tag matches this app's
// own version and renders its body as Markdown — nothing about the notes is
// hardcoded, only how Markdown is displayed.
export function WhatsNewDialog({ onClose }: { onClose: () => void }) {
  const [data, setData] = useState<ReleaseNotes | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    getReleaseNotes()
      .then(setData)
      .catch((e) => setError(String(e)));
  }, []);

  return createPortal(
    <div className="modal-backdrop" onClick={onClose}>
      <div className="modal whatsnew-modal" onClick={(e) => e.stopPropagation()}>
        <div className="mods-modal-head">
          <h3>What's New</h3>
          {data && (
            <div className="whatsnew-subhead">
              <span className="whatsnew-date">
                {data.date
                  ? new Date(data.date).toLocaleDateString(undefined, {
                      year: "numeric",
                      month: "long",
                      day: "numeric",
                    })
                  : ""}
              </span>
              <p className="whatsnew-link">
                View full notes{" "}
                <a
                  onClick={() =>
                    openUrl(
                      `https://github.com/Deez125/GP-Client/releases/tag/v${data.version}`,
                    )
                  }
                >
                  here
                </a>
              </p>
            </div>
          )}
        </div>

        {!data && !error && <p className="muted">Loading release notes…</p>}
        {error && <p className="warn">{error}</p>}

        {data && (
          <div className="whatsnew-scroll">
            {data.notes ? (
              <div className="markdown">
                <ReactMarkdown>{data.notes}</ReactMarkdown>
              </div>
            ) : (
              <p className="muted">
                No release notes published for v{data.version} yet.
              </p>
            )}
          </div>
        )}

        <div className="modal-actions">
          <button className="btn primary" onClick={onClose}>
            Close
          </button>
        </div>
      </div>
    </div>,
    document.body,
  );
}
