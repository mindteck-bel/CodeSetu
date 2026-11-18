import { useCallback, useEffect, useMemo, useState } from "react";
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
} from "./api";
import { DiffViewer } from "./DiffViewer";
import { ErrorBoundary } from "./ErrorBoundary";
import { IgnoreRulesSettings } from "./IgnoreRulesSettings";
import { ReviewList } from "./ReviewList";
import type {
  ApplyPreview,
  ApplyReport,
  Decision,
  FileRow,
  ProgressEvent,
  ReviewFilter,
  ScanResult,
  Step,
} from "./types";
import "./App.css";

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

  const summaryLine = useMemo(() => {
    if (!result) return "";
    const s = result.summary;
    return `${s.bringIn} bring into Intranet · ${s.keep} keep Intranet · ${s.needsYou} need you · ${s.deletedBrought} deletions · ${s.intranetOnly} intranet-only · ${s.internetOnly} internet-only`;
  }, [result]);

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
          {recoveryNote && step === "pick" && (
            <p className="warn-line">{recoveryNote}</p>
          )}

          {step === "settings" && (
            <IgnoreRulesSettings
              projectPath={thisPc || undefined}
              onClose={() => setStep(returnStep)}
              onRulesChanged={() => setRulesDirty(true)}
            />
          )}

          {step === "pick" && (
            <section className="card-free pick-step enter">
              <h1>Pick the two folders</h1>
              <p className="lede">
                Only the <strong>Intranet</strong> folder on This PC will be
                updated. The <strong>Internet</strong> / brought-over copy stays
                read-only.
              </p>
              <div className="folder-grid">
                <div className="folder-pane write">
                  <p className="eyebrow">Intranet · This PC · will change</p>
                  <p className="folder-path">{thisPc || "No folder selected"}</p>
                  <button
                    type="button"
                    className="btn primary"
                    onClick={() => pickFolder("this")}
                  >
                    Choose Intranet folder
                  </button>
                </div>
                <div className="folder-pane read">
                  <p className="eyebrow">Internet · brought over · read only</p>
                  <p className="folder-path">
                    {broughtOver || "No folder selected"}
                  </p>
                  <button
                    type="button"
                    className="btn"
                    onClick={() => pickFolder("brought")}
                  >
                    Choose Internet folder
                  </button>
                </div>
              </div>
              {pickError && <p className="error-line">{pickError}</p>}
              <div className="actions">
                <button
                  type="button"
                  className="btn primary large"
                  disabled={!thisPc || !broughtOver}
                  onClick={beginScan}
                >
                  Scan differences
                </button>
              </div>
            </section>
          )}

          {step === "scan" && (
            <section className="card-free scan-step enter">
              <h1>Scanning</h1>
              <p className="lede">
                {progress?.message ?? "Starting…"} Both trees are hashed in
                parallel — you can cancel anytime.
              </p>
              <div className="scan-meter" aria-live="polite">
                <div className="scan-stats">
                  <div>
                    <span className="stat-num">
                      {progress?.filesSeen ?? 0}
                    </span>
                    <span className="stat-label">files seen</span>
                  </div>
                  <div>
                    <span className="stat-num">
                      {progress?.filesHashed ?? 0}
                    </span>
                    <span className="stat-label">hashed</span>
                  </div>
                </div>
                <p className="muted tiny">
                  Phase: {progress?.phase ?? "starting"}
                </p>
                <p className="current-path">
                  {progress?.currentPath || "Preparing workers…"}
                </p>
                <div className="pulse-bar" />
              </div>
              {scanError ? (
                <div className="fallback-panel">
                  <p>{scanError}</p>
                  <div className="actions">
                    <button type="button" className="btn" onClick={reset}>
                      Back
                    </button>
                    <button
                      type="button"
                      className="btn primary"
                      onClick={beginScan}
                    >
                      Retry
                    </button>
                  </div>
                </div>
              ) : (
                <button type="button" className="btn ghost" onClick={onCancelScan}>
                  Cancel scan
                </button>
              )}
            </section>
          )}

          {step === "review" && result && (
            <section className="review-step enter">
              <div className="review-head">
                <div>
                  <h1>Review changes</h1>
                  <p className="lede summary-line">{summaryLine}</p>
                  <p className="muted">
                    Intranet ← Internet · scanned in{" "}
                    {(result.summary.elapsedMs / 1000).toFixed(1)}s ·{" "}
                    {result.summary.identical} identical hidden
                  </p>
                  {rulesDirty && (
                    <p className="error-line">
                      Ignore rules changed — rescan before apply.
                    </p>
                  )}
                </div>
                <div className="actions">
                  <button
                    type="button"
                    className="btn"
                    onClick={() => bulkNeedsYou("bringIn")}
                  >
                    Bring into Intranet (all conflicts)
                  </button>
                  <button
                    type="button"
                    className="btn"
                    onClick={() => bulkNeedsYou("keep")}
                  >
                    Keep Intranet (all conflicts)
                  </button>
                  <button
                    type="button"
                    className="btn primary large"
                    disabled={applying || rulesDirty}
                    onClick={openConfirm}
                  >
                    {applying ? "Applying…" : "Apply to Intranet"}
                  </button>
                </div>
              </div>

              {result.warnings.length > 0 && (
                <ul className="warn-list">
                  {result.warnings.slice(0, 5).map((w) => (
                    <li key={w}>{w}</li>
                  ))}
                </ul>
              )}

              <div className="filter-bar">
                {(
                  [
                    ["needsYou", "Needs you"],
                    ["bringIn", "Bring into Intranet"],
                    ["keep", "Keep Intranet"],
                    ["deletions", `Deletions (${result.summary.deletedBrought})`],
                    ["other", "Env / secret"],
                    ["all", "All changes"],
                  ] as const
                ).map(([key, label]) => (
                  <button
                    key={key}
                    type="button"
                    className={filter === key ? "chip on" : "chip"}
                    onClick={() => setFilter(key)}
                  >
                    {label}
                  </button>
                ))}
              </div>

              <ErrorBoundary fallbackTitle="Review list failed">
                <ReviewList
                  rows={rows}
                  filter={filter}
                  onDecision={onDecision}
                  onOpen={setOpenPath}
                />
              </ErrorBoundary>

              {applyError && <p className="error-line">{applyError}</p>}

              {openPath && sessionId && (
                <ErrorBoundary fallbackTitle="Diff viewer failed">
                  <DiffViewer
                    sessionId={sessionId}
                    relPath={openPath}
                    onClose={() => setOpenPath(null)}
                  />
                </ErrorBoundary>
              )}
            </section>
          )}

          {step === "result" && applyReport && (
            <section className="card-free result-step enter">
              <h1>
                {applyReport.recovered ? "Recovered / undone" : "Sync applied"}
              </h1>
              <p className="lede">
                {applyReport.message
                  ? applyReport.message
                  : `${applyReport.applied} files updated · ${applyReport.skipped} skipped · ${applyReport.failed} failed`}
              </p>
              {applyReport.journalId && (
                <p className="muted">Journal: {applyReport.journalId}</p>
              )}
              <p className="muted">
                Commit and push with your usual Git remote. CodeSetu does not
                push. Undo restores files and the snapshot baseline.
              </p>
              {applyReport.failures.length > 0 && (
                <div className="fallback-panel compact">
                  <p>Some files failed — others still applied.</p>
                  <ul>
                    {applyReport.failures.slice(0, 8).map((f) => (
                      <li key={f}>{f}</li>
                    ))}
                  </ul>
                </div>
              )}
              {applyError && <p className="error-line">{applyError}</p>}
              <div className="actions">
                <button type="button" className="btn" onClick={runUndo}>
                  Undo last apply
                </button>
                <button type="button" className="btn primary" onClick={reset}>
                  New sync
                </button>
              </div>
            </section>
          )}
        </ErrorBoundary>
      </main>

      {showConfirm && preview && (
        <div className="modal-backdrop" role="presentation">
          <div
            className="modal"
            role="dialog"
            aria-modal="true"
            aria-labelledby="confirm-title"
          >
            <h2 id="confirm-title">Confirm apply to Intranet</h2>
            <p>
              This will write to <strong>This PC (Intranet)</strong> only. The
              Internet folder is never modified.
            </p>
            <ul className="confirm-stats">
              <li>{preview.creates} new files</li>
              <li>{preview.overwrites} overwrites</li>
              <li className={preview.deletes > 0 ? "danger-text" : ""}>
                {preview.deletes} local deletions
              </li>
              <li>{preview.moves} moves</li>
              <li>{preview.skips} skipped</li>
            </ul>
            {preview.deletes > 0 && (
              <p className="error-line">
                Deletions remove files from Intranet. This is never labeled
                “Bring into Intranet.”
              </p>
            )}
            {applyError && <p className="error-line">{applyError}</p>}
            <div className="actions">
              <button
                type="button"
                className="btn"
                onClick={() => setShowConfirm(false)}
                disabled={applying}
              >
                Cancel
              </button>
              <button
                type="button"
                className="btn primary"
                disabled={applying}
                onClick={() => runApply(true)}
              >
                {applying
                  ? "Applying…"
                  : preview.destructive
                    ? "Confirm destructive apply"
                    : "Apply"}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
