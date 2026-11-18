import { ErrorBoundary } from "../../shared/ErrorBoundary";
import type {
  Decision,
  FileRow,
  ReviewFilter,
  ScanResult,
} from "../../shared/types";
import { DiffViewer } from "./DiffViewer";
import { ReviewList } from "./ReviewList";

type Props = {
  result: ScanResult;
  rows: FileRow[];
  filter: ReviewFilter;
  applying: boolean;
  rulesDirty: boolean;
  applyError: string | null;
  sessionId: string | null;
  openPath: string | null;
  onFilter: (f: ReviewFilter) => void;
  onDecision: (id: string, decision: Decision) => void;
  onOpen: (path: string | null) => void;
  onBulkNeedsYou: (decision: Decision) => void;
  onApply: () => void;
};

export function ReviewStep({
  result,
  rows,
  filter,
  applying,
  rulesDirty,
  applyError,
  sessionId,
  openPath,
  onFilter,
  onDecision,
  onOpen,
  onBulkNeedsYou,
  onApply,
}: Props) {
  const s = result.summary;
  const summaryLine = `${s.bringIn} bring into Intranet · ${s.keep} keep Intranet · ${s.needsYou} need you · ${s.deletedBrought} deletions · ${s.intranetOnly} intranet-only · ${s.internetOnly} internet-only`;
  const firstSync = Boolean(s.firstSync);

  return (
    <section className="review-step enter">
      <div className="review-head">
        <div>
          <h1>Review changes</h1>
          <p className="lede summary-line">{summaryLine}</p>
          <p className="muted">
            Intranet ← Internet · scanned in{" "}
            {(result.summary.elapsedMs / 1000).toFixed(1)}s ·{" "}
            {result.summary.identical} identical hidden
          </p>
          <p className="muted tiny side-legend">
            Left = Intranet (This PC, writable) · Right = Internet (read-only).
            Conflicts need an explicit choice — neither side is taken
            automatically.
          </p>
          {firstSync && (
            <p className="warn-line">
              First sync — new files are pre-selected; confirm carefully before
              apply. Bulk conflict Bring In is disabled.
            </p>
          )}
          {rulesDirty && (
            <p className="error-line">
              Ignore rules changed — rescan before apply.
            </p>
          )}
        </div>
        <div className="actions">
          <button
            type="button"
            className="btn"
            disabled={firstSync}
            title={
              firstSync
                ? "Disabled on first sync — resolve conflicts individually"
                : undefined
            }
            onClick={() => onBulkNeedsYou("bringIn")}
          >
            Bring into Intranet (all conflicts)
          </button>
          <button
            type="button"
            className="btn"
            onClick={() => onBulkNeedsYou("keep")}
          >
            Keep Intranet (all conflicts)
          </button>
          <button
            type="button"
            className="btn primary large"
            disabled={applying || rulesDirty}
            onClick={onApply}
          >
            {applying ? "Applying…" : "Apply to Intranet"}
          </button>
        </div>
      </div>

      {result.warnings.length > 0 && (
        <ul className="warn-list">
          {result.warnings.slice(0, 5).map((w) => (
            <li key={w}>{w}</li>
          ))}
        </ul>
      )}

      <div className="filter-bar">
        {(
          [
            ["needsYou", "Needs you"],
            ["bringIn", "Bring into Intranet"],
            ["keep", "Keep Intranet"],
            ["deletions", `Deletions (${result.summary.deletedBrought})`],
            ["other", "Env / secret"],
            ["all", "All changes"],
          ] as const
        ).map(([key, label]) => (
          <button
            key={key}
            type="button"
            className={filter === key ? "chip on" : "chip"}
            onClick={() => onFilter(key)}
          >
            {label}
          </button>
        ))}
      </div>

      <ErrorBoundary fallbackTitle="Review list failed">
        <ReviewList
          rows={rows}
          filter={filter}
          onDecision={onDecision}
          onOpen={(p) => onOpen(p)}
        />
      </ErrorBoundary>

      {applyError && <p className="error-line">{applyError}</p>}

      {openPath && sessionId && (
        <ErrorBoundary fallbackTitle="Diff viewer failed">
          <DiffViewer
            sessionId={sessionId}
            relPath={openPath}
            onClose={() => onOpen(null)}
          />
        </ErrorBoundary>
      )}
    </section>
  );
}
