import { describe, expect, it } from "vitest";
import { alignSplitHunks, gapPlaceholder } from "./alignedDiff";

describe("alignSplitHunks", () => {
  it("keeps equal hunks on both sides", () => {
    const rows = alignSplitHunks([
      { tag: "equal", thisText: "a\n", broughtText: "a\n" },
    ]);
    expect(rows).toHaveLength(1);
    expect(rows[0]?.sourceText).toBe("a\n");
    expect(rows[0]?.destText).toBe("a\n");
    expect(rows[0]?.kind).toBe("equal");
  });

  it("puts inserts on source with a destination gap", () => {
    const rows = alignSplitHunks([
      { tag: "insert", thisText: "", broughtText: "new\n" },
    ]);
    expect(rows[0]?.sourceText).toBe("new\n");
    expect(rows[0]?.destText).toBeNull();
    expect(rows[0]?.kind).toBe("insert");
  });

  it("puts deletes on destination with a source gap", () => {
    const rows = alignSplitHunks([
      { tag: "delete", thisText: "old\n", broughtText: "" },
    ]);
    expect(rows[0]?.sourceText).toBeNull();
    expect(rows[0]?.destText).toBe("old\n");
    expect(rows[0]?.kind).toBe("delete");
  });
});

describe("gapPlaceholder", () => {
  it("matches single-line height", () => {
    expect(gapPlaceholder("x\n")).toBe("\n");
  });

  it("matches multi-line height", () => {
    expect(gapPlaceholder("a\nb\nc\n")).toBe("\n\n\n");
  });
});
