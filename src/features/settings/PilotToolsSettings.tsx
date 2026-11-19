import { useEffect, useState } from "react";
import { getSyncHistory } from "../../shared/api";
import type { HistoryEntry } from "../../shared/types";

type Props = {
  diagBusy?: boolean;
  diagMessage?: string | null;
  onExportDiagnostics: () => void;
};

function kindLabel(kind: string): string {
  switch (kind) {
    case "apply":
      return "Apply";
    case "undo":
      return "Undo";
    case "recover":
      return "Recover";
    default:
      return kind;
  }
}

function formatWhen(ts: string): string {
  const d = Date.parse(ts);
  if (Number.isNaN(d)) return ts;
  return new Date(d).toLocaleString();
}

export function PilotToolsSettings({
  diagBusy,
  diagMessage,
  onExportDiagnostics,
}: Props) {
  const [history, setHistory] = useState<HistoryEntry[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    getSyncHistory(30)
      .then(setHistory)
      .catch((e) => setError(e instanceof Error ? e.message : String(e)));
  }, []);

  return (
    <section className="pilot-tools">
      <h2>Pilot tools</h2>
      <p className="muted">
        Local-only diagnostics and sync history for enterprise pilots. Nothing
        is uploaded — you choose where to save the report.
      </p>

      <div className="actions tight">
        <button
          type="button"
          className="btn primary"
          disabled={diagBusy}
          onClick={onExportDiagnostics}
        >
          {diagBusy ? "Exporting…" : "Export diagnostic report"}
        </button>
      </div>
      {diagMessage && <p className="muted tiny">{diagMessage}</p>}

      <p className="eyebrow" style={{ marginTop: "1.25rem" }}>
        Recent sync history
      </p>
      {error && <p className="error-line">{error}</p>}
      {!error && history.length === 0 && (
        <p className="muted tiny">No apply / undo events recorded yet.</p>
      )}
      {history.length > 0 && (
        <ul className="history-list">
          {history.map((h, i) => (
            <li key={`${h.ts}-${h.kind}-${i}`}>
              <span className="history-kind">{kindLabel(h.kind)}</span>
              <span className="history-when">{formatWhen(h.ts)}</span>
              <span className="muted tiny">
                applied {h.applied} · kept {h.kept} · failed {h.failed}
                {h.journalId ? ` · ${h.journalId.slice(0, 8)}` : ""}
              </span>
              {h.message && <span className="history-msg">{h.message}</span>}
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
