import type { DiffHunk } from "../../shared/types";

/** One aligned row in a Source → Destination split diff. */
export type AlignedDiffRow = {
  key: string;
  /** Source (left) text; null means a gap spacer matching the other side's height. */
  sourceText: string | null;
  /** Destination / mine (right) text; null means a gap spacer. */
  destText: string | null;
  /** Visual class for the non-gap side(s). */
  kind: "equal" | "insert" | "delete" | "replace";
};

function lineCount(text: string): number {
  if (!text) return 1;
  const endsWithNl = text.endsWith("\n");
  const n = text.split("\n").length;
  return endsWithNl ? Math.max(1, n - 1) : Math.max(1, n);
}

/** Gap placeholder with the same line count as `text` so columns stay aligned. */
export function gapPlaceholder(text: string): string {
  const n = lineCount(text);
  return n <= 1 ? "\n" : `${"\n".repeat(n - 1)}\n`;
}

/**
 * Build Source (left) → Destination (right) rows with gaps where only one side changed.
 * Engine hunk tags: insert = source-only, delete = destination-only, equal = both.
 */
export function alignSplitHunks(hunks: DiffHunk[]): AlignedDiffRow[] {
  const rows: AlignedDiffRow[] = [];
  hunks.forEach((h, i) => {
    const tag = h.tag;
    if (tag === "equal") {
      rows.push({
        key: `e-${i}`,
        sourceText: h.broughtText || h.thisText,
        destText: h.thisText || h.broughtText,
        kind: "equal",
      });
      return;
    }
    if (tag === "insert") {
      rows.push({
        key: `i-${i}`,
        sourceText: h.broughtText,
        destText: null,
        kind: "insert",
      });
      return;
    }
    if (tag === "delete") {
      rows.push({
        key: `d-${i}`,
        sourceText: null,
        destText: h.thisText,
        kind: "delete",
      });
      return;
    }
    // replace / other — show both when present
    rows.push({
      key: `r-${i}`,
      sourceText: h.broughtText || null,
      destText: h.thisText || null,
      kind: "replace",
    });
  });
  return rows;
}
