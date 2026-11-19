import { invoke } from "@tauri-apps/api/core";
import type {
  ApplyPreview,
  ApplyReport,
  Decision,
  FileDiff,
  FileRow,
  IgnoreRulesView,
  ScanResult,
} from "./types";

export async function validateFolder(path: string): Promise<boolean> {
  return invoke("validate_folder", { path });
}

export async function startScan(
  thisPc: string,
  broughtOver: string,
): Promise<ScanResult> {
  return invoke("start_scan", { thisPc, broughtOver });
}

export async function cancelScan(sessionId: string): Promise<void> {
  return invoke("cancel_scan", { sessionId });
}

export async function updateDecisions(
  sessionId: string,
  updates: [string, Decision][],
): Promise<void> {
  if (sessionId.startsWith("demo-")) return;
  return invoke("update_decisions", { sessionId, updates });
}

export async function setAllNeedsYou(
  sessionId: string,
  decision: Decision,
  onlyIds?: string[] | null,
): Promise<FileRow[]> {
  if (sessionId.startsWith("demo-")) {
    return [];
  }
  return invoke("set_all_needs_you", {
    sessionId,
    decision,
    onlyIds: onlyIds?.length ? onlyIds : null,
  });
}

export async function getApplyPreview(
  sessionId: string,
  onlyIds?: string[] | null,
): Promise<ApplyPreview> {
  if (sessionId.startsWith("demo-")) {
    const n = onlyIds?.length ?? 0;
    return {
      creates: 0,
      overwrites: n || 2,
      deletes: 0,
      moves: 0,
      skips: 0,
      destructive: true,
      rowsToApply: n || 2,
      selectedCount: n || undefined,
    };
  }
  return invoke("get_apply_preview", {
    sessionId,
    onlyIds: onlyIds?.length ? onlyIds : null,
  });
}

export async function executeApply(
  sessionId: string,
  confirmDestructive: boolean,
  onlyIds?: string[] | null,
): Promise<ApplyReport> {
  if (sessionId.startsWith("demo-")) {
    const n = onlyIds?.length ?? 2;
    return {
      applied: n,
      skipped: 0,
      failed: 0,
      failures: [],
      message: "Demo apply (no filesystem writes)",
    };
  }
  return invoke("execute_apply", {
    sessionId,
    confirmDestructive,
    onlyIds: onlyIds?.length ? onlyIds : null,
  });
}

export async function undoApply(): Promise<ApplyReport> {
  return invoke("undo_apply");
}

export async function recoverJournals(): Promise<ApplyReport[]> {
  return invoke("recover_journals");
}

export async function getFileDiff(
  sessionId: string,
  relPath: string,
): Promise<FileDiff> {
  if (sessionId.startsWith("demo-")) {
    const { DEMO_DIFF } = await import("../features/review/reviewDemoData");
    return { ...DEMO_DIFF, relPath };
  }
  return invoke("get_file_diff", { sessionId, relPath });
}

export async function getIgnoreRules(
  projectPath?: string | null,
): Promise<IgnoreRulesView> {
  return invoke("get_ignore_rules", {
    projectPath: projectPath ?? null,
  });
}

export async function setIgnorePatternEnabled(
  id: string,
  enabled: boolean,
  projectPath?: string | null,
  projectScope = false,
): Promise<IgnoreRulesView> {
  return invoke("set_ignore_pattern_enabled", {
    id,
    enabled,
    projectPath: projectPath ?? null,
    projectScope,
  });
}

export async function setIgnoreCategoryEnabled(
  category: string,
  enabled: boolean,
  projectPath?: string | null,
  projectScope = false,
): Promise<IgnoreRulesView> {
  return invoke("set_ignore_category_enabled", {
    category,
    enabled,
    projectPath: projectPath ?? null,
    projectScope,
  });
}

export async function addIgnorePattern(
  pattern: string,
  projectPath?: string | null,
  projectScope = false,
): Promise<IgnoreRulesView> {
  return invoke("add_ignore_pattern", {
    pattern,
    projectPath: projectPath ?? null,
    projectScope,
  });
}

export async function updateIgnorePattern(
  id: string,
  pattern: string,
  projectPath?: string | null,
  projectScope = false,
): Promise<IgnoreRulesView> {
  return invoke("update_ignore_pattern", {
    id,
    pattern,
    projectPath: projectPath ?? null,
    projectScope,
  });
}

export async function deleteIgnorePattern(
  id: string,
  projectPath?: string | null,
  projectScope = false,
): Promise<IgnoreRulesView> {
  return invoke("delete_ignore_pattern", {
    id,
    projectPath: projectPath ?? null,
    projectScope,
  });
}

export async function exportIgnoreRules(
  projectPath?: string | null,
): Promise<string> {
  return invoke("export_ignore_rules", {
    projectPath: projectPath ?? null,
  });
}

export async function importIgnoreRules(
  json: string,
  projectPath?: string | null,
  asProject = false,
): Promise<IgnoreRulesView> {
  return invoke("import_ignore_rules", {
    json,
    projectPath: projectPath ?? null,
    asProject,
  });
}
