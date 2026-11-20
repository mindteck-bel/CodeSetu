import { invoke } from "@tauri-apps/api/core";
import type {
  ApplyPreview,
  ApplyReport,
  Decision,
  FileDiff,
  FileRow,
  HistoryEntry,
  IgnoreRulesView,
  ProjectProfile,
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
      keeps: 0,
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
      kept: 0,
      skipped: 0,
      failed: 0,
      failures: [],
      message:
        n === 1
          ? "1 file updated on destination (demo — no filesystem writes)"
          : `${n} files updated on destination (demo — no filesystem writes)`,
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

function browserDemoShell(): boolean {
  return (
    typeof window !== "undefined" &&
    !(window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__
  );
}

const DEMO_IGNORE_VIEW: IgnoreRulesView = {
  version: 1,
  categories: [
    {
      id: "vcs",
      label: "Version control",
      explanation: "Git / VCS metadata",
      enabled: true,
      activeCount: 3,
      totalCount: 3,
    },
    {
      id: "deps",
      label: "Dependencies",
      explanation: "node_modules and package caches",
      enabled: true,
      activeCount: 4,
      totalCount: 4,
    },
    {
      id: "build",
      label: "Build outputs",
      explanation: "dist, target, build folders",
      enabled: true,
      activeCount: 5,
      totalCount: 5,
    },
    {
      id: "ide",
      label: "IDE / editor",
      explanation: "Editor config and caches",
      enabled: true,
      activeCount: 3,
      totalCount: 3,
    },
  ],
  patterns: [],
  categoryEnabled: {
    vcs: true,
    deps: true,
    build: true,
    ide: true,
  },
  effectivePatterns: ["**/.git/**", "**/node_modules/**", "**/dist/**"],
  warnings: [],
};

export async function getIgnoreRules(
  projectPath?: string | null,
): Promise<IgnoreRulesView> {
  if (browserDemoShell()) {
    return DEMO_IGNORE_VIEW;
  }
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

export async function exportDiagnostics(destPath: string): Promise<string> {
  return invoke("export_diagnostics", { destPath });
}

export async function getSyncHistory(limit = 40): Promise<HistoryEntry[]> {
  if (browserDemoShell()) {
    return [
      {
        ts: "2026-11-20T20:48:02.000Z",
        kind: "apply",
        applied: 1,
        kept: 0,
        skipped: 0,
        failed: 0,
        deletes: 0,
        journalId: "3c1acd51-demo",
        message: "1 file updated on destination",
      },
      {
        ts: "2026-11-20T20:46:45.000Z",
        kind: "apply",
        applied: 0,
        kept: 1,
        skipped: 0,
        failed: 0,
        deletes: 0,
        journalId: "d8815b46-demo",
        message:
          "Nothing written to destination · 1 kept as mine (destination unchanged)",
      },
    ].slice(0, limit);
  }
  return invoke("get_sync_history", { limit });
}

export async function getProjectProfiles(): Promise<ProjectProfile[]> {
  return invoke("get_project_profiles");
}

export async function upsertProjectProfile(input: {
  name: string;
  mode: string;
  thisPc: string;
  broughtOver: string;
  replaceId?: string | null;
}): Promise<ProjectProfile> {
  return invoke("upsert_project_profile", {
    name: input.name,
    mode: input.mode,
    thisPc: input.thisPc,
    broughtOver: input.broughtOver,
    replaceId: input.replaceId ?? null,
  });
}

export async function removeProjectProfile(id: string): Promise<void> {
  return invoke("remove_project_profile", { id });
}
