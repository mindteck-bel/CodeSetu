import { useMemo, useState } from "react";
import { modeCopy, type CompareMode } from "../../shared/compareMode";
import { ErrorBoundary } from "../../shared/ErrorBoundary";
import type {
  Decision,
  FileRow,
  ReviewFilter,
  ScanResult,
} from "../../shared/types";
import { DiffViewer, type DiffLayout } from "./DiffViewer";
import { FileTree } from "./FileTree";
import { filterRows } from "./ReviewList";

type Props = {
  mode: CompareMode;
  result: ScanResult;
  rows: FileRow[];
  filter: ReviewFilter;
  applying: boolean;
  rulesDirty: boolean;
  applyError: string | null;
  sessionId: string | null;
  openPath: string | null;
  selectedIds: Set<string>;
  onFilter: (f: ReviewFilter) => void;
  onDecision: (id: string, decision: Decision) => void;
  onOpen: (path: string | null) => void;
  onBulkNeedsYou: (decision: Decision) => void;
  onApply: () => void;
  onBack: () => void;
  onToggleSelect: (id: string, selected: boolean) => void;
  onToggleSelectMany: (ids: string[], selected: boolean) => void;
};

export function ReviewStep({
  mode,
  result,
  rows,
  filter,
  applying,
  rulesDirty,
  applyError,
  sessionId,
  openPath,
  selectedIds,
  onFilter,
  onDecision,
  onOpen,
  onBulkNeedsYou,
  onApply,
  onBack,
  onToggleSelect,
  onToggleSelectMany,
}: Props) {
  const copy = modeCopy(mode);
  const s = result.summary;
  const summaryLine = `${s.bringIn} ${copy.summaryBringIn} · ${s.keep} ${copy.summaryKeep} · ${s.needsYou} ${copy.summaryConflict} · ${s.deletedBrought} deletions · ${s.intranetOnly} ${copy.summaryDestOnly} · ${s.internetOnly} ${copy.summarySourceOnly}`;
  const firstSync = Boolean(s.firstSync);
  const hasSelection = selectedIds.size > 0;
  const [diffLayout, setDiffLayout] = useState<DiffLayout>("split");

  const filtered = useMemo(() => filterRows(rows, filter), [rows, filter]);
  const openRow = useMemo(
    () => (openPath ? (rows.find((r) => r.relPath === openPath) ?? null) : null),
    [rows, openPath],
  );

  return (
    <section className="review-step enter">
      <div className="review-head">
        <div>
          <div className="review-title-row">
            <button type="button" className="btn ghost" onClick={onBack}>
              ← Back to Folders
            </button>
            <h1>Review changes</h1>
          </div>
          <p className="lede summary-line">{summaryLine}</p>
          <p className="muted tiny">
            {copy.reviewArrow} · {(result.summary.elapsedMs / 1000).toFixed(1)}s
            · {result.summary.identical} identical hidden · {copy.sideLegend}
          </p>
          {firstSync && (
            <p className="warn-line">
              First sync — confirm carefully. Bulk Use source is disabled unless
              files are selected.
            </p>
          )}
          {rulesDirty && (
            <p className="error-line">
              Ignore rules changed — rescan before apply.
            </p>
          )}
          <p className="muted tiny selection-hint">
            {hasSelection
              ? `Applying and bulk actions will use ${selectedIds.size} selected file(s) only.`
              : copy.emptySelectionHint}
          </p>
        </div>
        <div className="actions">
          <button
            type="button"
            className="btn"
            disabled={firstSync && !hasSelection}
            title={
              firstSync && !hasSelection
                ? "Disabled on first sync — resolve conflicts individually or select files"
                : undefined
            }
            onClick={() => onBulkNeedsYou("bringIn")}
          >
            {hasSelection ? copy.bringIntoSelected : copy.bringIntoAll}
          </button>
          <button
            type="button"
            className="btn"
            onClick={() => onBulkNeedsYou("keep")}
          >
            {hasSelection ? copy.keepSelected : copy.keepAll}
          </button>
          <button
            type="button"
            className="btn primary large"
            disabled={applying || rulesDirty}
            onClick={onApply}
          >
            {applying
              ? "Applying…"
              : hasSelection
                ? copy.applySelectedButton
                : copy.applyButton}
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
            ["needsYou", copy.filterConflict],
            ["bringIn", copy.filterBringIn],
            ["keep", copy.filterKeep],
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

      <div className="review-split">
        <ErrorBoundary fallbackTitle="File tree failed">
          <FileTree
            mode={mode}
            rows={filtered}
            selectedIds={selectedIds}
            openPath={openPath}
            onToggleSelect={onToggleSelect}
            onToggleSelectMany={onToggleSelectMany}
            onOpen={(p) => onOpen(p)}
          />
        </ErrorBoundary>

        <div className="review-diff-host">
          {openPath && sessionId ? (
            <ErrorBoundary fallbackTitle="Diff viewer failed">
              <DiffViewer
                mode={mode}
                sessionId={sessionId}
                relPath={openPath}
                row={openRow}
                layout={diffLayout}
                onLayoutChange={setDiffLayout}
                onDecision={onDecision}
              />
            </ErrorBoundary>
          ) : (
            <div className="fallback-panel compact diff-placeholder">
              <p>Select a file in the tree to view its diff here.</p>
              <p className="muted tiny">
                Split and Unified views stay in this pane — the file list stays
                visible.
              </p>
            </div>
          )}
        </div>
      </div>

      {applyError && <p className="error-line">{applyError}</p>}
    </section>
  );
}
