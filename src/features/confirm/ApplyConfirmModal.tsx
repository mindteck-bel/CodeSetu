import { modeCopy, type CompareMode } from "../../shared/compareMode";
import type { ApplyPreview } from "../../shared/types";

type Props = {
  mode: CompareMode;
  preview: ApplyPreview;
  applying: boolean;
  applyError: string | null;
  onCancel: () => void;
  onConfirm: () => void;
};

export function ApplyConfirmModal({
  mode,
  preview,
  applying,
  applyError,
  onCancel,
  onConfirm,
}: Props) {
  const copy = modeCopy(mode);
  const needsGate = preview.destructive || Boolean(preview.firstSync);
  return (
    <div className="modal-backdrop" role="presentation">
      <div
        className="modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="confirm-title"
      >
        <h2 id="confirm-title">{copy.confirmTitle}</h2>
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
          <li>{preview.skips} skipped</li>
        </ul>
        {preview.deletes > 0 && (
          <p className="error-line">{copy.deleteWarning}</p>
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
            disabled={applying}
            onClick={onConfirm}
          >
            {applying
              ? "Applying…"
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
