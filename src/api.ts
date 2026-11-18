import { invoke } from "@tauri-apps/api/core";
import type {
  ApplyReport,
  Decision,
  FileDiff,
  FileRow,
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
  return invoke("update_decisions", { sessionId, updates });
}

export async function setAllNeedsYou(
  sessionId: string,
  decision: Decision,
): Promise<FileRow[]> {
  return invoke("set_all_needs_you", { sessionId, decision });
}

export async function executeApply(sessionId: string): Promise<ApplyReport> {
  return invoke("execute_apply", { sessionId });
}

export async function undoApply(): Promise<ApplyReport> {
  return invoke("undo_apply");
}

export async function getFileDiff(
  sessionId: string,
  relPath: string,
): Promise<FileDiff> {
  return invoke("get_file_diff", { sessionId, relPath });
}
