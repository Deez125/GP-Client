// Full-window loading screen shown until the app's initial data (account +
// skin) is ready, so the UI doesn't pop in piecemeal. The ring spinner is the
// classic CSS "lds-ring" (loading-ui.com style).
export function LoadingOverlay() {
  return (
    <div className="loading-overlay">
      <div className="lds-ring">
        <div />
        <div />
        <div />
        <div />
      </div>
      <p className="loading-text">Loading</p>
    </div>
  );
}
