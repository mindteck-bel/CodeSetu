import { useEffect, useState } from "react";
import { getFileDiff } from "../../shared/api";
import { modeCopy, type CompareMode } from "../../shared/compareMode";
import type { FileDiff } from "../../shared/types";

type Props = {
  mode: CompareMode;
  sessionId: string;
  relPath: string;
  onClose: () => void;
};

export function DiffViewer({ mode, sessionId, relPath, onClose }: Props) {
  const copy = modeCopy(mode);
  const [diff, setDiff] = useState<FileDiff | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let alive = true;
    setLoading(true);
    setError(null);
    getFileDiff(sessionId, relPath)
      .then((d) => {
        if (alive) setDiff(d);
      })
      .catch((e: Error) => {
        if (alive) setError(e.message ?? String(e));
      })
      .finally(() => {
        if (alive) setLoading(false);
      });
    return () => {
      alive = false;
    };
  }, [sessionId, relPath]);

  return (
    <div className="diff-drawer" role="dialog" aria-label="File diff">
      <div className="diff-header">
        <div>
          <p className="eyebrow">{copy.diffEyebrow}</p>
          <h3>{relPath}</h3>
          <p className="muted tiny">{copy.diffHint}</p>
        </div>
        <button type="button" className="btn ghost" onClick={onClose}>
          Close
        </button>
      </div>

      {loading && <div className="panel-loading">Reading file…</div>}
      {error && (
        <div className="fallback-panel compact">
          <p>Could not open this diff.</p>
          <pre className="fallback-detail">{error}</pre>
        </div>
      )}
      {diff && !loading && (
        <>
          {diff.error && <p className="warn-line">{diff.error}</p>}
          {(diff.kind === "binary" || diff.kind === "large") && (
            <div className="fallback-panel compact">
              <p>
                This file is {diff.kind}. {copy.binaryHint}
              </p>
            </div>
          )}
          {diff.kind === "text" && (
            <div className="diff-grid">
              <div className="diff-col">
                <div className="diff-col-title">{copy.diffLeftTitle}</div>
                <pre className="diff-body">
                  {diff.hunks.map((h, i) =>
                    h.tag === "insert" ? null : (
                      <span key={i} className={`hunk ${h.tag}`}>
                        {h.thisText}
                      </span>
                    ),
                  )}
                </pre>
              </div>
              <div className="diff-col">
                <div className="diff-col-title">{copy.diffRightTitle}</div>
                <pre className="diff-body">
                  {diff.hunks.map((h, i) =>
                    h.tag === "delete" ? null : (
                      <span key={i} className={`hunk ${h.tag}`}>
                        {h.broughtText}
                      </span>
                    ),
                  )}
                </pre>
              </div>
            </div>
          )}
          {diff.truncated && (
            <p className="muted">Diff truncated for speed.</p>
          )}
        </>
      )}
    </div>
  );
}
