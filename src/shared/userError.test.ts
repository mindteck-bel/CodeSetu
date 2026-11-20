import { describe, expect, it } from "vitest";
import { failureRecoveryHint, formatUserError } from "./userError";

describe("formatUserError", () => {
  it("adds recovery for expired sessions", () => {
    expect(formatUserError("Session expired. Run a new scan.")).toMatch(/Folders/);
  });

  it("passes through plain messages", () => {
    expect(formatUserError("Boom")).toBe("Boom");
  });

  it("suggests re-selecting folders when a path is missing", () => {
    const out = formatUserError("Path does not exist");
    expect(out).toMatch(/re-select both folders/i);
    expect(out.toLowerCase()).not.toContain("usb");
  });
});

describe("failureRecoveryHint", () => {
  it("mentions undo when a journal exists", () => {
    expect(failureRecoveryHint(2, true)).toMatch(/Undo/);
  });

  it("is empty when nothing failed", () => {
    expect(failureRecoveryHint(0, true)).toBe("");
  });
});
