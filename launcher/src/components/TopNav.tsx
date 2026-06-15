export type Tab = "play" | "installations";

// Top tab bar, mirroring the vanilla launcher's Play / Installations split.
export function TopNav({
  tab,
  onChange,
}: {
  tab: Tab;
  onChange: (t: Tab) => void;
}) {
  return (
    <nav className="topnav">
      <div className="tabs">
        <button
          className={`tab${tab === "play" ? " active" : ""}`}
          onClick={() => onChange("play")}
        >
          Play
        </button>
        <button
          className={`tab${tab === "installations" ? " active" : ""}`}
          onClick={() => onChange("installations")}
        >
          Installations
        </button>
      </div>
    </nav>
  );
}
