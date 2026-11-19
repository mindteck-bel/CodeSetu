import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import {
  cancelScan,
  executeApply,
  getApplyPreview,
  recoverJournals,
  setAllNeedsYou,
  startScan,
  undoApply,
  updateDecisions,
  validateFolder,
} from "./shared/api";
import {
  modeCopy,
  readComparePrefs,
  writeComparePrefs,
  type CompareMode,
} from "./shared/compareMode";
import { ErrorBoundary } from "./shared/ErrorBoundary";
import {
  applyTheme,
  readThemePreference,
  type ThemePreference,
} from "./shared/theme";
import type {
  ApplyPreview,
  ApplyReport,
  Decision,
  FileRow,
  ProgressEvent,
  ReviewFilter,
  ScanResult,
  Step,
} from "./shared/types";
import { ApplyConfirmModal } from "./features/confirm/ApplyConfirmModal";
import { PickStep } from "./features/pick/PickStep";
import { ResultStep } from "./features/result/ResultStep";
import { ReviewStep } from "./features/review/ReviewStep";
import { ScanStep } from "./features/scan/ScanStep";
import { IgnoreRulesSettings } from "./features/settings/IgnoreRulesSettings";
import { buildReviewDemo } from "./features/review/reviewDemoData";
import "./styles/App.css";

function demoBootstrap(): {
  step: Step;
  result: ScanResult | null;
  rows: FileRow[];
  sessionId: string | null;
  openPath: string | null;
  filter: ReviewFilter;
  mode: CompareMode;
} | null {
  if (typeof window === "undefined") return null;
  if (window.location.hash !== "#review-demo") return null;
  const demo = buildReviewDemo();
  return {
    step: "review",
    result: demo.result,
    rows: demo.rows,
    sessionId: demo.result.sessionId,
    openPath: "package.json",
    filter: "needsYou",
    mode: "flexible",
  };
}

export default function App() {
  const demo = demoBootstrap();
  const [step, setStep] = useState<Step>(demo?.step ?? "pick");
  const [returnStep, setReturnStep] = useState<Step>("pick");
  const [compareMode, setCompareMode] = useState<CompareMode>(
    () => demo?.mode ?? readComparePrefs().mode,
  );
  const [thisPc, setThisPc] = useState(() => readComparePrefs().thisPc);
  const [broughtOver, setBroughtOver] = useState(
    () => readComparePrefs().broughtOver,
  );
  const [pickError, setPickError] = useState<string | null>(null);
  const [progress, setProgress] = useState<ProgressEvent | null>(null);
  const [scanError, setScanError] = useState<string | null>(null);
  const [result, setResult] = useState<ScanResult | null>(demo?.result ?? null);
  const [rows, setRows] = useState<FileRow[]>(demo?.rows ?? []);
  const [filter, setFilter] = useState<ReviewFilter>(demo?.filter ?? "needsYou");
  const [openPath, setOpenPath] = useState<string | null>(demo?.openPath ?? null);
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [applying, setApplying] = useState(false);
  const [applyReport, setApplyReport] = useState<ApplyReport | null>(null);
  const [applyError, setApplyError] = useState<string | null>(null);
  const [sessionId, setSessionId] = useState<string | null>(
    demo?.sessionId ?? null,
  );
  const [preview, setPreview] = useState<ApplyPreview | null>(null);
  const [showConfirm, setShowConfirm] = useState(false);
  const [themePreference, setThemePreference] = useState<ThemePreference>(() =>
    readThemePreference(),
  );
  const [rulesDirty, setRulesDirty] = useState(false);
  const [recoveryNote, setRecoveryNote] = useState<string | null>(null);

  const copy = modeCopy(compareMode);

  useEffect(() => {
    writeComparePrefs({ mode: compareMode, thisPc, broughtOver });
  }, [compareMode, thisPc, broughtOver]);

  useEffect(() => {
    applyTheme(themePreference);
  }, [themePreference]);

  useEffect(() => {
    if (themePreference !== "system") return;
    const mq = window.matchMedia("(prefers-color-scheme: light)");
    const onChange = () => applyTheme("system");
    mq.addEventListener("change", onChange);
    return () => mq.removeEventListener("change", onChange);
  }, [themePreference]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    listen<ProgressEvent>("scan-progress", (event) => {
      setProgress(event.payload);
      if (event.payload.sessionId) {
        setSessionId((prev) => prev ?? event.payload.sessionId);
      }
    }).then((fn) => {
      unlisten = fn;
    });
    return () => {
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    recoverJournals()
      .then((reports) => {
        if (reports.length > 0) {
          const n = reports.reduce((a, r) => a + r.applied, 0);
          setRecoveryNote(
            `Recovered incomplete apply — restored ${n} file change(s) from journal backups.`,
          );
        }
      })
      .catch(() => {
        /* offline/startup best-effort */
      });
  }, []);

  const pickFolder = async (side: "this" | "brought") => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title:
          side === "this" ? copy.dialogDestination : copy.dialogSource,
      });
      if (typeof selected === "string") {
        if (side === "this") setThisPc(selected);
        else setBroughtOver(selected);
        setPickError(null);
      }
    } catch (e) {
      setPickError(e instanceof Error ? e.message : String(e));
    }
  };

  const beginScan = async () => {
    setPickError(null);
    setScanError(null);
    setProgress(null);
    setRulesDirty(false);
    try {
      await validateFolder(thisPc);
      await validateFolder(broughtOver);
    } catch (e) {
      setPickError(e instanceof Error ? e.message : String(e));
      return;
    }
    setStep("scan");
    try {
      // Engine always treats first path as writable destination (This PC).
      const scan = await startScan(thisPc, broughtOver);
      setResult(scan);
      setRows(scan.rows);
      setSessionId(scan.sessionId);
      setFilter(
        scan.summary.needsYou > 0
          ? "needsYou"
          : scan.summary.deletedBrought > 0
            ? "deletions"
            : "bringIn",
      );
      setSelectedIds(new Set());
      setOpenPath(null);
      setStep("review");
    } catch (e) {
      setScanError(e instanceof Error ? e.message : String(e));
    }
  };

  const onCancelScan = async () => {
    if (sessionId) await cancelScan(sessionId);
    else if (progress?.sessionId) await cancelScan(progress.sessionId);
  };

  const onDecision = useCallback(
    async (id: string, decision: Decision) => {
      setRows((prev) =>
        prev.map((r) => (r.id === id ? { ...r, decision } : r)),
      );
      if (!sessionId) return;
      try {
        await updateDecisions(sessionId, [[id, decision]]);
      } catch (e) {
        setApplyError(e instanceof Error ? e.message : String(e));
      }
    },
    [sessionId],
  );

  const bulkNeedsYou = async (decision: Decision) => {
    if (!sessionId) return;
    const only =
      selectedIds.size > 0 ? Array.from(selectedIds) : null;
    if (sessionId.startsWith("demo-")) {
      setRows((prev) =>
        prev.map((r) => {
          const inScope = only
            ? only.includes(r.id) &&
              r.class !== "intranetOnly" &&
              r.class !== "internetOnly" &&
              r.class !== "parkedSecret"
            : r.class === "needsYou";
          return inScope ? { ...r, decision } : r;
        }),
      );
      return;
    }
    try {
      const updated = await setAllNeedsYou(sessionId, decision, only);
      setRows(updated);
    } catch (e) {
      setApplyError(e instanceof Error ? e.message : String(e));
    }
  };

  const selectionIdsOrNull = () =>
    selectedIds.size > 0 ? Array.from(selectedIds) : null;

  const openConfirm = async () => {
    if (!sessionId) return;
    if (rulesDirty) {
      setApplyError(
        "Ignore rules changed since this scan. Run a fresh compare before applying.",
      );
      return;
    }
    setApplyError(null);
    try {
      const p = await getApplyPreview(sessionId, selectionIdsOrNull());
      setPreview(p);
      setShowConfirm(true);
    } catch (e) {
      setApplyError(e instanceof Error ? e.message : String(e));
    }
  };

  const runApply = async (confirmDestructive: boolean) => {
    if (!sessionId) return;
    setApplying(true);
    setApplyError(null);
    try {
      const report = await executeApply(
        sessionId,
        confirmDestructive,
        selectionIdsOrNull(),
      );
      setApplyReport(report);
      setShowConfirm(false);
      setStep("result");
    } catch (e) {
      setApplyError(e instanceof Error ? e.message : String(e));
    } finally {
      setApplying(false);
    }
  };

  const toggleSelect = (id: string, selected: boolean) => {
    setSelectedIds((prev) => {
      const next = new Set(prev);
      if (selected) next.add(id);
      else next.delete(id);
      return next;
    });
  };

  const toggleSelectMany = (ids: string[], selected: boolean) => {
    setSelectedIds((prev) => {
      const next = new Set(prev);
      for (const id of ids) {
        if (selected) next.add(id);
        else next.delete(id);
      }
      return next;
    });
  };

  const backFromReview = () => {
    setStep("pick");
    setResult(null);
    setRows([]);
    setSessionId(null);
    setOpenPath(null);
    setSelectedIds(new Set());
    setApplyError(null);
    setPreview(null);
    setShowConfirm(false);
    setProgress(null);
    setScanError(null);
  };

  const runUndo = async () => {
    setApplyError(null);
    try {
      const report = await undoApply();
      setApplyReport(report);
    } catch (e) {
      setApplyError(e instanceof Error ? e.message : String(e));
    }
  };

  const reset = () => {
    setStep("pick");
    setResult(null);
    setRows([]);
    setSessionId(null);
    setApplyReport(null);
    setOpenPath(null);
    setSelectedIds(new Set());
    setProgress(null);
    setScanError(null);
    setApplyError(null);
    setPreview(null);
    setShowConfirm(false);
    setRulesDirty(false);
  };

  const openSettings = () => {
    setReturnStep(step === "settings" ? "pick" : step);
    setStep("settings");
  };

  return (
    <div className="app-shell">
      <header className="topbar">
        <div className="brand-block">
          <img src="/codesetu.svg" alt="" width={32} height={32} />
          <div>
            <p className="brand">CodeSetu</p>
            <p className="tagline">{copy.tagline}</p>
          </div>
        </div>
        <ol className="steps" aria-label="Progress">
          {(
            [
              ["pick", "1 Folders"],
              ["scan", "2 Scan"],
              ["review", "3 Review"],
              ["result", "4 Apply"],
            ] as const
          ).map(([key, label]) => {
            const canGoBack =
              (key === "pick" &&
                (step === "scan" || step === "review" || step === "result")) ||
              (key === "scan" && step === "review");
            return (
              <li key={key} className={step === key ? "active" : ""}>
                {canGoBack ? (
                  <button
                    type="button"
                    className="step-link"
                    onClick={() => {
                      if (key === "pick") {
                        if (step === "result") reset();
                        else backFromReview();
                      } else if (key === "scan" && step === "review") {
                        // Scan is transient; revisit folders to rescan.
                        backFromReview();
                      }
                    }}
                  >
                    {label}
                  </button>
                ) : (
                  label
                )}
              </li>
            );
          })}
        </ol>
        <div className="top-actions">
          <label className="sr-only" htmlFor="theme-select">
            Theme
          </label>
          <select
            id="theme-select"
            className="theme-select"
            value={themePreference}
            onChange={(e) =>
              setThemePreference(e.target.value as ThemePreference)
            }
            aria-label="Theme"
          >
            <option value="dark">Dark</option>
            <option value="light">Light</option>
            <option value="system">System</option>
          </select>
          <button type="button" className="btn" onClick={openSettings}>
            Settings
          </button>
        </div>
      </header>

      <main className="stage">
        <ErrorBoundary fallbackTitle="Main view failed">
          {step === "settings" && (
            <IgnoreRulesSettings
              projectPath={thisPc || undefined}
              onClose={() => setStep(returnStep)}
              onRulesChanged={() => setRulesDirty(true)}
            />
          )}

          {step === "pick" && (
            <PickStep
              mode={compareMode}
              thisPc={thisPc}
              broughtOver={broughtOver}
              pickError={pickError}
              recoveryNote={recoveryNote}
              onModeChange={setCompareMode}
              onPick={pickFolder}
              onScan={beginScan}
            />
          )}

          {step === "scan" && (
            <ScanStep
              progress={progress}
              scanError={scanError}
              onCancel={onCancelScan}
              onRetry={beginScan}
              onBack={reset}
            />
          )}

          {step === "review" && result && (
            <ReviewStep
              mode={compareMode}
              result={result}
              rows={rows}
              filter={filter}
              applying={applying}
              rulesDirty={rulesDirty}
              applyError={applyError}
              sessionId={sessionId}
              openPath={openPath}
              selectedIds={selectedIds}
              onFilter={setFilter}
              onDecision={onDecision}
              onOpen={setOpenPath}
              onBulkNeedsYou={bulkNeedsYou}
              onApply={openConfirm}
              onBack={backFromReview}
              onToggleSelect={toggleSelect}
              onToggleSelectMany={toggleSelectMany}
            />
          )}

          {step === "result" && applyReport && (
            <ResultStep
              mode={compareMode}
              applyReport={applyReport}
              applyError={applyError}
              onUndo={runUndo}
              onReset={reset}
            />
          )}
        </ErrorBoundary>
      </main>

      {showConfirm && preview && (
        <ApplyConfirmModal
          mode={compareMode}
          preview={preview}
          applying={applying}
          applyError={applyError}
          onCancel={() => setShowConfirm(false)}
          onConfirm={() => runApply(true)}
        />
      )}
    </div>
  );
}
