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
  it("keeps Intranet terminology in airgap mode", () => {
    const copy = modeCopy("airgap");
    expect(copy.bringInto).toBe("Bring into Intranet");
    expect(copy.applyButton).toBe("Apply to Intranet");
    expect(copy.destinationEyebrow).toContain("Intranet");
    expect(copy.sourceEyebrow).toContain("Internet");
    expect(changeClassLabel("airgap", "internetOnly")).toBe("Internet only");
  });

  it("uses destination/source labels in flexible mode", () => {
    const copy = modeCopy("flexible");
    expect(copy.bringInto).toBe("Bring into destination");
    expect(copy.applyButton).toBe("Apply to This PC");
    expect(copy.destinationEyebrow).toContain("Destination");
    expect(copy.sourceEyebrow).toContain("Source");
    expect(changeClassLabel("flexible", "addedBrought")).toBe("New on source");
    expect(changeClassLabel("flexible", "intranetOnly")).toBe(
      "Destination only",
    );
  });

  it("never labels deletions as bring-in in either mode", () => {
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
      expect(labels.keep.toLowerCase()).toContain(
        mode === "airgap" ? "intranet" : "destination",
      );
    }
  });

  it("maps bring-in action labels by mode", () => {
    expect(
      primaryActionLabel("airgap", row({ id: "2", class: "bringIn" })),
    ).toBe("Bring into Intranet");
    expect(
      primaryActionLabel("flexible", row({ id: "2", class: "bringIn" })),
    ).toBe("Bring into destination");
  });
});
