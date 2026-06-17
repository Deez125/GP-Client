// Friends list (placeholder for now). Eventually driven by the server player
// list (Phase 1) + a presence service (Phase 2). Shows each friend's head,
// online status, what server they're on, and a Join button when joinable.
const STEVE_HEAD = "https://minotar.net/helm/MHF_Steve/64.png";

interface Friend {
  name: string;
  online: boolean;
  server: string | null;
  joinable: boolean;
}

const FRIENDS: Friend[] = [
  { name: "MomentoBruh", online: true, server: "Epstein's Island", joinable: true },
  { name: "Deez125", online: true, server: "Epstein's Skyblock", joinable: true },
  { name: "Herobrine", online: true, server: "In launcher", joinable: false },
  { name: "jomatnolen", online: true, server: "Hypixel", joinable: false },
  { name: "Notch", online: false, server: null, joinable: false },
];

export function FriendsPanel() {
  return (
    <div className="friends">
      <p className="friends-label">Friends</p>
      <div className="friends-list">
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
    </div>
  );
}
