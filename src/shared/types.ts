export type Decision = "bringIn" | "keep" | "skip";

export type ChangeClass =
  | "identical"
  | "bringIn"
  | "keep"
  | "needsYou"
  | "addedBrought"
  | "deletedBrought"
  | "moved"
  | "intranetOnly"
  | "internetOnly"
  | "parkedSecret";

export type FileKind = "text" | "binary" | "large";

export interface FileRow {
  id: string;
  relPath: string;
  group: string;
  class: ChangeClass;
  decision: Decision;
  zone: string;
  kind: FileKind;
  thisSize?: number;
  broughtSize?: number;
  thisHash?: string;
  broughtHash?: string;
  movedFrom?: string;
  error?: string;
}

export interface ScanSummary {
  totalThis: number;
  totalBrought: number;
  hashed: number;
  identical: number;
  bringIn: number;
  keep: number;
  needsYou: number;
  intranetOnly: number;
  internetOnly: number;
  parkedSecret: number;
  deletedBrought: number;
  moved: number;
  errors: number;
  elapsedMs: number;
  firstSync?: boolean;
}

export interface ScanResult {
  sessionId: string;
  thisPc: string;
  broughtOver: string;
  summary: ScanSummary;
  rows: FileRow[];
  warnings: string[];
}

export interface ProgressEvent {
  sessionId: string;
  phase: string;
  filesSeen: number;
  filesHashed: number;
  currentPath: string;
  message: string;
}

export interface DiffHunk {
  tag: string;
  thisText: string;
  broughtText: string;
}

export interface FileDiff {
  relPath: string;
  kind: FileKind;
  thisExists: boolean;
  broughtExists: boolean;
  hunks: DiffHunk[];
  truncated: boolean;
  error?: string;
}

export interface ApplyReport {
  applied: number;
  /** Intentional Keep mine — destination bytes unchanged; baseline updated. */
  kept?: number;
  skipped: number;
  failed: number;
  failures: string[];
  journalId?: string;
  creates?: number;
  overwrites?: number;
  deletes?: number;
  recovered?: boolean;
  message?: string;
}

export interface ApplyPreview {
  creates: number;
  overwrites: number;
  deletes: number;
  moves: number;
  /** Intentional Keep mine — destination files stay as-is. */
  keeps?: number;
  skips: number;
  destructive: boolean;
  rowsToApply: number;
  firstSync?: boolean;
  /** Present when preview/apply is limited to a checkbox selection. */
  selectedCount?: number;
}

export interface IgnorePattern {
  id: string;
  pattern: string;
  category: string;
  enabled: boolean;
  builtin: boolean;
  source: string;
}

export interface CategoryInfo {
  id: string;
  label: string;
  explanation: string;
  enabled: boolean;
  activeCount: number;
  totalCount: number;
}

export interface IgnoreRulesView {
  version: number;
  patterns: IgnorePattern[];
  categoryEnabled: Record<string, boolean>;
  categories: CategoryInfo[];
  effectivePatterns: string[];
  projectPath?: string;
  warnings: string[];
}

export type Step = "pick" | "scan" | "review" | "result" | "settings";
export type ReviewFilter =
  | "needsYou"
  | "bringIn"
  | "keep"
  | "deletions"
  | "other"
  | "all";
