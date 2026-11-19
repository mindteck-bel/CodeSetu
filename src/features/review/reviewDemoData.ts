import type { FileRow, ScanResult } from "../../shared/types";

function row(
  partial: Partial<FileRow> & Pick<FileRow, "id" | "relPath" | "class">,
): FileRow {
  return {
    group: partial.relPath.split(/[/\\]/)[0] ?? "root",
    decision: partial.decision ?? "skip",
    zone: "shared",
    kind: "text",
    error:
      partial.class === "needsYou"
        ? "Both sides differ and there is no prior baseline — choose which version to keep."
        : undefined,
    ...partial,
  };
}

/** Fixture used by `#review-demo` hash for local UI walkthroughs (no Tauri). */
export function buildReviewDemo(): { result: ScanResult; rows: FileRow[] } {
  const rows: FileRow[] = [
    row({ id: "1", relPath: ".gitignore", class: "needsYou" }),
    row({ id: "2", relPath: "package.json", class: "needsYou" }),
    row({
      id: "3",
      relPath: "src/Navbar/Dropdown.jsx",
      class: "needsYou",
    }),
    row({
      id: "4",
      relPath: "src/Navbar/Navbar.jsx",
      class: "needsYou",
    }),
    row({
      id: "5",
      relPath: "src/Redux/Actions/AllotDC/AllotDcAction.js",
      class: "needsYou",
      decision: "bringIn",
    }),
    row({
      id: "6",
      relPath: "src/Redux/Actions/AllotDC/AllotDcActionTypes.js",
      class: "needsYou",
    }),
    row({
      id: "7",
      relPath: "src/utils/format.js",
      class: "bringIn",
      decision: "bringIn",
    }),
    row({
      id: "8",
      relPath: "README.md",
      class: "keep",
      decision: "keep",
    }),
  ];

  const result: ScanResult = {
    sessionId: "demo-review-session",
    thisPc: "C:/demo/intranet",
    broughtOver: "C:/demo/internet",
    summary: {
      totalThis: 120,
      totalBrought: 118,
      hashed: 200,
      identical: 90,
      bringIn: 1,
      keep: 1,
      needsYou: 6,
      intranetOnly: 0,
      internetOnly: 0,
      parkedSecret: 0,
      deletedBrought: 0,
      moved: 0,
      errors: 0,
      elapsedMs: 1820,
      firstSync: false,
    },
    rows,
    warnings: [],
  };

  return { result, rows };
}

export const DEMO_DIFF = {
  relPath: "package.json",
  kind: "text" as const,
  thisExists: true,
  broughtExists: true,
  truncated: false,
  hunks: [
    {
      tag: "equal",
      thisText: '{\n  "name": "codesetu",\n',
      broughtText: '{\n  "name": "codesetu",\n',
    },
    {
      tag: "delete",
      thisText: '  "version": "0.1.0",\n',
      broughtText: "",
    },
    {
      tag: "insert",
      thisText: "",
      broughtText: '  "version": "0.2.0",\n',
    },
    {
      tag: "equal",
      thisText: '  "private": true\n}\n',
      broughtText: '  "private": true\n}\n',
    },
  ],
};
