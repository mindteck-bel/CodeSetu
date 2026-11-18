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
import { ErrorBoundary } from "./shared/ErrorBoundary";
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
import "./styles/App.css";

export default function App() {
  const [step, setStep] = useState<Step>("pick");
  const [returnStep, setReturnStep] = useState<Step>("pick");
  const [thisPc, setThisPc] = useState("");
  const [broughtOver, setBroughtOver] = useState("");
  const [pickError, setPickError] = useState<string | null>(null);
  const [progress, setProgress] = useState<ProgressEvent | null>(null);
  const [scanError, setScanError] = useState<string | null>(null);
  const [result, setResult] = useState<ScanResult | null>(null);
  const [rows, setRows] = useState<FileRow[]>([]);
  const [filter, setFilter] = useState<ReviewFilter>("needsYou");
  const [openPath, setOpenPath] = useState<string | null>(null);
  const [applying, setApplying] = useState(false);
  const [applyReport, setApplyReport] = useState<ApplyReport | null>(null);
  const [applyError, setApplyError] = useState<string | null>(null);
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [preview, setPreview] = useState<ApplyPreview | null>(null);
  const [showConfirm, setShowConfirm] = useState(false);
  const [theme, setTheme] = useState<"dark" | "light">(() => {
    const saved = localStorage.getItem("codesetu-theme");
    return saved === "light" ? "light" : "dark";
  });
  const [rulesDirty, setRulesDirty] = useState(false);
  const [recoveryNote, setRecoveryNote] = useState<string | null>(null);

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    localStorage.setItem("codesetu-theme", theme);
  }, [theme]);

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
          side === "this"
            ? "Choose Intranet folder on This PC (will be updated)"
            : "Choose Internet / brought-over folder (read only)",
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
    try {
      const updated = await setAllNeedsYou(sessionId, decision);
      setRows(updated);
    } catch (e) {
      setApplyError(e instanceof Error ? e.message : String(e));
    }
  };

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
      const p = await getApplyPreview(sessionId);
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
      const report = await executeApply(sessionId, confirmDestructive);
      setApplyReport(report);
      setShowConfirm(false);
      setStep("result");
    } catch (e) {
      setApplyError(e instanceof Error ? e.message : String(e));
    } finally {
      setApplying(false);
    }
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
      <div className="atmosphere" aria-hidden />
      <header className="topbar">
        <div className="brand-block">
          <img src="/codesetu.svg" alt="" width={36} height={36} />
          <div>
            <p className="brand">CodeSetu</p>
            <p className="tagline">Bridge Intranet ↔ Internet code</p>
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
          ).map(([key, label]) => (
            <li key={key} className={step === key ? "active" : ""}>
              {label}
            </li>
          ))}
        </ol>
        <div className="top-actions">
          <button
            type="button"
            className="btn ghost"
            onClick={() => setTheme((t) => (t === "dark" ? "light" : "dark"))}
          >
            {theme === "dark" ? "Light" : "Dark"}
          </button>
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
              thisPc={thisPc}
              broughtOver={broughtOver}
              pickError={pickError}
              recoveryNote={recoveryNote}
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
              result={result}
              rows={rows}
              filter={filter}
              applying={applying}
              rulesDirty={rulesDirty}
              applyError={applyError}
              sessionId={sessionId}
              openPath={openPath}
              onFilter={setFilter}
              onDecision={onDecision}
              onOpen={setOpenPath}
              onBulkNeedsYou={bulkNeedsYou}
              onApply={openConfirm}
            />
          )}

          {step === "result" && applyReport && (
            <ResultStep
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
