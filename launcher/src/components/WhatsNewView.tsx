import { useEffect, useState } from "react";
import { LuArrowLeft } from "react-icons/lu";
import ReactMarkdown from "react-markdown";
import { openUrl } from "@tauri-apps/plugin-opener";
import { getReleaseNotes, type ReleaseNotes } from "../lib/updates";
import { LoadingSpinner } from "./LoadingSpinner";

// Full-page "What's New" screen (same shape as the Settings page). Fetches the
// GitHub release whose tag matches this app's version and renders its body as
// Markdown — nothing about the notes is hardcoded, only how Markdown displays.
export function WhatsNewView({ onBack }: { onBack: () => void }) {
  const [data, setData] = useState<ReleaseNotes | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    getReleaseNotes()
      .then(setData)
      .catch((e) => setError(String(e)));
  }, []);

  return (
    <div className="settings-view">
      <div className="settings-head">
        <button className="settings-back" onClick={onBack} title="Back">
          <LuArrowLeft />
        </button>
        <h2>What's New</h2>
      </div>

      <div className="settings-scroll">
        {!data && !error && <LoadingSpinner />}
        {error && <p className="warn">{error}</p>}
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
        {data &&
          (data.notes ? (
            <div className="markdown">
              <ReactMarkdown>{data.notes}</ReactMarkdown>
            </div>
          ) : (
            <p className="muted">
              No release notes published for v{data.version} yet.
            </p>
          ))}
      </div>
    </div>
  );
}
