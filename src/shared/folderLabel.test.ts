import { describe, expect, it } from "vitest";
import {
  folderCaption,
  folderRoleLabel,
  normalizeFolderPath,
  truncateMiddle,
} from "./folderLabel";

describe("folderCaption", () => {
  it("uses basename and truncated parent for Windows paths", () => {
    const c = folderCaption("C:\\Users\\naveen\\projects\\intranet");
    expect(c.name).toBe("intranet");
    expect(c.pathHint).toContain("Users");
    expect(c.full).toContain("intranet");
  });

  it("handles empty path", () => {
    expect(folderCaption("").name).toBe("(no folder)");
  });
});

describe("truncateMiddle", () => {
  it("leaves short strings alone", () => {
    expect(truncateMiddle("abc", 10)).toBe("abc");
  });

  it("ellipsizes long strings", () => {
    const out = truncateMiddle("abcdefghijklmnopqrstuvwxyz", 11);
    expect(out).toContain("…");
    expect(out.length).toBeLessThanOrEqual(11);
  });
});

describe("normalizeFolderPath", () => {
  it("converts backslashes and trims trailing slash", () => {
    expect(normalizeFolderPath("C:\\a\\b\\")).toBe("C:/a/b");
  });
});

describe("folderRoleLabel", () => {
  it("keeps intranet/internet roles in airgap mode", () => {
    expect(folderRoleLabel("destination", "airgap")).toMatch(/Intranet/);
    expect(folderRoleLabel("source", "airgap")).toMatch(/Internet/);
  });
});
