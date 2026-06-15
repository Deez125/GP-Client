import { useState } from "react";
import { useAccount } from "./hooks/useAccount";
import { Sidebar } from "./components/Sidebar";
import { TopNav, type Tab } from "./components/TopNav";
import { PlayView } from "./components/PlayView";
import { InstallationsView } from "./components/InstallationsView";
import { SkinSidebar } from "./components/SkinSidebar";
import { SERVERS } from "./config/servers";
import "./App.css";

function App() {
  const account = useAccount();
  const [tab, setTab] = useState<Tab>("play");
  const [serverId, setServerId] = useState(SERVERS[0].id);
  const server = SERVERS.find((s) => s.id === serverId) ?? SERVERS[0];

  return (
    <div className="app">
      <Sidebar serverId={serverId} onSelectServer={setServerId} />

      <div className="center">
        <TopNav tab={tab} onChange={setTab} />
        <div className="view">
          {tab === "play" ? (
            <PlayView signedIn={!!account.profile} heroImage={server.hero} />
          ) : (
            <InstallationsView />
          )}
        </div>
      </div>

      <SkinSidebar account={account} />
    </div>
  );
}

export default App;
