import { useState } from "react";
import { LuArrowLeft } from "react-icons/lu";
import { Dropdown } from "./Dropdown";

// A simple on/off switch used by the settings rows.
function Toggle({
  on,
  onChange,
}: {
  on: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <button
      type="button"
      className={`switch${on ? " on" : ""}`}
      onClick={() => onChange(!on)}
      role="switch"
      aria-checked={on}
    >
      <span className="switch-knob" />
    </button>
  );
}

// One settings line: label + description on the left, control on the right.
function Row({
  title,
  desc,
  children,
}: {
  title: string;
  desc?: string;
  children: React.ReactNode;
}) {
  return (
    <div className="setting-row">
      <div className="setting-text">
        <span className="setting-title">{title}</span>
        {desc && <span className="setting-desc">{desc}</span>}
      </div>
      <div className="setting-control">{children}</div>
    </div>
  );
}

// Full-page Settings view. Replaces the Play/Installations tabs and the play
// dock. Placeholder controls for now — wired up later.
export function SettingsView({ onBack }: { onBack: () => void }) {
  // Local-only state so the placeholder controls actually respond.
  const [launchBehavior, setLaunchBehavior] = useState("keep");
  const [reopenOnClose, setReopenOnClose] = useState(true);
  const [checkUpdates, setCheckUpdates] = useState(true);
  const [prerelease, setPrerelease] = useState(false);
  const [ram, setRam] = useState(6);
  const [animatedBg, setAnimatedBg] = useState(true);
  const [closeToTray, setCloseToTray] = useState(false);

  return (
    <div className="settings-view">
      <div className="settings-head">
        <button className="settings-back" onClick={onBack} title="Back">
          <LuArrowLeft />
        </button>
        <h2>Settings</h2>
      </div>

      <div className="settings-scroll">
        <section className="settings-section">
          <h3>General</h3>
          <Row title="When the game launches" desc="What the launcher does after Minecraft starts.">
            <Dropdown
              value={launchBehavior}
              onChange={setLaunchBehavior}
              options={[
                { value: "keep", label: "Keep launcher open" },
                { value: "minimize", label: "Minimize launcher" },
                { value: "close", label: "Close launcher" },
              ]}
            />
          </Row>
          <Row title="Reopen launcher when game closes" desc="Bring the launcher back after you quit Minecraft.">
            <Toggle on={reopenOnClose} onChange={setReopenOnClose} />
          </Row>
          <Row title="Close to system tray" desc="Keep running in the background instead of fully closing.">
            <Toggle on={closeToTray} onChange={setCloseToTray} />
          </Row>
        </section>

        <section className="settings-section">
          <h3>Updates</h3>
          <Row title="Check for updates on startup" desc="Look for a newer version each time the launcher opens.">
            <Toggle on={checkUpdates} onChange={setCheckUpdates} />
          </Row>
          <Row title="Receive pre-release updates" desc="Get early test builds. May contain bugs.">
            <Toggle on={prerelease} onChange={setPrerelease} />
          </Row>
        </section>

        <section className="settings-section">
          <h3>Game</h3>
          <Row title="Default memory" desc={`${ram} GB allocated to new installations.`}>
            <div className="setting-slider">
              <input
                type="range"
                min={2}
                max={16}
                step={1}
                value={ram}
                onChange={(e) => setRam(Number(e.target.value))}
              />
            </div>
          </Row>
        </section>

        <section className="settings-section">
          <h3>Appearance</h3>
          <Row title="Animated background" desc="Subtle motion on the Play screen hero.">
            <Toggle on={animatedBg} onChange={setAnimatedBg} />
          </Row>
        </section>
      </div>
    </div>
  );
}
