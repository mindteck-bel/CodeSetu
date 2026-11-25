/** @vitest-environment jsdom */
import { afterEach, describe, expect, it } from "vitest";
import {
  applyTargetSide,
  applyWritesOnlyDestination,
  changeClassLabel,
  COMPARE_STORAGE_KEY,
  keepActionLabel,
  modeCopy,
  primaryActionLabel,
  readComparePrefs,
  writeComparePrefs,
  type CompareMode,
} from "./compareMode";
import type { FileRow } from "./types";

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

afterEach(() => {
  try {
    localStorage.removeItem(COMPARE_STORAGE_KEY);
  } catch {
    /* node env without localStorage */
  }
});

describe("compare mode persistence", () => {
  it("defaults to airgap with empty paths", () => {
    const prefs = readComparePrefs();
    expect(prefs.mode).toBe("airgap");
    expect(prefs.thisPc).toBe("");
    expect(prefs.broughtOver).toBe("");
  });

  it("round-trips mode and paths", () => {
    writeComparePrefs({
      mode: "flexible",
      thisPc: "C:/dest",
      broughtOver: "C:/src",
    });
    expect(readComparePrefs()).toEqual({
      mode: "flexible",
      thisPc: "C:/dest",
      broughtOver: "C:/src",
    });
  });

  it("rejects unknown mode values", () => {
    localStorage.setItem(
      COMPARE_STORAGE_KEY,
      JSON.stringify({ mode: "weird", thisPc: "a", broughtOver: "b" }),
    );
    expect(readComparePrefs().mode).toBe("airgap");
  });
});

describe("apply target safety", () => {
  const modes: CompareMode[] = ["airgap", "flexible"];

  it("always applies only to destination for both modes", () => {
    for (const mode of modes) {
      expect(applyTargetSide(mode)).toBe("destination");
      expect(applyWritesOnlyDestination(mode)).toBe(true);
    }
  });
});

describe("label mapping", () => {
  it("uses plain Review language in airgap mode", () => {
    const copy = modeCopy("airgap");
    expect(copy.bringInto).toBe("Use source");
    expect(copy.keepVersion).toBe("Keep mine");
    expect(copy.filterConflict).toBe("Conflict");
    expect(copy.applySelectedButton).toBe("Apply selected");
    expect(copy.applyButton).toBe("Apply all");
    expect(copy.destinationEyebrow).toContain("Intranet");
    expect(copy.sourceEyebrow).toContain("Internet");
    expect(copy.sourceEyebrow.toLowerCase()).not.toContain("brought");
    expect(copy.pickLede.toLowerCase()).not.toContain("brought");
    expect(copy.dialogSource.toLowerCase()).not.toContain("brought");
    expect(changeClassLabel("airgap", "needsYou")).toBe("Conflict");
    expect(changeClassLabel("airgap", "internetOnly")).toBe("Internet only");
  });

  it("uses destination/source labels in flexible mode", () => {
    const copy = modeCopy("flexible");
    expect(copy.bringInto).toBe("Use source");
    expect(copy.keep).toBe("Keep mine");
    expect(copy.applyButton).toBe("Apply all");
    expect(copy.destinationEyebrow).toContain("Destination");
    expect(copy.sourceEyebrow).toContain("Source");
    expect(copy.reviewArrow).toContain("→");
    expect(copy.diffLeftTitle.toLowerCase()).toContain("source");
    expect(copy.diffRightTitle.toLowerCase()).toContain("mine");
    expect(changeClassLabel("flexible", "addedBrought")).toBe("New on source");
    expect(changeClassLabel("flexible", "intranetOnly")).toBe(
      "Destination only",
    );
  });

  it("result copy stays local-folder only (no Git push/pull)", () => {
    for (const mode of ["airgap", "flexible"] as const) {
      const copy = modeCopy(mode);
      const blob = [
        copy.resultTitle,
        copy.resultKeptTitle,
        copy.resultNothingTitle,
        copy.resultBody,
        copy.resultBodyNothing,
      ].join(" ");
      expect(blob.toLowerCase()).not.toContain("git");
      expect(blob.toLowerCase()).not.toContain("push");
      expect(blob.toLowerCase()).not.toContain("commit");
      expect(blob.toLowerCase()).not.toContain("remote");
      expect(copy.resultBody.toLowerCase()).toMatch(/folder|intranet|destination/);
      expect(copy.resultNothingTitle.toLowerCase()).toContain("nothing");
    }
  });

  it("never labels deletions as use-source in either mode", () => {
    for (const mode of ["airgap", "flexible"] as const) {
      const labels = {
        primary: primaryActionLabel(
          mode,
          row({ id: "1", class: "deletedBrought" }),
        ),
        keep: keepActionLabel(mode, row({ id: "1", class: "deletedBrought" })),
      };
      expect(labels.primary).toBe("Delete locally");
      expect(labels.primary.toLowerCase()).not.toContain("bring");
      expect(labels.primary.toLowerCase()).not.toContain("use source");
      expect(labels.keep).toBe("Keep mine");
    }
  });

  it("maps bring-in action labels to Use source", () => {
    expect(
      primaryActionLabel("airgap", row({ id: "2", class: "bringIn" })),
    ).toBe("Use source");
    expect(
      primaryActionLabel("flexible", row({ id: "2", class: "bringIn" })),
    ).toBe("Use source");
  });
});
