import { describe, expect, it } from "vitest";
import { failureRecoveryHint, formatUserError } from "./userError";

describe("formatUserError", () => {
  it("adds recovery for expired sessions", () => {
    expect(formatUserError("Session expired. Run a new scan.")).toMatch(/Folders/);
  });

  it("passes through plain messages", () => {
    expect(formatUserError("Boom")).toBe("Boom");
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
