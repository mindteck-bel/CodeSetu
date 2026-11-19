import { describe, expect, it } from "vitest";
import type { FileRow } from "../../shared/types";
import { buildFileTree, collectFileIds } from "./fileTree";

function row(
  partial: Partial<FileRow> & Pick<FileRow, "id" | "relPath" | "class">,
): FileRow {
  return {
    group: "src",
    decision: "skip",
    zone: "shared",
    kind: "text",
    ...partial,
  };
}

describe("buildFileTree", () => {
  it("only includes changed files and their ancestor folders", () => {
    const tree = buildFileTree([
      row({ id: "1", relPath: "src/a.ts", class: "needsYou" }),
      row({ id: "2", relPath: "src/nested/b.ts", class: "bringIn" }),
      row({ id: "3", relPath: "readme.md", class: "keep" }),
      row({ id: "4", relPath: "src/same.ts", class: "identical" }),
    ]);
    expect(tree.map((n) => n.name).sort()).toEqual(["readme.md", "src"]);
    const src = tree.find((n) => n.name === "src");
    expect(src?.children?.map((c) => c.name).sort()).toEqual([
      "a.ts",
      "nested",
    ]);
    const nested = src?.children?.find((c) => c.name === "nested");
    expect(nested?.children?.map((c) => c.name)).toEqual(["b.ts"]);
    expect(collectFileIds(src!).sort()).toEqual(["1", "2"]);
  });

  it("never lists unchanged siblings", () => {
    const tree = buildFileTree([
      row({ id: "1", relPath: "pkg/changed.js", class: "needsYou" }),
    ]);
    expect(JSON.stringify(tree)).not.toContain("unchanged");
    expect(tree).toHaveLength(1);
    expect(tree[0].name).toBe("pkg");
    expect(tree[0].children).toHaveLength(1);
    expect(tree[0].children![0].name).toBe("changed.js");
  });
});
