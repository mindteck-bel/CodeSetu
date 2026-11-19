import { useEffect, useState } from "react";
import {
  MODE_OPTIONS,
  modeCopy,
  type CompareMode,
} from "../../shared/compareMode";
import type { ProjectProfile } from "../../shared/types";

type Props = {
  mode: CompareMode;
  thisPc: string;
  broughtOver: string;
  pickError: string | null;
  recoveryNote: string | null;
  profiles: ProjectProfile[];
  profileBusy?: boolean;
  profileMessage?: string | null;
  onModeChange: (mode: CompareMode) => void;
  onPick: (side: "this" | "brought") => void;
  onScan: () => void;
  onDismissRecovery?: () => void;
  onLoadProfile: (profile: ProjectProfile) => void;
  onSaveProfile: (name: string) => void;
  onDeleteProfile: (id: string) => void;
};

export function PickStep({
  mode,
  thisPc,
  broughtOver,
  pickError,
  recoveryNote,
  profiles,
  profileBusy,
  profileMessage,
  onModeChange,
  onPick,
  onScan,
  onDismissRecovery,
  onLoadProfile,
  onSaveProfile,
  onDeleteProfile,
}: Props) {
  const copy = modeCopy(mode);
  const [profileName, setProfileName] = useState("");
  const [selectedId, setSelectedId] = useState("");

  useEffect(() => {
    if (!selectedId && profiles.length > 0) {
      setSelectedId(profiles[0].id);
    }
  }, [profiles, selectedId]);

  const selected = profiles.find((p) => p.id === selectedId) ?? null;

  return (
    <section className="card-free pick-step enter">
      {recoveryNote && (
        <div className="recovery-banner" role="status">
          <div>
            <p className="warn-line">{recoveryNote}</p>
            <p className="muted tiny">
              Incomplete apply journals are rolled back automatically on launch.
              Re-scan after confirming the destination tree looks right. Use Undo
              from a Result screen only for the last committed apply.
            </p>
          </div>
          {onDismissRecovery && (
            <button type="button" className="btn ghost" onClick={onDismissRecovery}>
              Dismiss
            </button>
          )}
        </div>
      )}
      <h1>{copy.pickTitle}</h1>
      <p className="lede">{copy.pickLede}</p>

      <fieldset className="mode-picker">
        <legend className="sr-only">Compare mode</legend>
        <p className="eyebrow">Compare mode</p>
        <div className="mode-options" role="radiogroup" aria-label="Compare mode">
          {MODE_OPTIONS.map((option) => {
            const selectedMode = mode === option.id;
            return (
              <label
                key={option.id}
                className={`mode-option${selectedMode ? " selected" : ""}`}
              >
                <input
                  type="radio"
                  name="compare-mode"
                  value={option.id}
                  checked={selectedMode}
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

      <div className="profile-panel">
        <p className="eyebrow">Project profiles</p>
        <p className="muted tiny">
          Save named folder pairs for repeated pilot syncs. Stored only on this
          PC.
        </p>
        <div className="profile-row">
          <select
            className="theme-select profile-select"
            value={selectedId}
            onChange={(e) => setSelectedId(e.target.value)}
            aria-label="Saved project profile"
            disabled={profiles.length === 0}
          >
            {profiles.length === 0 && <option value="">No saved profiles</option>}
            {profiles.map((p) => (
              <option key={p.id} value={p.id}>
                {p.name}
              </option>
            ))}
          </select>
          <button
            type="button"
            className="btn"
            disabled={!selected || profileBusy}
            onClick={() => selected && onLoadProfile(selected)}
          >
            Load
          </button>
          <button
            type="button"
            className="btn ghost"
            disabled={!selected || profileBusy}
            onClick={() => selected && onDeleteProfile(selected.id)}
          >
            Delete
          </button>
        </div>
        <div className="profile-row">
          <input
            className="search-input grow"
            placeholder="Profile name, e.g. Payments service"
            value={profileName}
            onChange={(e) => setProfileName(e.target.value)}
            disabled={profileBusy}
          />
          <button
            type="button"
            className="btn"
            disabled={
              profileBusy || !profileName.trim() || !thisPc || !broughtOver
            }
            onClick={() => {
              onSaveProfile(profileName.trim());
              setProfileName("");
            }}
          >
            Save current
          </button>
        </div>
        {profileMessage && <p className="muted tiny">{profileMessage}</p>}
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
