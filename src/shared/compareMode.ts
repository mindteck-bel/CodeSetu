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
  bringIntoSelected: string;
  keepSelected: string;
  applyButton: string;
  applySelectedButton: string;
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
  filterConflict: string;
  summaryBringIn: string;
  summaryKeep: string;
  summaryConflict: string;
  summaryDestOnly: string;
  summarySourceOnly: string;
  resultTitle: string;
  bringTitle: string;
  keepTitle: string;
  conflictTitle: string;
  addedSource: string;
  deletedSource: string;
  destOnly: string;
  sourceOnly: string;
  bringTooltip: string;
  deleteTooltip: string;
  keepTooltip: string;
  emptySelectionHint: string;
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
  reviewArrow: "Mine (Intranet) ← Source (Internet)",
  sideLegend:
    "Left = mine (Intranet on This PC) · Right = source (Internet, read-only). Conflicts need an explicit choice.",
  bringInto: "Use source",
  keep: "Keep mine",
  keepVersion: "Keep mine",
  bringIntoAll: "Use source (all conflicts)",
  keepAll: "Keep mine (all conflicts)",
  bringIntoSelected: "Use source (selected)",
  keepSelected: "Keep mine (selected)",
  applyButton: "Apply all",
  applySelectedButton: "Apply selected",
  confirmTitle: "Confirm apply to Intranet",
  confirmBody:
    "This will write to This PC (Intranet) only. The Internet folder is never modified.",
  deleteWarning:
    "Deletions remove files from Intranet (mine). This is never labeled “Use source.”",
  diffEyebrow: "Mine (left) vs source (right)",
  diffHint: "Use source takes the right side. Keep mine keeps the left.",
  diffLeftTitle: "Mine (Intranet)",
  diffRightTitle: "Source (Internet)",
  binaryHint:
    "Use source or Keep mine on the whole file — text hunks are not shown.",
  filterBringIn: "Use source",
  filterKeep: "Keep mine",
  filterConflict: "Conflict",
  summaryBringIn: "use source",
  summaryKeep: "keep mine",
  summaryConflict: "conflicts",
  summaryDestOnly: "intranet-only",
  summarySourceOnly: "internet-only",
  resultTitle: "Sync applied",
  bringTitle: "Use source",
  keepTitle: "Keep mine",
  conflictTitle: "Conflict",
  addedSource: "New on source",
  deletedSource: "Deleted on source",
  destOnly: "Intranet only",
  sourceOnly: "Internet only",
  bringTooltip: "Copy the Internet (source) version onto Intranet (mine)",
  deleteTooltip:
    "Accept the Internet deletion — remove this file from Intranet (mine)",
  keepTooltip: "Keep the Intranet (mine) version; ignore the source",
  emptySelectionHint:
    "No files selected — Apply all uses every decision. Check files in the tree to apply only those.",
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
  reviewArrow: "Mine (destination) ← Source",
  sideLegend:
    "Left = mine (destination on This PC) · Right = source (read-only). Conflicts need an explicit choice.",
  bringInto: "Use source",
  keep: "Keep mine",
  keepVersion: "Keep mine",
  bringIntoAll: "Use source (all conflicts)",
  keepAll: "Keep mine (all conflicts)",
  bringIntoSelected: "Use source (selected)",
  keepSelected: "Keep mine (selected)",
  applyButton: "Apply all",
  applySelectedButton: "Apply selected",
  confirmTitle: "Confirm apply to destination",
  confirmBody:
    "This will write to the destination folder on This PC only. The source folder is never modified.",
  deleteWarning:
    "Deletions remove files from the destination (mine). This is never labeled “Use source.”",
  diffEyebrow: "Mine (left) vs source (right)",
  diffHint: "Use source takes the right side. Keep mine keeps the left.",
  diffLeftTitle: "Mine (destination)",
  diffRightTitle: "Source",
  binaryHint:
    "Use source or Keep mine on the whole file — text hunks are not shown.",
  filterBringIn: "Use source",
  filterKeep: "Keep mine",
  filterConflict: "Conflict",
  summaryBringIn: "use source",
  summaryKeep: "keep mine",
  summaryConflict: "conflicts",
  summaryDestOnly: "destination-only",
  summarySourceOnly: "source-only",
  resultTitle: "Changes applied",
  bringTitle: "Use source",
  keepTitle: "Keep mine",
  conflictTitle: "Conflict",
  addedSource: "New on source",
  deletedSource: "Deleted on source",
  destOnly: "Destination only",
  sourceOnly: "Source only",
  bringTooltip: "Copy the source version onto the destination (mine)",
  deleteTooltip:
    "Accept the source deletion — remove this file from the destination (mine)",
  keepTooltip: "Keep the destination (mine) version; ignore the source",
  emptySelectionHint:
    "No files selected — Apply all uses every decision. Check files in the tree to apply only those.",
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
      return c.conflictTitle;
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
