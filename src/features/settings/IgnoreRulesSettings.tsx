import { useEffect, useMemo, useState } from "react";
import {
  addIgnorePattern,
  deleteIgnorePattern,
  exportIgnoreRules,
  getIgnoreRules,
  importIgnoreRules,
  setIgnoreCategoryEnabled,
  setIgnorePatternEnabled,
  updateIgnorePattern,
} from "../../shared/api";
import type { IgnoreRulesView } from "../../shared/types";

type Props = {
  projectPath?: string;
  onClose: () => void;
  onRulesChanged: () => void;
};

export function validateIgnorePatternLocal(pattern: string): string | null {
  const p = pattern.trim();
  if (!p) return "Pattern cannot be empty.";
  if (p.includes("\0")) return "Pattern contains invalid characters.";
  // Light client-side check; engine re-validates with globset.
  if (p.startsWith("!") ) return "Negation patterns are not supported.";
  return null;
}

export function IgnoreRulesSettings({
  projectPath,
  onClose,
  onRulesChanged,
}: Props) {
  const [view, setView] = useState<IgnoreRulesView | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [search, setSearch] = useState("");
  const [custom, setCustom] = useState("");
  const [projectScope, setProjectScope] = useState(Boolean(projectPath));
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editValue, setEditValue] = useState("");
  const [importText, setImportText] = useState("");
  const [showPreview, setShowPreview] = useState(false);

  const reload = async () => {
    try {
      const v = await getIgnoreRules(projectPath);
      setView(v);
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  useEffect(() => {
    void reload();
  }, [projectPath]);

  const filtered = useMemo(() => {
    if (!view) return [];
    const q = search.trim().toLowerCase();
    return view.patterns.filter((p) => {
      if (!q) return true;
      return (
        p.pattern.toLowerCase().includes(q) ||
        p.category.toLowerCase().includes(q) ||
        p.source.toLowerCase().includes(q)
      );
    });
  }, [view, search]);

  const applyView = (v: IgnoreRulesView) => {
    setView(v);
    onRulesChanged();
  };

  const onTogglePattern = async (id: string, enabled: boolean) => {
    try {
      applyView(
        await setIgnorePatternEnabled(id, enabled, projectPath, projectScope),
      );
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  const onToggleCategory = async (id: string, enabled: boolean) => {
    try {
      applyView(
        await setIgnoreCategoryEnabled(id, enabled, projectPath, projectScope),
      );
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  const onAdd = async () => {
    const err = validateIgnorePatternLocal(custom);
    if (err) {
      setError(err);
      return;
    }
    try {
      applyView(await addIgnorePattern(custom, projectPath, projectScope));
      setCustom("");
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  const onSaveEdit = async () => {
    if (!editingId) return;
    const err = validateIgnorePatternLocal(editValue);
    if (err) {
      setError(err);
      return;
    }
    try {
      applyView(
        await updateIgnorePattern(
          editingId,
          editValue,
          projectPath,
          projectScope,
        ),
      );
      setEditingId(null);
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  const onDelete = async (id: string) => {
    try {
      applyView(await deleteIgnorePattern(id, projectPath, projectScope));
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  const onExport = async () => {
    try {
      const json = await exportIgnoreRules(projectPath);
      await navigator.clipboard.writeText(json);
      setError(null);
      setImportText(json);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  const onImport = async () => {
    try {
      applyView(
        await importIgnoreRules(importText, projectPath, projectScope),
      );
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  return (
    <section className="settings-panel enter" aria-label="Ignore Rules">
      <div className="settings-head">
        <div>
          <h1>Ignore Rules</h1>
          <p className="lede">
            Offline patterns excluded from both Intranet and Internet scans.
            Ignored paths never appear as deletions, conflicts, or bulk actions.
            Changing rules invalidates the current scan — run a fresh compare.
          </p>
        </div>
        <button type="button" className="btn" onClick={onClose}>
          Back
        </button>
      </div>

      <div className="settings-toolbar">
        <label className="scope-toggle">
          <input
            type="checkbox"
            checked={projectScope}
            disabled={!projectPath}
            onChange={(e) => setProjectScope(e.target.checked)}
          />
          Save as project override
          {!projectPath && (
            <span className="muted tiny"> (select Intranet folder first)</span>
          )}
        </label>
        <input
          className="search-input"
          placeholder="Search patterns…"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
        />
        <button type="button" className="btn" onClick={() => setShowPreview((v) => !v)}>
          {showPreview ? "Hide" : "Show"} effective preview
        </button>
        <button type="button" className="btn" onClick={onExport}>
          Export
        </button>
      </div>

      {view && (
        <div className="category-grid">
          {view.categories.map((c) => (
            <div key={c.id} className="category-card">
              <div className="category-top">
                <label>
                  <input
                    type="checkbox"
                    checked={c.enabled}
                    onChange={(e) => onToggleCategory(c.id, e.target.checked)}
                  />
                  <strong>{c.label}</strong>
                </label>
                <span className="muted tiny">
                  {c.activeCount}/{c.totalCount} active
                </span>
              </div>
              <p className="muted tiny">{c.explanation}</p>
            </div>
          ))}
        </div>
      )}

      <div className="add-pattern-row">
        <input
          className="search-input grow"
          placeholder="Add custom glob, e.g. **/vendor/**"
          value={custom}
          onChange={(e) => setCustom(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") void onAdd();
          }}
        />
        <button type="button" className="btn primary" onClick={onAdd}>
          Add pattern
        </button>
      </div>

      <div className="pattern-list">
        {filtered.map((p) => {
          const catOn = view?.categoryEnabled[p.category.toLowerCase()] ?? true;
          const active = p.enabled && catOn;
          return (
            <div key={p.id} className={`pattern-row${active ? " active" : ""}`}>
              <input
                type="checkbox"
                checked={p.enabled}
                onChange={(e) => onTogglePattern(p.id, e.target.checked)}
                aria-label={`Enable ${p.pattern}`}
              />
              {editingId === p.id ? (
                <>
                  <input
                    className="search-input grow"
                    value={editValue}
                    onChange={(e) => setEditValue(e.target.value)}
                  />
                  <button type="button" className="btn" onClick={onSaveEdit}>
                    Save
                  </button>
                </>
              ) : (
                <>
                  <code className="pattern-text">{p.pattern}</code>
                  <span className="muted tiny">
                    {p.category} · {p.source}
                    {active ? " · active" : ""}
                  </span>
                  {!p.builtin && (
                    <>
                      <button
                        type="button"
                        className="btn ghost"
                        onClick={() => {
                          setEditingId(p.id);
                          setEditValue(p.pattern);
                        }}
                      >
                        Edit
                      </button>
                      <button
                        type="button"
                        className="btn ghost"
                        onClick={() => onDelete(p.id)}
                      >
                        Delete
                      </button>
                    </>
                  )}
                  {p.builtin && !p.enabled && (
                    <span className="muted tiny">built-in disabled</span>
                  )}
                </>
              )}
            </div>
          );
        })}
      </div>

      {showPreview && view && (
        <div className="fallback-panel compact">
          <p className="eyebrow">Effective rules ({view.effectivePatterns.length})</p>
          <ul className="effective-list">
            {view.effectivePatterns.map((p) => (
              <li key={p}>
                <code>{p}</code>
              </li>
            ))}
          </ul>
        </div>
      )}

      <div className="import-box">
        <p className="eyebrow">Import JSON</p>
        <textarea
          value={importText}
          onChange={(e) => setImportText(e.target.value)}
          rows={4}
          placeholder="Paste exported ignore-rules JSON"
        />
        <button type="button" className="btn" onClick={onImport}>
          Import
        </button>
      </div>

      {error && <p className="error-line">{error}</p>}
      {view?.warnings?.map((w) => (
        <p key={w} className="warn-line">
          {w}
        </p>
      ))}
    </section>
  );
}
