import { useState } from "react";
import { useAccount } from "./hooks/useAccount";
import { Sidebar } from "./components/Sidebar";
import { TopNav, type Tab } from "./components/TopNav";
import { PlayView } from "./components/PlayView";
import { InstallationsView } from "./components/InstallationsView";
import { SettingsView } from "./components/SettingsView";
import { SkinSidebar } from "./components/SkinSidebar";
import { UpdateIndicator } from "./components/UpdateIndicator";
import { SERVERS } from "./config/servers";
import "./App.css";

function App() {
  const account = useAccount();
  const [tab, setTab] = useState<Tab>("play");
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [serverId, setServerId] = useState(SERVERS[0].id);
  const server = SERVERS.find((s) => s.id === serverId) ?? SERVERS[0];

  return (
    <div className="app">
      <Sidebar
        serverId={serverId}
        onSelectServer={setServerId}
        onOpenSettings={() => setSettingsOpen(true)}
      />

      <div className="center">
        {settingsOpen ? (
          <SettingsView onBack={() => setSettingsOpen(false)} />
        ) : (
          <>
            <TopNav tab={tab} onChange={setTab} />
            <div className="view">
              {tab === "play" ? (
                <PlayView signedIn={!!account.profile} heroImage={server.hero} />
              ) : (
                <InstallationsView />
              )}
            </div>
          </>
        )}
      </div>

      <SkinSidebar account={account} />

      <UpdateIndicator />
    </div>
  );
}

export default App;
