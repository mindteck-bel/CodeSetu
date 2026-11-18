import type { ApplyReport } from "../../shared/types";

type Props = {
  applyReport: ApplyReport;
  applyError: string | null;
  onUndo: () => void;
  onReset: () => void;
};

export function ResultStep({
  applyReport,
  applyError,
  onUndo,
  onReset,
}: Props) {
  return (
    <section className="card-free result-step enter">
      <h1>{applyReport.recovered ? "Recovered / undone" : "Sync applied"}</h1>
      <p className="lede">
        {applyReport.message
          ? applyReport.message
          : `${applyReport.applied} files updated · ${applyReport.skipped} skipped · ${applyReport.failed} failed`}
      </p>
      {applyReport.journalId && (
        <p className="muted">Journal: {applyReport.journalId}</p>
      )}
      <p className="muted">
        Commit and push with your usual Git remote. CodeSetu does not push.
        Undo restores files and the snapshot baseline.
      </p>
      {applyReport.failures.length > 0 && (
        <div className="fallback-panel compact">
          <p>Some files failed — others still applied.</p>
          <ul>
            {applyReport.failures.slice(0, 8).map((f) => (
              <li key={f}>{f}</li>
            ))}
          </ul>
        </div>
      )}
      {applyError && <p className="error-line">{applyError}</p>}
      <div className="actions">
        <button type="button" className="btn" onClick={onUndo}>
          Undo last apply
        </button>
        <button type="button" className="btn primary" onClick={onReset}>
          New sync
        </button>
      </div>
    </section>
  );
}
