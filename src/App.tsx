import { useCallback, useEffect, useMemo, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import {
  cancelScan,
  executeApply,
  setAllNeedsYou,
  startScan,
  undoApply,
  updateDecisions,
  validateFolder,
} from "./api";
import { DiffViewer } from "./DiffViewer";
import { ErrorBoundary } from "./ErrorBoundary";
import { ReviewList } from "./ReviewList";
import type {
  ApplyReport,
  Decision,
  FileRow,
  ProgressEvent,
  ScanResult,
  Step,
} from "./types";
import "./App.css";

export default function App() {
  const [step, setStep] = useState<Step>("pick");
  const [thisPc, setThisPc] = useState("");
  const [broughtOver, setBroughtOver] = useState("");
  const [pickError, setPickError] = useState<string | null>(null);
  const [progress, setProgress] = useState<ProgressEvent | null>(null);
  const [scanError, setScanError] = useState<string | null>(null);
  const [result, setResult] = useState<ScanResult | null>(null);
  const [rows, setRows] = useState<FileRow[]>([]);
  const [filter, setFilter] = useState<
    "needsYou" | "bringIn" | "keep" | "all" | "other"
  >("needsYou");
  const [openPath, setOpenPath] = useState<string | null>(null);
  const [applying, setApplying] = useState(false);
  const [applyReport, setApplyReport] = useState<ApplyReport | null>(null);
  const [applyError, setApplyError] = useState<string | null>(null);
  const [sessionId, setSessionId] = useState<string | null>(null);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    listen<ProgressEvent>("scan-progress", (event) => {
      setProgress(event.payload);
    }).then((fn) => {
      unlisten = fn;
    });
    return () => {
      unlisten?.();
    };
  }, []);

  const pickFolder = async (side: "this" | "brought") => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title:
          side === "this"
            ? "Choose the folder on this PC (will be updated)"
            : "Choose the brought-over folder (read only)",
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
      setFilter(scan.summary.needsYou > 0 ? "needsYou" : "bringIn");
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
    return `${s.bringIn} will be brought in · ${s.keep} kept local · ${s.needsYou} need you · ${s.intranetOnly} intranet-only left alone · ${s.internetOnly} internet-only skipped`;
  }, [result]);

  const runApply = async () => {
    if (!sessionId) return;
    setApplying(true);
    setApplyError(null);
    try {
      const report = await executeApply(sessionId);
      setApplyReport(report);
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
  };

  return (
    <div className="app-shell">
      <div className="atmosphere" aria-hidden />
      <header className="topbar">
        <div className="brand-block">
          <img src="/codesetu.svg" alt="" width={36} height={36} />
          <div>
            <p className="brand">CodeSetu</p>
            <p className="tagline">Bridge intranet ↔ internet code</p>
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
      </header>

      <main className="stage">
        <ErrorBoundary fallbackTitle="Main view failed">
          {step === "pick" && (
            <section className="card-free pick-step enter">
              <h1>Pick the two folders</h1>
              <p className="lede">
                Only <strong>This PC</strong> will be updated. The brought-over
                copy stays read-only.
              </p>
              <div className="folder-grid">
                <div className="folder-pane write">
                  <p className="eyebrow">This PC · will change</p>
                  <p className="folder-path">{thisPc || "No folder selected"}</p>
                  <button
                    type="button"
                    className="btn primary"
                    onClick={() => pickFolder("this")}
                  >
                    Choose folder
                  </button>
                </div>
                <div className="folder-pane read">
                  <p className="eyebrow">Brought over · read only</p>
                  <p className="folder-path">
                    {broughtOver || "No folder selected"}
                  </p>
                  <button
                    type="button"
                    className="btn"
                    onClick={() => pickFolder("brought")}
                  >
                    Choose folder
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
                {progress?.message ?? "Starting…"} Huge trees stay responsive —
                you can cancel anytime.
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
                    Scanned in {(result.summary.elapsedMs / 1000).toFixed(1)}s ·{" "}
                    {result.summary.identical} identical hidden
                  </p>
                </div>
                <div className="actions">
                  <button
                    type="button"
                    className="btn"
                    onClick={() => bulkNeedsYou("bringIn")}
                  >
                    Bring in all conflicts
                  </button>
                  <button
                    type="button"
                    className="btn primary large"
                    disabled={applying}
                    onClick={runApply}
                  >
                    {applying ? "Applying…" : "Apply to This PC"}
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
                    ["bringIn", "Bring in"],
                    ["keep", "Keep"],
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
              <h1>Sync applied</h1>
              <p className="lede">
                {applyReport.applied} files updated · {applyReport.skipped}{" "}
                skipped · {applyReport.failed} failed
              </p>
              <p className="muted">
                Commit and push with your usual Git remote. CodeSetu does not
                push.
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
    </div>
  );
}
