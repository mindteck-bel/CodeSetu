import { useEffect, useMemo, useState } from "react";
import {
  changeClassLabel,
  type CompareMode,
} from "../../shared/compareMode";
import type { FileRow } from "../../shared/types";
import {
  buildFileTree,
  collectFileIds,
  defaultExpandedPaths,
  type FileTreeNode,
} from "./buildFileTree";

type Props = {
  mode: CompareMode;
  rows: FileRow[];
  selectedIds: Set<string>;
  openPath: string | null;
  onToggleSelect: (id: string, selected: boolean) => void;
  onToggleSelectMany: (ids: string[], selected: boolean) => void;
  onOpen: (relPath: string) => void;
};

export function FileTree({
  mode,
  rows,
  selectedIds,
  openPath,
  onToggleSelect,
  onToggleSelectMany,
  onOpen,
}: Props) {
  const tree = useMemo(() => buildFileTree(rows), [rows]);
  const treeKey = useMemo(
    () => rows.map((r) => r.relPath).join("\0"),
    [rows],
  );
  const [expanded, setExpanded] = useState<Set<string>>(() =>
    defaultExpandedPaths(tree),
  );

  useEffect(() => {
    setExpanded(defaultExpandedPaths(tree));
  }, [treeKey, tree]);

  if (rows.length === 0) {
    return (
      <div className="fallback-panel compact tree-empty">
        <p>No files in this filter.</p>
      </div>
    );
  }

  const allIds = rows.map((r) => r.id);
  const allSelected =
    allIds.length > 0 && allIds.every((id) => selectedIds.has(id));
  const someSelected = allIds.some((id) => selectedIds.has(id));

  const toggleExpand = (path: string) => {
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(path)) next.delete(path);
      else next.add(path);
      return next;
    });
  };

  return (
    <div className="file-tree-pane">
      <div className="file-tree-toolbar">
        <label className="tree-select-all">
          <input
            type="checkbox"
            checked={allSelected}
            ref={(el) => {
              if (el) el.indeterminate = someSelected && !allSelected;
            }}
            onChange={(e) => onToggleSelectMany(allIds, e.target.checked)}
            aria-label="Select all files in filter"
          />
          <span>
            {selectedIds.size > 0
              ? `${selectedIds.size} selected`
              : "Select files"}
          </span>
        </label>
        {selectedIds.size > 0 && (
          <button
            type="button"
            className="btn ghost tiny-btn"
            onClick={() => onToggleSelectMany([...selectedIds], false)}
          >
            Clear
          </button>
        )}
      </div>
      <div className="file-tree-scroll" role="tree">
        {tree.map((node) => (
          <TreeNodeView
            key={node.path}
            node={node}
            depth={0}
            mode={mode}
            expanded={expanded}
            selectedIds={selectedIds}
            openPath={openPath}
            onToggleExpand={toggleExpand}
            onToggleSelect={onToggleSelect}
            onToggleSelectMany={onToggleSelectMany}
            onOpen={onOpen}
          />
        ))}
      </div>
    </div>
  );
}

type NodeProps = {
  node: FileTreeNode;
  depth: number;
  mode: CompareMode;
  expanded: Set<string>;
  selectedIds: Set<string>;
  openPath: string | null;
  onToggleExpand: (path: string) => void;
  onToggleSelect: (id: string, selected: boolean) => void;
  onToggleSelectMany: (ids: string[], selected: boolean) => void;
  onOpen: (relPath: string) => void;
};

function TreeNodeView({
  node,
  depth,
  mode,
  expanded,
  selectedIds,
  openPath,
  onToggleExpand,
  onToggleSelect,
  onToggleSelectMany,
  onOpen,
}: NodeProps) {
  const pad = { paddingLeft: `${8 + depth * 14}px` };

  if (node.row) {
    const row = node.row;
    const active = openPath === row.relPath;
    const decisionMark =
      row.decision === "bringIn"
        ? "→"
        : row.decision === "keep"
          ? "✓"
          : row.decision === "skip"
            ? "–"
            : "";
    return (
      <div
        className={`tree-file class-${row.class}${active ? " active" : ""}`}
        style={pad}
        role="treeitem"
        aria-selected={active}
      >
        <label className="tree-check">
          <input
            type="checkbox"
            checked={selectedIds.has(row.id)}
            onChange={(e) => onToggleSelect(row.id, e.target.checked)}
            aria-label={`Select ${row.relPath}`}
            onClick={(e) => e.stopPropagation()}
          />
        </label>
        <button
          type="button"
          className="tree-file-main"
          onClick={() => onOpen(row.relPath)}
        >
          <span className="badge">{changeClassLabel(mode, row.class)}</span>
          <span className="tree-name" title={row.relPath}>
            {node.name}
          </span>
          {decisionMark && (
            <span className="tree-decision muted tiny" title={row.decision}>
              {decisionMark}
            </span>
          )}
        </button>
      </div>
    );
  }

  const ids = collectFileIds(node);
  const allOn = ids.length > 0 && ids.every((id) => selectedIds.has(id));
  const someOn = ids.some((id) => selectedIds.has(id));
  const isOpen = expanded.has(node.path);

  return (
    <div className="tree-folder" role="treeitem" aria-expanded={isOpen}>
      <div className="tree-folder-row" style={pad}>
        <label className="tree-check">
          <input
            type="checkbox"
            checked={allOn}
            ref={(el) => {
              if (el) el.indeterminate = someOn && !allOn;
            }}
            onChange={(e) => onToggleSelectMany(ids, e.target.checked)}
            aria-label={`Select folder ${node.path}`}
          />
        </label>
        <button
          type="button"
          className="tree-folder-toggle"
          onClick={() => onToggleExpand(node.path)}
          aria-expanded={isOpen}
        >
          <span className="tree-chevron" aria-hidden>
            {isOpen ? "▾" : "▸"}
          </span>
          <span className="tree-folder-name">{node.name}</span>
          <span className="muted tiny">{ids.length}</span>
        </button>
      </div>
      {isOpen &&
        (node.children ?? []).map((child) => (
          <TreeNodeView
            key={child.path}
            node={child}
            depth={depth + 1}
            mode={mode}
            expanded={expanded}
            selectedIds={selectedIds}
            openPath={openPath}
            onToggleExpand={onToggleExpand}
            onToggleSelect={onToggleSelect}
            onToggleSelectMany={onToggleSelectMany}
            onOpen={onOpen}
          />
        ))}
    </div>
  );
}
