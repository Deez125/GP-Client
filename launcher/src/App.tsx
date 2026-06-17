import { useEffect, useState } from "react";
import { useAccount } from "./hooks/useAccount";
import { Sidebar } from "./components/Sidebar";
import { TopNav, type Tab } from "./components/TopNav";
import { PlayView } from "./components/PlayView";
import { InstallationsView } from "./components/InstallationsView";
import { SettingsView } from "./components/SettingsView";
import { WhatsNewView } from "./components/WhatsNewView";
import { SkinSidebar } from "./components/SkinSidebar";
import { UpdateIndicator } from "./components/UpdateIndicator";
import { LoadingOverlay } from "./components/LoadingOverlay";
import { getPlayerTextures } from "./lib/skin";
import { SERVERS } from "./config/servers";
import "./App.css";

function App() {
  const account = useAccount();
  const [tab, setTab] = useState<Tab>("play");
  // Which full-page view fills the center column.
  const [centerView, setCenterView] = useState<"tabs" | "settings" | "whatsnew">(
    "tabs",
  );
  const [serverId, setServerId] = useState(SERVERS[0].id);
  const server = SERVERS.find((s) => s.id === serverId) ?? SERVERS[0];
  // A quick-join request (server address + nonce to re-trigger). Routes to the
  // Play screen so the launch progress shows in the play dock.
  const [quickJoin, setQuickJoin] = useState<{
    address: string;
    n: number;
  } | null>(null);

  function handleQuickJoin(address: string) {
    setCenterView("tabs");
    setTab("play");
    setQuickJoin({ address, n: Date.now() });
  }

  // Startup loading gate: keep the overlay up until the account has resolved
  // and (if signed in) the player's skin has actually been fetched, so the UI
  // doesn't visibly pop the profile/skin in after everything else.
  const [skinReady, setSkinReady] = useState(false);
  useEffect(() => {
    if (account.initializing) return;
    if (!account.profile) {
      setSkinReady(true); // nothing to load when signed out
      return;
    }
    let settled = false;
    const finish = () => {
      settled = true;
      setSkinReady(true);
    };
    getPlayerTextures(account.profile.uuid).then(finish).catch(finish);
    // Safety net so a hung request can never trap the overlay.
    const t = setTimeout(() => {
      if (!settled) setSkinReady(true);
    }, 8000);
    return () => clearTimeout(t);
  }, [account.initializing, account.profile]);

  const loading = account.initializing || !skinReady;

  return (
    <div className="app">
      <Sidebar
        serverId={serverId}
        onSelectServer={setServerId}
        onOpenSettings={() => setCenterView("settings")}
        onOpenWhatsNew={() => setCenterView("whatsnew")}
        onQuickJoin={handleQuickJoin}
        activeView={centerView}
      />

      <div className="center">
        {centerView === "settings" ? (
          <SettingsView onBack={() => setCenterView("tabs")} />
        ) : centerView === "whatsnew" ? (
          <WhatsNewView onBack={() => setCenterView("tabs")} />
        ) : (
          <>
            <TopNav tab={tab} onChange={setTab} />
            <div className="view">
              {tab === "play" ? (
                <PlayView
                  signedIn={!!account.profile}
                  heroImage={server.hero}
                  quickJoin={quickJoin}
                />
              ) : (
                <InstallationsView />
              )}
            </div>
          </>
        )}
      </div>

      <SkinSidebar account={account} />

      <UpdateIndicator />

      {loading && <LoadingOverlay />}
    </div>
  );
}

export default App;
