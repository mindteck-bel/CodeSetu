/** Map raw IPC / engine errors into short recovery-oriented copy for pilots. */

export function formatUserError(raw: unknown): string {
  const message = raw instanceof Error ? raw.message : String(raw ?? "Unknown error");
  const lower = message.toLowerCase();

  if (lower.includes("session expired") || lower.includes("run a new scan")) {
    return `${message} Recovery: go back to Folders and scan again.`;
  }
  if (lower.includes("ignore rules changed") || lower.includes("fresh compare")) {
    return `${message} Recovery: rescan before Apply.`;
  }
  if (lower.includes("nothing to undo")) {
    return `${message} There is no committed apply journal left to reverse.`;
  }
  if (lower.includes("confirm") && lower.includes("destructive")) {
    return `${message} Recovery: open Apply again and confirm the destructive action.`;
  }
  if (lower.includes("first sync")) {
    return `${message} Recovery: review Creates carefully, then confirm first sync.`;
  }
  if (
    lower.includes("permission") ||
    lower.includes("access is denied") ||
    lower.includes("os error 5")
  ) {
    return `${message} Recovery: check folder permissions or close locks held by other apps.`;
  }
  if (lower.includes("does not exist") || lower.includes("cannot find")) {
    return `${message} Recovery: re-select both folders; paths may have moved or become unavailable.`;
  }
  if (lower.includes("disk") || lower.includes("space") || lower.includes("no space")) {
    return `${message} Recovery: free disk space on the destination drive, then retry.`;
  }
  return message;
}

/** One-line next step when an apply partially failed. */
export function failureRecoveryHint(failed: number, hasJournal: boolean): string {
  if (failed <= 0) return "";
  if (hasJournal) {
    return "Some files failed while others may have applied. Use Undo last apply to restore the previous tree, then rescan. Export diagnostics if you need to share details with the pilot contact.";
  }
  return "Some files failed. Rescan to see current state. Export diagnostics if the failure repeats.";
}
