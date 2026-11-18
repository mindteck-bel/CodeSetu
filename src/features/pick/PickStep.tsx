type Props = {
  thisPc: string;
  broughtOver: string;
  pickError: string | null;
  recoveryNote: string | null;
  onPick: (side: "this" | "brought") => void;
  onScan: () => void;
};

export function PickStep({
  thisPc,
  broughtOver,
  pickError,
  recoveryNote,
  onPick,
  onScan,
}: Props) {
  return (
    <section className="card-free pick-step enter">
      {recoveryNote && <p className="warn-line">{recoveryNote}</p>}
      <h1>Pick the two folders</h1>
      <p className="lede">
        Only the <strong>Intranet</strong> folder on This PC will be updated.
        The <strong>Internet</strong> / brought-over copy stays read-only.
      </p>
      <div className="folder-grid">
        <div className="folder-pane write">
          <p className="eyebrow">Intranet · This PC · will change</p>
          <p className="folder-path">{thisPc || "No folder selected"}</p>
          <button
            type="button"
            className="btn primary"
            onClick={() => onPick("this")}
          >
            Choose Intranet folder
          </button>
        </div>
        <div className="folder-pane read">
          <p className="eyebrow">Internet · brought over · read only</p>
          <p className="folder-path">{broughtOver || "No folder selected"}</p>
          <button type="button" className="btn" onClick={() => onPick("brought")}>
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
          onClick={onScan}
        >
          Scan differences
        </button>
      </div>
    </section>
  );
}
