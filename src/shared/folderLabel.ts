/** Display helpers for destination / source folder paths in Review and Apply. */

export type FolderCaption = {
  /** Last path segment (folder name). */
  name: string;
  /** Truncated parent path for secondary display. */
  pathHint: string;
  /** Full path for tooltips / title attributes. */
  full: string;
};

export function normalizeFolderPath(path: string): string {
  return path.replace(/\\/g, "/").replace(/\/+$/, "");
}

export function truncateMiddle(value: string, max = 42): string {
  if (value.length <= max) return value;
  const keep = Math.max(4, Math.floor((max - 1) / 2));
  return `${value.slice(0, keep)}…${value.slice(-keep)}`;
}

export function folderCaption(path: string): FolderCaption {
  const full = path.trim();
  if (!full) {
    return { name: "(no folder)", pathHint: "", full: "" };
  }
  const normalized = normalizeFolderPath(full);
  const parts = normalized.split("/").filter(Boolean);
  const name = parts.length > 0 ? (parts[parts.length - 1] ?? full) : full;
  const parent = parts.length > 1 ? parts.slice(0, -1).join("/") : "";
  // Keep drive letter readable on Windows (C:/Users/…)
  const pathHint = parent ? truncateMiddle(parent, 40) : "";
  return { name, pathHint, full };
}

export function folderRoleLabel(
  side: "destination" | "source",
  mode: "airgap" | "flexible",
): string {
  if (side === "destination") {
    return mode === "airgap"
      ? "Destination · Intranet · mine"
      : "Destination · mine";
  }
  return mode === "airgap" ? "Source · Internet · read only" : "Source · read only";
}
