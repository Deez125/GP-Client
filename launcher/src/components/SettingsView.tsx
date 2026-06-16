import { useEffect, useState } from "react";
import { LuArrowLeft } from "react-icons/lu";
import { Dropdown } from "./Dropdown";
import { useSettings } from "../hooks/useSettings";
import { getCurrentVersion, getReleaseNotes } from "../lib/updates";

// A simple on/off switch used by the settings rows.
function Toggle({
  on,
  onChange,
  disabled,
}: {
  on: boolean;
  onChange: (v: boolean) => void;
  disabled?: boolean;
}) {
  return (
    <button
      type="button"
      className={`switch${on ? " on" : ""}`}
      onClick={() => !disabled && onChange(!on)}
      disabled={disabled}
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
// dock.
export function SettingsView({ onBack }: { onBack: () => void }) {
  const { settings, update } = useSettings();
  const [version, setVersion] = useState("");
  // "Pre-release" | "Release" | null (unknown / no matching GitHub release).
  const [channel, setChannel] = useState<string | null>(null);

  useEffect(() => {
    getCurrentVersion()
      .then(setVersion)
      .catch(() => {});
    // Best-effort: the channel comes from this version's GitHub release.
    getReleaseNotes()
      .then((n) => {
        if (n.prerelease != null) {
          setChannel(n.prerelease ? "Pre-release" : "Release");
        }
      })
      .catch(() => {});
  }, []);

  // These two are temporarily disabled; force them off in storage if a prior
  // version (or default) left them on.
  useEffect(() => {
    if (settings && (settings.reopen_on_close || settings.close_to_tray)) {
      update({ reopen_on_close: false, close_to_tray: false });
    }
  }, [settings]);

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
              value={settings?.launch_behavior ?? "keep"}
              disabled={!settings}
              onChange={(v) =>
                update({ launch_behavior: v as "keep" | "minimize" | "close" })
              }
              options={[
                { value: "keep", label: "Keep launcher open" },
                { value: "minimize", label: "Minimize launcher" },
                { value: "close", label: "Close launcher" },
              ]}
            />
          </Row>
          <Row title="Reopen launcher when game closes" desc="Bring the launcher back after you quit Minecraft. (Coming soon)">
            <Toggle on={false} disabled onChange={() => {}} />
          </Row>
          <Row title="Close to system tray" desc="Keep running in the background instead of fully closing. (Coming soon)">
            <Toggle on={false} disabled onChange={() => {}} />
          </Row>
        </section>

        <section className="settings-section">
          <h3>Updates</h3>
          <Row title="Check for updates on startup" desc="Look for a newer version each time the launcher opens.">
            <Toggle
              on={settings?.check_updates_on_startup ?? true}
              onChange={(v) => update({ check_updates_on_startup: v })}
            />
          </Row>
          <Row title="Receive pre-release updates" desc="Get early test builds. May contain bugs.">
            <Toggle
              on={settings?.prerelease_updates ?? false}
              onChange={(v) => update({ prerelease_updates: v })}
            />
          </Row>
        </section>

        <section className="settings-section">
          <h3>Game</h3>
          <Row
            title="Default memory"
            desc={`${settings?.default_memory_gb ?? 6} GB allocated to new installations.`}
          >
            <div className="setting-slider">
              <input
                type="range"
                min={2}
                max={16}
                step={1}
                value={settings?.default_memory_gb ?? 6}
                disabled={!settings}
                onChange={(e) => update({ default_memory_gb: Number(e.target.value) })}
              />
            </div>
          </Row>
        </section>

        <section className="settings-section">
          <h3>Appearance</h3>
          <Row title="Animated background" desc="Subtle motion on the Play screen hero.">
            <Toggle
              on={settings?.animated_background ?? true}
              onChange={(v) => update({ animated_background: v })}
            />
          </Row>
        </section>

        <section className="settings-section">
          <h3>About</h3>
          <Row title="GP Client" desc="Current version">
            <span className="setting-version">
              {version ? `v${version}` : "…"}
              {channel && (
                <span
                  className={`channel-badge${channel === "Pre-release" ? " pre" : ""}`}
                >
                  {channel}
                </span>
              )}
            </span>
          </Row>
        </section>
      </div>
    </div>
  );
}
