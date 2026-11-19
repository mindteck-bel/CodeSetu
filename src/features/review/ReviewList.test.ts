import { describe, expect, it } from "vitest";
import { decisionLabelsForTest, filterRows } from "./ReviewList";
import { validateIgnorePatternLocal } from "../settings/IgnoreRulesSettings";
import type { FileRow } from "../../shared/types";

function row(
  partial: Partial<FileRow> & Pick<FileRow, "id" | "class">,
): FileRow {
  return {
    relPath: partial.relPath ?? "a.txt",
    group: "src",
    decision: partial.decision ?? "skip",
    zone: "shared",
    kind: "text",
    ...partial,
  };
}

describe("ReviewList decision labels", () => {
  it("never labels deletion as Use source", () => {
    const labels = decisionLabelsForTest(
      row({ id: "1", class: "deletedBrought", decision: "skip" }),
    );
    expect(labels.primary).toBe("Delete locally");
    expect(labels.keep).toBe("Keep mine");
    expect(labels.primary.toLowerCase()).not.toContain("use source");
    expect(labels.primary.toLowerCase()).not.toContain("bring");
  });

  it("uses Use source for normal bring-in rows", () => {
    const labels = decisionLabelsForTest(
      row({ id: "2", class: "bringIn", decision: "bringIn" }),
    );
    expect(labels.primary).toBe("Use source");
  });

  it("uses plain Keep mine in flexible mode", () => {
    const labels = decisionLabelsForTest(
      row({ id: "3", class: "needsYou", decision: "skip" }),
      "flexible",
    );
    expect(labels.primary).toBe("Use source");
    expect(labels.keep).toBe("Keep mine");
  });
});

describe("ReviewList filters", () => {
  const rows: FileRow[] = [
    row({ id: "1", class: "needsYou" }),
    row({ id: "2", class: "bringIn" }),
    row({ id: "3", class: "deletedBrought" }),
    row({ id: "4", class: "parkedSecret" }),
    row({ id: "5", class: "keep" }),
  ];

  it("isolates deletions from env/secret filter", () => {
    expect(filterRows(rows, "deletions").map((r) => r.id)).toEqual(["3"]);
    expect(filterRows(rows, "other").map((r) => r.id)).toEqual(["4"]);
    expect(filterRows(rows, "needsYou").map((r) => r.id)).toEqual(["1"]);
  });
});

describe("Ignore pattern validation", () => {
  it("rejects empty and negation", () => {
    expect(validateIgnorePatternLocal("")).toBeTruthy();
    expect(validateIgnorePatternLocal("!")).toBeTruthy();
    expect(validateIgnorePatternLocal("**/ok/**")).toBeNull();
  });
});
