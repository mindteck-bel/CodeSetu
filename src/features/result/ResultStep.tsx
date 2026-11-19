import { modeCopy, type CompareMode } from "../../shared/compareMode";
import { failureRecoveryHint } from "../../shared/userError";
import type { ApplyReport } from "../../shared/types";

type Props = {
  mode: CompareMode;
  applyReport: ApplyReport;
  applyError: string | null;
  diagBusy?: boolean;
  diagMessage?: string | null;
  onUndo: () => void;
  onReset: () => void;
  onExportDiagnostics?: () => void;
};

function resultTitle(mode: CompareMode, report: ApplyReport): string {
  const copy = modeCopy(mode);
  if (report.recovered) return "Recovered / undone";
  if ((report.failed ?? 0) > 0 && (report.applied ?? 0) === 0) {
    return "Apply finished with failures";
  }
  if ((report.failed ?? 0) > 0) return "Apply finished with partial failures";
  if ((report.applied ?? 0) > 0) return copy.resultTitle;
  if ((report.kept ?? 0) > 0) return copy.resultKeptTitle;
  return copy.resultNothingTitle;
}

function plural(n: number, one: string, many: string): string {
  return n === 1 ? `${n} ${one}` : `${n} ${many}`;
}

function resultSummary(report: ApplyReport): string {
  if (report.message) return report.message;
  const parts: string[] = [];
  if (report.applied > 0) {
    parts.push(
      `${plural(report.applied, "file", "files")} updated on destination`,
    );
  }
  if ((report.kept ?? 0) > 0) {
    parts.push(
      `${report.kept} kept as mine (destination unchanged)`,
    );
  }
  if (report.skipped > 0) {
    parts.push(
      `${report.skipped} left unresolved (Skip) — destination not changed`,
    );
  }
  if (report.failed > 0) {
    parts.push(`${report.failed} failed`);
  }
  if (parts.length === 0) {
    return "Nothing changed on the destination folder.";
  }
  if (report.applied === 0 && report.failed === 0) {
    return `Nothing written to destination · ${parts.join(" · ")}`;
  }
  return parts.join(" · ");
}

export function ResultStep({
  mode,
  applyReport,
  applyError,
  diagBusy,
  diagMessage,
  onUndo,
  onReset,
  onExportDiagnostics,
}: Props) {
  const copy = modeCopy(mode);
  const wroteFiles = (applyReport.applied ?? 0) > 0;
  const failed = applyReport.failed ?? 0;
  const recovery = failureRecoveryHint(
    failed,
    Boolean(applyReport.journalId) || Boolean(applyReport.recovered),
  );

  return (
    <section className="card-free result-step enter">
      <h1>{resultTitle(mode, applyReport)}</h1>
      <p className="lede">{resultSummary(applyReport)}</p>
      {applyReport.journalId && (
        <p className="muted">Journal: {applyReport.journalId}</p>
      )}
      <p className="muted">{wroteFiles ? copy.resultBody : copy.resultBodyNothing}</p>

      {failed > 0 && (
        <div className="fallback-panel" role="alert">
          <p className="eyebrow">Failures ({failed})</p>
          <p>{recovery}</p>
          <ul className="failure-list">
            {applyReport.failures.map((f) => (
              <li key={f}>
                <code>{f}</code>
              </li>
            ))}
          </ul>
          <div className="actions tight">
            <button type="button" className="btn" onClick={onUndo}>
              Undo last apply
            </button>
            {onExportDiagnostics && (
              <button
                type="button"
                className="btn"
                disabled={diagBusy}
                onClick={onExportDiagnostics}
              >
                {diagBusy ? "Exporting…" : "Export diagnostics"}
              </button>
            )}
          </div>
        </div>
      )}

      {failed === 0 && applyReport.failures.length > 0 && (
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
      {diagMessage && <p className="muted tiny">{diagMessage}</p>}

      <div className="actions">
        {failed === 0 && (
          <button type="button" className="btn" onClick={onUndo}>
            Undo last apply
          </button>
        )}
        {failed === 0 && onExportDiagnostics && (
          <button
            type="button"
            className="btn"
            disabled={diagBusy}
            onClick={onExportDiagnostics}
          >
            {diagBusy ? "Exporting…" : "Export diagnostics"}
          </button>
        )}
        <button type="button" className="btn primary" onClick={onReset}>
          New sync
        </button>
      </div>
    </section>
  );
}
