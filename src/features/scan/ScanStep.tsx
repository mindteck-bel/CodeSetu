import type { ProgressEvent } from "../../shared/types";

type Props = {
  progress: ProgressEvent | null;
  scanError: string | null;
  onCancel: () => void;
  onRetry: () => void;
  onBack: () => void;
};

export function ScanStep({
  progress,
  scanError,
  onCancel,
  onRetry,
  onBack,
}: Props) {
  return (
    <section className="card-free scan-step enter">
      <h1>Scanning</h1>
      <p className="lede">
        {progress?.message ?? "Starting…"} Both trees are hashed in parallel —
        you can cancel anytime.
      </p>
      <div className="scan-meter" aria-live="polite">
        <div className="scan-stats">
          <div>
            <span className="stat-num">{progress?.filesSeen ?? 0}</span>
            <span className="stat-label">files seen</span>
          </div>
          <div>
            <span className="stat-num">{progress?.filesHashed ?? 0}</span>
            <span className="stat-label">hashed</span>
          </div>
        </div>
        <p className="muted tiny">Phase: {progress?.phase ?? "starting"}</p>
        <p className="current-path">
          {progress?.currentPath || "Preparing workers…"}
        </p>
        <div className="pulse-bar" />
      </div>
      {scanError ? (
        <div className="fallback-panel" role="alert">
          <p className="eyebrow">Scan failed</p>
          <p>{scanError}</p>
          <p className="muted tiny">
            Common fixes: re-select folders if a USB letter changed, confirm both
            paths are readable, then retry. No destination files were modified by
            this scan.
          </p>
          <div className="actions">
            <button type="button" className="btn" onClick={onBack}>
              ← Back to Folders
            </button>
            <button type="button" className="btn primary" onClick={onRetry}>
              Retry scan
            </button>
          </div>
        </div>
      ) : (
        <div className="actions">
          <button type="button" className="btn ghost" onClick={onBack}>
            ← Back to Folders
          </button>
          <button type="button" className="btn ghost" onClick={onCancel}>
            Cancel scan
          </button>
        </div>
      )}
    </section>
  );
}
