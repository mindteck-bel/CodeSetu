import { useState } from "react";
import { modeCopy, type CompareMode } from "../../shared/compareMode";
import { FolderPairBar } from "../../shared/FolderPairBar";
import type { ApplyPreview } from "../../shared/types";

type Props = {
  mode: CompareMode;
  preview: ApplyPreview;
  destinationPath: string;
  sourcePath: string;
  applying: boolean;
  applyError: string | null;
  onCancel: () => void;
  onConfirm: () => void;
};

export function ApplyConfirmModal({
  mode,
  preview,
  destinationPath,
  sourcePath,
  applying,
  applyError,
  onCancel,
  onConfirm,
}: Props) {
  const copy = modeCopy(mode);
  const keeps = preview.keeps ?? 0;
  const willWrite = preview.rowsToApply > 0;
  const nothingToDo = !willWrite && keeps === 0;
  const needsGate = preview.destructive || Boolean(preview.firstSync);
  const needsDeleteAck = preview.deletes > 0;
  const [deleteAck, setDeleteAck] = useState("");
  const deleteOk = !needsDeleteAck || deleteAck.trim().toUpperCase() === "DELETE";
  const canConfirm = !applying && !nothingToDo && deleteOk;

  return (
    <div className="modal-backdrop" role="presentation">
      <div
        className="modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="confirm-title"
      >
        <h2 id="confirm-title">{copy.confirmTitle}</h2>
        <FolderPairBar
          mode={mode}
          destinationPath={destinationPath}
          sourcePath={sourcePath}
          compact
        />
        <p>{copy.confirmBody}</p>
        {preview.firstSync && (
          <p className="warn-line">
            First sync for this folder pair — there is no prior baseline. Review
            creates carefully before confirming.
          </p>
        )}
        {preview.selectedCount != null && preview.selectedCount > 0 && (
          <p className="muted">
            Applying {preview.selectedCount} selected file(s) only — other
            decisions stay untouched.
          </p>
        )}
        <ul className="confirm-stats">
          <li>{preview.creates} new files</li>
          <li>{preview.overwrites} overwrites</li>
          <li className={preview.deletes > 0 ? "danger-text" : ""}>
            {preview.deletes} local deletions
          </li>
          <li>{preview.moves} moves</li>
          <li>{keeps} keep mine (destination unchanged)</li>
          <li>{preview.skips} unresolved (Skip)</li>
        </ul>
        {nothingToDo && (
          <p className="error-line">
            Nothing will change on the destination. Choose Use source or Keep
            mine on the selected files first.
          </p>
        )}
        {!willWrite && keeps > 0 && (
          <p className="warn-line">
            No files will be written — Keep mine leaves destination bytes as
            they are and updates the sync baseline.
          </p>
        )}
        {preview.deletes > 0 && (
          <p className="error-line">{copy.deleteWarning}</p>
        )}
        {needsDeleteAck && (
          <label className="confirm-ack">
            <span>
              Type <strong>DELETE</strong> to confirm local deletions (Undo is
              available after apply if something goes wrong):
            </span>
            <input
              className="search-input"
              value={deleteAck}
              onChange={(e) => setDeleteAck(e.target.value)}
              autoComplete="off"
              spellCheck={false}
              aria-label="Type DELETE to confirm deletions"
              disabled={applying}
            />
          </label>
        )}
        {applyError && <p className="error-line">{applyError}</p>}
        <div className="actions">
          <button
            type="button"
            className="btn"
            onClick={onCancel}
            disabled={applying}
          >
            Cancel
          </button>
          <button
            type="button"
            className={
              preview.destructive ? "btn destructive" : "btn primary"
            }
            disabled={!canConfirm}
            onClick={onConfirm}
          >
            {applying
              ? "Applying…"
              : nothingToDo
                ? "Nothing to apply"
                : !deleteOk
                  ? "Type DELETE to continue"
                  : needsGate
                    ? preview.firstSync && !preview.destructive
                      ? "Confirm first sync apply"
                      : "Confirm destructive apply"
                    : "Apply"}
          </button>
        </div>
      </div>
    </div>
  );
}
