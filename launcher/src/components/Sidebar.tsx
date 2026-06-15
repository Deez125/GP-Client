import { brand } from "../config/brand";
import logoWhite from "../assets/logo-white.svg";
import { SERVERS } from "../config/servers";

// Left sidebar. Logo at top; the server list in the middle (selecting one drives
// the hero image, and later which modpack syncs); What's New / Settings below.
export function Sidebar({
  serverId,
  onSelectServer,
}: {
  serverId: string;
  onSelectServer: (id: string) => void;
}) {
  return (
    <aside className="sidebar-left">
      <img className="sidebar-logo" src={logoWhite} alt={brand.appName} />

      <div className="server-list">
        <p className="sidebar-label">Active Servers</p>
        {SERVERS.map((s) => (
          <button
            key={s.id}
            className={`server-item${serverId === s.id ? " active" : ""}`}
            onClick={() => onSelectServer(s.id)}
          >
            <img className="server-icon" src={s.icon} alt="" />
            <span className="server-name">{s.name}</span>
          </button>
        ))}
      </div>

      <div className="sidebar-bottom">
        <button className="sidebar-link">What's New</button>
        <button className="sidebar-link">Settings</button>
      </div>
    </aside>
  );
}
