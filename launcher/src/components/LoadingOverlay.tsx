import { useState } from "react";

// Full-window loading screen shown until the app's initial data (account +
// skin) is ready, so the UI doesn't pop in piecemeal. The ring spinner is the
// classic CSS "lds-ring" (loading-ui.com style).
const PHRASES = [
  "Warming up",
  "Spinning up",
  "Getting things ready",
  "Loading up",
  "Almost there",
  "Booting up",
  "Hang tight",
];

export function LoadingOverlay() {
  // Pick one phrase per mount.
  const [phrase] = useState(
    () => PHRASES[Math.floor(Math.random() * PHRASES.length)],
  );

  return (
    <div className="loading-overlay">
      <div className="lds-ring">
        <div />
        <div />
        <div />
        <div />
      </div>
      <p className="loading-text">{phrase}</p>
    </div>
  );
}
