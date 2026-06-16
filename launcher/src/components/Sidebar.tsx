import { useState } from "react";
import { LuMegaphone, LuSettings } from "react-icons/lu";
import { brand } from "../config/brand";
import logoWhite from "../assets/logo-white.svg";
import { SERVERS } from "../config/servers";
import { WhatsNewDialog } from "./WhatsNewDialog";

// Left sidebar. Logo at top; the server list in the middle (selecting one drives
// the hero image, and later which modpack syncs); What's New / Settings below.
export function Sidebar({
  serverId,
  onSelectServer,
  onOpenSettings,
}: {
  serverId: string;
  onSelectServer: (id: string) => void;
  onOpenSettings: () => void;
}) {
  const [whatsNewOpen, setWhatsNewOpen] = useState(false);
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
        <button className="sidebar-link" onClick={() => setWhatsNewOpen(true)}>
          <LuMegaphone />
          What's New
        </button>
        <button className="sidebar-link" onClick={onOpenSettings}>
          <LuSettings />
          Settings
        </button>
      </div>

      {whatsNewOpen && <WhatsNewDialog onClose={() => setWhatsNewOpen(false)} />}
    </aside>
  );
}
