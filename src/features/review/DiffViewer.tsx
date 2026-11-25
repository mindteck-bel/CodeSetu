import { useEffect, useMemo, useState } from "react";
import { getFileDiff } from "../../shared/api";
import {
  keepActionLabel,
  modeCopy,
  primaryActionLabel,
  type CompareMode,
} from "../../shared/compareMode";
import { folderCaption } from "../../shared/folderLabel";
import type { Decision, FileDiff, FileRow } from "../../shared/types";
import { alignSplitHunks, gapPlaceholder } from "./alignedDiff";

export type DiffLayout = "split" | "unified";

type Props = {
  mode: CompareMode;
  sessionId: string;
  relPath: string;
  row: FileRow | null;
  destinationPath: string;
  sourcePath: string;
  layout: DiffLayout;
  onLayoutChange: (layout: DiffLayout) => void;
  onDecision: (id: string, decision: Decision) => void;
};

export function DiffViewer({
  mode,
  sessionId,
  relPath,
  row,
  destinationPath,
  sourcePath,
  layout,
  onLayoutChange,
  onDecision,
}: Props) {
  const copy = modeCopy(mode);
  const dest = folderCaption(destinationPath);
  const source = folderCaption(sourcePath);
  const [diff, setDiff] = useState<FileDiff | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let alive = true;
    setLoading(true);
    setError(null);
    getFileDiff(sessionId, relPath)
      .then((d) => {
        if (alive) setDiff(d);
      })
      .catch((e: Error) => {
        if (alive) setError(e.message ?? String(e));
      })
      .finally(() => {
        if (alive) setLoading(false);
      });
    return () => {
      alive = false;
    };
  }, [sessionId, relPath]);

  const aligned = useMemo(
    () => (diff?.kind === "text" ? alignSplitHunks(diff.hunks) : []),
    [diff],
  );

  const isDeletion = row?.class === "deletedBrought";
  const actionsDisabled =
    !row ||
    row.class === "intranetOnly" ||
    row.class === "internetOnly" ||
    row.class === "parkedSecret";

  return (
    <div className="diff-pane" aria-label="File diff">
      <div className="diff-header">
        <div className="diff-header-text">
          <p className="eyebrow">
            {source.name} (source) → {dest.name} (destination)
          </p>
          <h3>{relPath}</h3>
          <p className="muted tiny">{copy.diffHint}</p>
          {row?.error && (
            <p className="muted tiny conflict-hint">{row.error}</p>
          )}
        </div>
        <div className="diff-header-controls">
          <div className="diff-layout-toggle" role="group" aria-label="Diff layout">
            <button
              type="button"
              className={layout === "split" ? "chip on" : "chip"}
              onClick={() => onLayoutChange("split")}
            >
              Split
            </button>
            <button
              type="button"
              className={layout === "unified" ? "chip on" : "chip"}
              onClick={() => onLayoutChange("unified")}
            >
              Unified
            </button>
          </div>
          {row && (
            <div className="diff-file-actions">
              <button
                type="button"
                className={
                  row.decision === "bringIn"
                    ? `chip on${isDeletion ? " danger" : ""}`
                    : "chip"
                }
                disabled={actionsDisabled}
                title={isDeletion ? copy.deleteTooltip : copy.bringTooltip}
                onClick={() => onDecision(row.id, "bringIn")}
              >
                {primaryActionLabel(mode, row)}
              </button>
              <button
                type="button"
                className={row.decision === "keep" ? "chip on" : "chip"}
                title={copy.keepTooltip}
                onClick={() => onDecision(row.id, "keep")}
              >
                {keepActionLabel(mode, row)}
              </button>
              <button
                type="button"
                className={row.decision === "skip" ? "chip on" : "chip"}
                onClick={() => onDecision(row.id, "skip")}
              >
                Skip
              </button>
            </div>
          )}
        </div>
      </div>

      {loading && <div className="panel-loading">Reading file…</div>}
      {error && (
        <div className="fallback-panel compact">
          <p>Could not open this diff.</p>
          <pre className="fallback-detail">{error}</pre>
        </div>
      )}
      {diff && !loading && (
        <>
          {diff.error && <p className="warn-line">{diff.error}</p>}
          {(diff.kind === "binary" || diff.kind === "large") && (
            <div className="fallback-panel compact">
              <p>
                This file is {diff.kind}. {copy.binaryHint}
              </p>
            </div>
          )}
          {diff.kind === "text" && layout === "split" && (
            <div className="diff-grid aligned">
              <div className="diff-col-headers">
                <div className="diff-col-title" title={source.full || undefined}>
                  <span className="diff-folder-name">{source.name}</span>
                  <span className="diff-folder-role">{copy.diffLeftTitle}</span>
                  {source.pathHint ? (
                    <span className="diff-folder-path">{source.pathHint}</span>
                  ) : null}
                </div>
                <div className="diff-col-title" title={dest.full || undefined}>
                  <span className="diff-folder-name">{dest.name}</span>
                  <span className="diff-folder-role">{copy.diffRightTitle}</span>
                  {dest.pathHint ? (
                    <span className="diff-folder-path">{dest.pathHint}</span>
                  ) : null}
                </div>
              </div>
              <div className="diff-scroll" tabIndex={0}>
                <div className="diff-aligned-body">
                  {aligned.map((rowAlign) => {
                    const sourceGap = rowAlign.sourceText === null;
                    const destGap = rowAlign.destText === null;
                    const gapFrom =
                      rowAlign.sourceText ?? rowAlign.destText ?? "\n";
                    return (
                      <div key={rowAlign.key} className="diff-aligned-row">
                        <pre
                          className={`diff-cell hunk ${
                            sourceGap ? "gap" : rowAlign.kind
                          }`}
                        >
                          {sourceGap
                            ? gapPlaceholder(gapFrom)
                            : rowAlign.sourceText}
                        </pre>
                        <pre
                          className={`diff-cell hunk ${
                            destGap ? "gap" : rowAlign.kind
                          }`}
                        >
                          {destGap
                            ? gapPlaceholder(gapFrom)
                            : rowAlign.destText}
                        </pre>
                      </div>
                    );
                  })}
                </div>
              </div>
            </div>
          )}
          {diff.kind === "text" && layout === "unified" && (
            <div className="diff-unified">
              <div className="diff-col-title">
                Unified · −{dest.name} / +{source.name}
              </div>
              <pre className="diff-body">
                {diff.hunks.map((h, i) => {
                  if (h.tag === "equal") {
                    return (
                      <span key={i} className="hunk equal">
                        {prefixLines(h.thisText, " ")}
                      </span>
                    );
                  }
                  if (h.tag === "delete") {
                    return (
                      <span key={i} className="hunk delete">
                        {prefixLines(h.thisText, "-")}
                      </span>
                    );
                  }
                  if (h.tag === "insert") {
                    return (
                      <span key={i} className="hunk insert">
                        {prefixLines(h.broughtText, "+")}
                      </span>
                    );
                  }
                  return (
                    <span key={i} className={`hunk ${h.tag}`}>
                      {h.thisText || h.broughtText}
                    </span>
                  );
                })}
              </pre>
            </div>
          )}
          {diff.truncated && (
            <p className="muted">Diff truncated for speed.</p>
          )}
        </>
      )}
    </div>
  );
}

function prefixLines(text: string, mark: string): string {
  if (!text) return "";
  const endsWithNl = text.endsWith("\n");
  const lines = text.split("\n");
  if (endsWithNl) lines.pop();
  return `${lines.map((l) => `${mark}${l}`).join("\n")}${endsWithNl ? "\n" : ""}`;
}
