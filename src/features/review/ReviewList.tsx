import type { Decision, FileRow, ReviewFilter } from "../../shared/types";
import {
  keepActionLabel,
  primaryActionLabel,
  type CompareMode,
} from "../../shared/compareMode";

/** Kept for tests and shared filtering used by the Review tree. */
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

// Decision type re-export keeps older imports compiling in tests.
export type { Decision };
