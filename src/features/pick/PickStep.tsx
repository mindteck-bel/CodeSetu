import {
  MODE_OPTIONS,
  modeCopy,
  type CompareMode,
} from "../../shared/compareMode";

type Props = {
  mode: CompareMode;
  thisPc: string;
  broughtOver: string;
  pickError: string | null;
  recoveryNote: string | null;
  onModeChange: (mode: CompareMode) => void;
  onPick: (side: "this" | "brought") => void;
  onScan: () => void;
};

export function PickStep({
  mode,
  thisPc,
  broughtOver,
  pickError,
  recoveryNote,
  onModeChange,
  onPick,
  onScan,
}: Props) {
  const copy = modeCopy(mode);

  return (
    <section className="card-free pick-step enter">
      {recoveryNote && <p className="warn-line">{recoveryNote}</p>}
      <h1>{copy.pickTitle}</h1>
      <p className="lede">{copy.pickLede}</p>

      <fieldset className="mode-picker">
        <legend className="sr-only">Compare mode</legend>
        <p className="eyebrow">Compare mode</p>
        <div className="mode-options" role="radiogroup" aria-label="Compare mode">
          {MODE_OPTIONS.map((option) => {
            const selected = mode === option.id;
            return (
              <label
                key={option.id}
                className={`mode-option${selected ? " selected" : ""}`}
              >
                <input
                  type="radio"
                  name="compare-mode"
                  value={option.id}
                  checked={selected}
                  onChange={() => onModeChange(option.id)}
                />
                <span className="mode-option-title">{option.title}</span>
                <span className="mode-option-desc">{option.description}</span>
              </label>
            );
          })}
        </div>
      </fieldset>

      <div className="folder-grid">
        <div className="folder-pane write">
          <p className="eyebrow">{copy.destinationEyebrow}</p>
          <p className="folder-path">{thisPc || "No folder selected"}</p>
          <button
            type="button"
            className="btn primary"
            onClick={() => onPick("this")}
          >
            {copy.chooseDestination}
          </button>
        </div>
        <div className="folder-pane read">
          <p className="eyebrow">{copy.sourceEyebrow}</p>
          <p className="folder-path">{broughtOver || "No folder selected"}</p>
          <button type="button" className="btn" onClick={() => onPick("brought")}>
            {copy.chooseSource}
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
