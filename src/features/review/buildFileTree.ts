import type { FileRow } from "../../shared/types";

export type FileTreeNode = {
  name: string;
  /** Folder path prefix or file relPath */
  path: string;
  children?: FileTreeNode[];
  row?: FileRow;
};

type MutableNode = {
  name: string;
  path: string;
  children: Map<string, MutableNode>;
  row?: FileRow;
};

/**
 * Build a folder tree from changed files only.
 * Identical / unchanged paths are never included — folders appear only as
 * ancestors of files that actually have a change.
 */
export function buildFileTree(rows: FileRow[]): FileTreeNode[] {
  const root: MutableNode = { name: "", path: "", children: new Map() };

  for (const row of rows) {
    if (row.class === "identical") continue;
    const parts = row.relPath.split(/[/\\]/).filter(Boolean);
    if (parts.length === 0) continue;
    let cursor = root;
    let prefix = "";
    for (let i = 0; i < parts.length; i++) {
      const part = parts[i];
      prefix = prefix ? `${prefix}/${part}` : part;
      const isFile = i === parts.length - 1;
      if (isFile) {
        cursor.children.set(part, {
          name: part,
          path: row.relPath,
          children: new Map(),
          row,
        });
      } else {
        let next = cursor.children.get(part);
        if (!next || next.row) {
          next = { name: part, path: prefix, children: new Map() };
          cursor.children.set(part, next);
        }
        cursor = next;
      }
    }
  }

  return finalize(root);
}

function finalize(node: MutableNode): FileTreeNode[] {
  return [...node.children.values()]
    .sort((a, b) => {
      const aDir = !a.row;
      const bDir = !b.row;
      if (aDir !== bDir) return aDir ? -1 : 1;
      return a.name.localeCompare(b.name);
    })
    .map((child) => {
      if (child.row) {
        return { name: child.name, path: child.path, row: child.row };
      }
      return {
        name: child.name,
        path: child.path,
        children: finalize(child),
      };
    });
}

/** Collect every file row id under a folder node (inclusive). */
export function collectFileIds(node: FileTreeNode): string[] {
  if (node.row) return [node.row.id];
  const ids: string[] = [];
  for (const child of node.children ?? []) {
    ids.push(...collectFileIds(child));
  }
  return ids;
}

export function defaultExpandedPaths(roots: FileTreeNode[]): Set<string> {
  const open = new Set<string>();
  const walk = (nodes: FileTreeNode[], depth: number) => {
    for (const n of nodes) {
      if (n.children && n.children.length > 0) {
        if (depth < 2) open.add(n.path);
        walk(n.children, depth + 1);
      }
    }
  };
  walk(roots, 0);
  return open;
}
