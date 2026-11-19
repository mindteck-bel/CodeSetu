import type { ChangeClass, FileRow } from "./types";

/** How the user is pairing folders on this PC. */
export type CompareMode = "airgap" | "flexible";

/** Side that Apply may write. Always destination / This PC. */
export type ApplyTargetSide = "destination";

export const COMPARE_STORAGE_KEY = "codesetu-compare-prefs";

export type ComparePrefs = {
  mode: CompareMode;
  thisPc: string;
  broughtOver: string;
};

export const DEFAULT_COMPARE_PREFS: ComparePrefs = {
  mode: "airgap",
  thisPc: "",
  broughtOver: "",
};

export function isCompareMode(value: unknown): value is CompareMode {
  return value === "airgap" || value === "flexible";
}

export function readComparePrefs(): ComparePrefs {
  try {
    const raw = localStorage.getItem(COMPARE_STORAGE_KEY);
    if (!raw) return { ...DEFAULT_COMPARE_PREFS };
    const parsed = JSON.parse(raw) as Partial<ComparePrefs>;
    return {
      mode: isCompareMode(parsed.mode) ? parsed.mode : "airgap",
      thisPc: typeof parsed.thisPc === "string" ? parsed.thisPc : "",
      broughtOver:
        typeof parsed.broughtOver === "string" ? parsed.broughtOver : "",
    };
  } catch {
    return { ...DEFAULT_COMPARE_PREFS };
  }
}

export function writeComparePrefs(prefs: ComparePrefs): void {
  try {
    localStorage.setItem(COMPARE_STORAGE_KEY, JSON.stringify(prefs));
  } catch {
    /* ignore quota / private mode */
  }
}

/** Apply always mutates only the destination folder (This PC / left). */
export function applyTargetSide(_mode: CompareMode): ApplyTargetSide {
  return "destination";
}

export function applyWritesOnlyDestination(mode: CompareMode): boolean {
  return applyTargetSide(mode) === "destination";
}

export type ModeCopy = {
  tagline: string;
  pickTitle: string;
  pickLede: string;
  destinationEyebrow: string;
  sourceEyebrow: string;
  chooseDestination: string;
  chooseSource: string;
  dialogDestination: string;
  dialogSource: string;
  reviewArrow: string;
  sideLegend: string;
  bringInto: string;
  keep: string;
  keepVersion: string;
  bringIntoAll: string;
  keepAll: string;
  applyButton: string;
  confirmTitle: string;
  confirmBody: string;
  deleteWarning: string;
  diffEyebrow: string;
  diffHint: string;
  diffLeftTitle: string;
  diffRightTitle: string;
  binaryHint: string;
  filterBringIn: string;
  filterKeep: string;
  summaryBringIn: string;
  summaryKeep: string;
  summaryDestOnly: string;
  summarySourceOnly: string;
  resultTitle: string;
  bringTitle: string;
  keepTitle: string;
  addedSource: string;
  deletedSource: string;
  destOnly: string;
  sourceOnly: string;
  bringTooltip: string;
  deleteTooltip: string;
};

const AIRGAP: ModeCopy = {
  tagline: "Bridge Intranet ↔ Internet code",
  pickTitle: "Pick the two folders",
  pickLede:
    "Only the Intranet folder on This PC will be updated. The Internet / brought-over copy stays read-only.",
  destinationEyebrow: "Intranet · This PC · will change",
  sourceEyebrow: "Internet · brought over · read only",
  chooseDestination: "Choose Intranet folder",
  chooseSource: "Choose Internet folder",
  dialogDestination: "Choose Intranet folder on This PC (will be updated)",
  dialogSource: "Choose Internet / brought-over folder (read only)",
  reviewArrow: "Intranet ← Internet",
  sideLegend:
    "Left = Intranet (This PC, writable) · Right = Internet (read-only). Conflicts need an explicit choice — neither side is taken automatically.",
  bringInto: "Bring into Intranet",
  keep: "Keep Intranet",
  keepVersion: "Keep Intranet version",
  bringIntoAll: "Bring into Intranet (all conflicts)",
  keepAll: "Keep Intranet (all conflicts)",
  applyButton: "Apply to Intranet",
  confirmTitle: "Confirm apply to Intranet",
  confirmBody:
    "This will write to This PC (Intranet) only. The Internet folder is never modified.",
  deleteWarning:
    "Deletions remove files from Intranet. This is never labeled “Bring into Intranet.”",
  diffEyebrow: "Intranet (left) vs Internet (right)",
  diffHint:
    "Use “Bring into Intranet” to take the right side, or “Keep Intranet version” to keep the left.",
  diffLeftTitle: "Intranet (This PC)",
  diffRightTitle: "Internet (brought over)",
  binaryHint:
    "Use Bring into Intranet or Keep Intranet version on the whole file — text hunks are not shown.",
  filterBringIn: "Bring into Intranet",
  filterKeep: "Keep Intranet",
  summaryBringIn: "bring into Intranet",
  summaryKeep: "keep Intranet",
  summaryDestOnly: "intranet-only",
  summarySourceOnly: "internet-only",
  resultTitle: "Sync applied",
  bringTitle: "Bring into Intranet",
  keepTitle: "Keep Intranet",
  addedSource: "New on Internet",
  deletedSource: "Deleted on Internet",
  destOnly: "Intranet only",
  sourceOnly: "Internet only",
  bringTooltip: "Copy the Internet version onto Intranet (This PC)",
  deleteTooltip:
    "Accept the Internet deletion — remove this file from Intranet (This PC)",
};

const FLEXIBLE: ModeCopy = {
  tagline: "Compare any two local folders",
  pickTitle: "Pick the two folders",
  pickLede:
    "Only the destination folder on This PC will be updated. The source folder stays read-only.",
  destinationEyebrow: "Destination · This PC · will change",
  sourceEyebrow: "Source · compare from · read only",
  chooseDestination: "Choose destination folder",
  chooseSource: "Choose source folder",
  dialogDestination: "Choose destination folder on This PC (will be updated)",
  dialogSource: "Choose source folder (read only)",
  reviewArrow: "Destination ← Source",
  sideLegend:
    "Left = Destination (This PC, writable) · Right = Source (read-only). Conflicts need an explicit choice — neither side is taken automatically.",
  bringInto: "Bring into destination",
  keep: "Keep destination",
  keepVersion: "Keep destination version",
  bringIntoAll: "Bring into destination (all conflicts)",
  keepAll: "Keep destination (all conflicts)",
  applyButton: "Apply to This PC",
  confirmTitle: "Confirm apply to destination",
  confirmBody:
    "This will write to the destination folder on This PC only. The source folder is never modified.",
  deleteWarning:
    "Deletions remove files from the destination. This is never labeled “Bring into destination.”",
  diffEyebrow: "Destination (left) vs Source (right)",
  diffHint:
    "Use “Bring into destination” to take the right side, or “Keep destination version” to keep the left.",
  diffLeftTitle: "Destination (This PC)",
  diffRightTitle: "Source (read only)",
  binaryHint:
    "Use Bring into destination or Keep destination version on the whole file — text hunks are not shown.",
  filterBringIn: "Bring into destination",
  filterKeep: "Keep destination",
  summaryBringIn: "bring into destination",
  summaryKeep: "keep destination",
  summaryDestOnly: "destination-only",
  summarySourceOnly: "source-only",
  resultTitle: "Changes applied",
  bringTitle: "Bring into destination",
  keepTitle: "Keep destination",
  addedSource: "New on source",
  deletedSource: "Deleted on source",
  destOnly: "Destination only",
  sourceOnly: "Source only",
  bringTooltip: "Copy the source version onto the destination (This PC)",
  deleteTooltip:
    "Accept the source deletion — remove this file from the destination (This PC)",
};

export function modeCopy(mode: CompareMode): ModeCopy {
  return mode === "flexible" ? FLEXIBLE : AIRGAP;
}

export function changeClassLabel(
  mode: CompareMode,
  changeClass: ChangeClass,
): string {
  const c = modeCopy(mode);
  switch (changeClass) {
    case "identical":
      return "Same";
    case "bringIn":
      return c.bringTitle;
    case "keep":
      return c.keepTitle;
    case "needsYou":
      return "Resolve conflict";
    case "addedBrought":
      return c.addedSource;
    case "deletedBrought":
      return c.deletedSource;
    case "moved":
      return "Moved";
    case "intranetOnly":
      return c.destOnly;
    case "internetOnly":
      return c.sourceOnly;
    case "parkedSecret":
      return "Secret / env";
    default:
      return changeClass;
  }
}

export function primaryActionLabel(mode: CompareMode, row: FileRow): string {
  if (row.class === "deletedBrought") return "Delete locally";
  return modeCopy(mode).bringInto;
}

export function keepActionLabel(mode: CompareMode, _row: FileRow): string {
  return modeCopy(mode).keepVersion;
}

export const MODE_OPTIONS: {
  id: CompareMode;
  title: string;
  description: string;
}[] = [
  {
    id: "airgap",
    title: "Air-gap sync (Intranet / Internet)",
    description:
      "Classic workflow: bring changes from the Internet copy into Intranet on This PC.",
  },
  {
    id: "flexible",
    title: "Compare any two folders",
    description:
      "Same-PC compare of any two local trees. Apply still writes only the destination.",
  },
];
