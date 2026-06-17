import { LuMegaphone, LuSettings, LuPlay } from "react-icons/lu";
import { FaSignal } from "react-icons/fa";
import { HiStatusOffline } from "react-icons/hi";
import { Tooltip } from "./Tooltip";
import { brand } from "../config/brand";
import logoWhite from "../assets/logo-white.svg";
import { SERVERS } from "../config/servers";

// Left sidebar. Logo at top; the server list in the middle (selecting one drives
// the hero image, and later which modpack syncs); What's New / Settings below.
export function Sidebar({
  serverId,
  onSelectServer,
  onOpenSettings,
  onOpenWhatsNew,
  onQuickJoin,
  activeView,
}: {
  serverId: string;
  onSelectServer: (id: string) => void;
  onOpenSettings: () => void;
  onOpenWhatsNew: () => void;
  onQuickJoin: (address: string) => void;
  activeView: "tabs" | "settings" | "whatsnew";
}) {
  return (
    <aside className="sidebar-left">
      <img className="sidebar-logo" src={logoWhite} alt={brand.appName} />

      <div className="server-list">
        <p className="sidebar-label">Supported servers</p>
        {SERVERS.map((s, i) => {
          // Placeholder online state until live status lands: first is up.
          const online = i === 0;
          return (
            <div
              key={s.id}
              className={`server-item${serverId === s.id ? " active" : ""}`}
              role="button"
              onClick={() => onSelectServer(s.id)}
            >
              <img className="server-icon" src={s.icon} alt="" />
              <div className="server-meta">
                <span className="server-name">{s.name}</span>
                <span className="server-sub">
                  {online ? (
                    <>
                      <FaSignal className="signal-icon" />
                      12 online
                    </>
                  ) : (
                    <>
                      <HiStatusOffline className="offline-icon" />
                      Offline
                    </>
                  )}
                </span>
              </div>
              <Tooltip text="Quick join" disabled={!online}>
                <button
                  className="server-join"
                  disabled={!online}
                  onClick={(e) => {
                    e.stopPropagation();
                    onQuickJoin(s.address);
                  }}
                >
                  <LuPlay />
                </button>
              </Tooltip>
            </div>
          );
        })}
      </div>

      <div className="sidebar-bottom">
        <button
          className={`sidebar-link${activeView === "whatsnew" ? " active" : ""}`}
          onClick={onOpenWhatsNew}
        >
          <LuMegaphone />
          What's New
        </button>
        <button
          className={`sidebar-link${activeView === "settings" ? " active" : ""}`}
          onClick={onOpenSettings}
        >
          <LuSettings />
          Settings
        </button>
      </div>
    </aside>
  );
}
