import type { ApplyPreview } from "../../shared/types";

type Props = {
  preview: ApplyPreview;
  applying: boolean;
  applyError: string | null;
  onCancel: () => void;
  onConfirm: () => void;
};

export function ApplyConfirmModal({
  preview,
  applying,
  applyError,
  onCancel,
  onConfirm,
}: Props) {
  const needsGate = preview.destructive || Boolean(preview.firstSync);
  return (
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
        {preview.firstSync && (
          <p className="warn-line">
            First sync for this folder pair — there is no prior baseline. Review
            creates carefully before confirming.
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
          <p className="error-line">
            Deletions remove files from Intranet. This is never labeled “Bring
            into Intranet.”
          </p>
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
            className="btn primary"
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
