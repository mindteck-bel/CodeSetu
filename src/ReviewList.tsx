import { useMemo, useRef } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import type { ChangeClass, Decision, FileRow } from "./types";

type Filter = "needsYou" | "bringIn" | "keep" | "all" | "other";

type Props = {
  rows: FileRow[];
  filter: Filter;
  onDecision: (id: string, decision: Decision) => void;
  onOpen: (relPath: string) => void;
};

const LABELS: Record<ChangeClass, string> = {
  identical: "Same",
  bringIn: "Bring in",
  keep: "Keep",
  needsYou: "Needs you",
  addedBrought: "New",
  deletedBrought: "Deleted over there",
  moved: "Moved",
  intranetOnly: "Intranet only",
  internetOnly: "Internet only",
  parkedSecret: "Secret / env",
};

export function ReviewList({ rows, filter, onDecision, onOpen }: Props) {
  const filtered = useMemo(() => {
    return rows.filter((r) => {
      switch (filter) {
        case "needsYou":
          return r.class === "needsYou";
        case "bringIn":
          return r.class === "bringIn" || r.class === "moved";
        case "keep":
          return r.class === "keep";
        case "other":
          return (
            r.class === "intranetOnly" ||
            r.class === "internetOnly" ||
            r.class === "parkedSecret" ||
            r.class === "deletedBrought"
          );
        default:
          return r.class !== "identical";
      }
    });
  }, [rows, filter]);

  const parentRef = useRef<HTMLDivElement>(null);
  const virtualizer = useVirtualizer({
    count: filtered.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 64,
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
          return (
            <div
              key={row.id}
              className={`review-row class-${row.class}`}
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
                <span className="badge">{LABELS[row.class]}</span>
                <span className="path">{row.relPath}</span>
                {row.movedFrom && (
                  <span className="muted tiny">from {row.movedFrom}</span>
                )}
              </button>
              <div className="row-actions">
                <button
                  type="button"
                  className={row.decision === "bringIn" ? "chip on" : "chip"}
                  onClick={() => onDecision(row.id, "bringIn")}
                  disabled={
                    row.class === "intranetOnly" ||
                    row.class === "internetOnly" ||
                    row.class === "parkedSecret"
                  }
                >
                  Bring in
                </button>
                <button
                  type="button"
                  className={row.decision === "keep" ? "chip on" : "chip"}
                  onClick={() => onDecision(row.id, "keep")}
                >
                  Keep
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
