import { useMemo, useRef } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import {
  changeClassLabel,
  keepActionLabel,
  modeCopy,
  primaryActionLabel,
  type CompareMode,
} from "../../shared/compareMode";
import type { Decision, FileRow, ReviewFilter } from "../../shared/types";

type Props = {
  mode: CompareMode;
  rows: FileRow[];
  filter: ReviewFilter;
  onDecision: (id: string, decision: Decision) => void;
  onOpen: (relPath: string) => void;
};

export function decisionLabelsForTest(
  row: FileRow,
  mode: CompareMode = "airgap",
) {
  return {
    primary: primaryActionLabel(mode, row),
    keep: keepActionLabel(mode, row),
    skip: "Skip",
  };
}

export function filterRows(rows: FileRow[], filter: ReviewFilter): FileRow[] {
  return rows.filter((r) => {
    switch (filter) {
      case "needsYou":
        return r.class === "needsYou";
      case "bringIn":
        return r.class === "bringIn" || r.class === "moved";
      case "keep":
        return r.class === "keep";
      case "deletions":
        return r.class === "deletedBrought";
      case "other":
        return (
          r.class === "intranetOnly" ||
          r.class === "internetOnly" ||
          r.class === "parkedSecret"
        );
      default:
        return r.class !== "identical";
    }
  });
}

export function ReviewList({ mode, rows, filter, onDecision, onOpen }: Props) {
  const copy = modeCopy(mode);
  const filtered = useMemo(() => filterRows(rows, filter), [rows, filter]);

  const parentRef = useRef<HTMLDivElement>(null);
  const virtualizer = useVirtualizer({
    count: filtered.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 44,
    overscan: 12,
  });

  if (filtered.length === 0) {
    return (
      <div className="fallback-panel compact">
        <p>
          {filter === "needsYou"
            ? "Nothing needs a decision. Safe changes are already selected."
            : "No files in this filter."}
        </p>
      </div>
    );
  }

  return (
    <div className="review-scroll" ref={parentRef}>
      <div
        className="review-inner"
        style={{ height: `${virtualizer.getTotalSize()}px` }}
      >
        {virtualizer.getVirtualItems().map((item) => {
          const row = filtered[item.index];
          const isDeletion = row.class === "deletedBrought";
          return (
            <div
              key={row.id}
              className={`review-row class-${row.class}${isDeletion ? " destructive" : ""}`}
              style={{
                position: "absolute",
                top: 0,
                left: 0,
                width: "100%",
                height: `${item.size}px`,
                transform: `translateY(${item.start}px)`,
              }}
            >
              <button
                type="button"
                className="row-main"
                onClick={() => onOpen(row.relPath)}
              >
                <span className="badge">{changeClassLabel(mode, row.class)}</span>
                <span className="path">{row.relPath}</span>
                {row.movedFrom && (
                  <span className="muted tiny">from {row.movedFrom}</span>
                )}
                {row.class === "needsYou" && row.error && (
                  <span className="muted tiny conflict-hint">{row.error}</span>
                )}
              </button>
              <div className="row-actions">
                <button
                  type="button"
                  className={
                    row.decision === "bringIn"
                      ? `chip on${isDeletion ? " danger" : ""}`
                      : "chip"
                  }
                  onClick={() => onDecision(row.id, "bringIn")}
                  disabled={
                    row.class === "intranetOnly" ||
                    row.class === "internetOnly" ||
                    row.class === "parkedSecret"
                  }
                  title={isDeletion ? copy.deleteTooltip : copy.bringTooltip}
                >
                  {primaryActionLabel(mode, row)}
                </button>
                <button
                  type="button"
                  className={row.decision === "keep" ? "chip on" : "chip"}
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
            </div>
          );
        })}
      </div>
    </div>
  );
}
