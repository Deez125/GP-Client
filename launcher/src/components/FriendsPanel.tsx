// Friends list. The real data needs the Minecraft Services friends-graph +
// presence API (Java 26.2), which our servers don't expose yet — so for now
// the panel shows the example layout behind a "Coming soon" overlay. Flip
// COMING_SOON to false (0.1.6b) once the friends API is wired up.
const STEVE_HEAD = "https://minotar.net/helm/MHF_Steve/64.png";

// Gate: while true, the panel is display-only (blurred example + overlay).
const COMING_SOON = true;

interface Friend {
  name: string;
  online: boolean;
  server: string | null;
  joinable: boolean;
}

const FRIENDS: Friend[] = [
  { name: "ryankdy", online: true, server: "Epstein's Island", joinable: true },
  { name: "Sentrix", online: true, server: "Epstein's Skyblock", joinable: true },
  { name: "kayla_b", online: true, server: "In launcher", joinable: false },
  { name: "Volt_99", online: true, server: "Hypixel", joinable: false },
  { name: "mattheww", online: false, server: null, joinable: false },
];

export function FriendsPanel() {
  return (
    <div className="friends">
      <p className="friends-label">Friends</p>
      <div className={`friends-stage${COMING_SOON ? " coming-soon" : ""}`}>
        <div className="friends-list" aria-hidden={COMING_SOON}>
          {FRIENDS.map((f) => (
            <div className="friend" key={f.name}>
              <div className="friend-avatar-wrap">
                <img className="friend-avatar" src={STEVE_HEAD} alt="" />
                <span className={`friend-dot${f.online ? " online" : ""}`} />
              </div>
              <div className="friend-meta">
                <strong title={f.name}>{f.name}</strong>
                <span className="friend-sub">
                  {f.online ? (f.server ?? "In launcher") : "Offline"}
                </span>
              </div>
              {f.joinable && <button className="friend-join">Join</button>}
            </div>
          ))}
        </div>

        {COMING_SOON && (
          <div className="friends-overlay">
            <span className="friends-overlay-title">Coming soon</span>
            <span className="friends-overlay-sub">
              Friends &amp; presence arrive once the servers update to 26.2.
            </span>
          </div>
        )}
      </div>
    </div>
  );
}
