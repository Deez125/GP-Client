import { useEffect, useState } from "react";

// Centered ring spinner with cycling text, used in the What's New page and the
// Mods popup while their data loads.
const PHRASES = [
  "Loading",
  "Gathering",
  "Fetching",
  "Catching up",
  "Checking GitHub",
];

export function LoadingSpinner() {
  const [i, setI] = useState(0);
  useEffect(() => {
    const t = setInterval(() => setI((p) => (p + 1) % PHRASES.length), 1100);
    return () => clearInterval(t);
  }, []);

  return (
    <div className="loading-spinner">
      <div className="lds-ring">
        <div />
        <div />
        <div />
        <div />
      </div>
      <p className="loading-text">{PHRASES[i]}</p>
    </div>
  );
}
