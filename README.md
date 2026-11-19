# CodeSetu

Local desktop app (BSTC / Mindteck) that bridges **Intranet** and **Internet** copies of the same codebase.

You copy one side onto the other PC (USB), pick both folders, review what changed, and apply only the shared changes onto **This PC (Intranet)**. CodeSetu never pushes and never needs a server or cloud database.

## Why

Folder diff tools forget your decisions. Sync tools overwrite local work. CodeSetu remembers:

- intranet-only and internet-only paths (`codesetu.toml`)
- ignore rules (built-in categories + project/user overrides)
- what you already accepted last time (local snapshot)
- safe auto-choices, so you only sit with **Needs you**

## Quick start

On Windows you need [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with the **Desktop development with C++** workload (MSVC linker). Git’s `usr/bin/link.exe` must not shadow it.

```bash
npm install
npm run tauri dev
```

Build a Windows installer (NSIS + MSI):

```bash
npm run tauri build
# artifacts: src-tauri/target/release/bundle/nsis/*.exe
#            src-tauri/target/release/bundle/msi/*.msi
```

GitHub Actions runs fmt/clippy/tests on Linux + Windows and builds the installer on Windows — download the `codesetu-windows` artifact from the workflow run.

## Project rules & ignore rules

Copy [`codesetu.example.toml`](codesetu.example.toml) to `codesetu.toml` in the project you sync. Existing `codesetu.toml` is never overwritten by migration.

Use **Settings → Ignore Rules** for categories (VCS, deps, build, IDE, OS, temp, coverage), custom globs, import/export. Changing rules invalidates the current scan — run a fresh compare.

## Safety

- Only **Intranet / This PC** is written
- Durable per-file replace (Windows-safe) + **journal-first** apply + crash recovery
- **Undo** restores file bytes **and** the snapshot baseline
- Incomplete USB/copies never become mass deletions
- Destructive apply requires confirmation; deletions are labeled **Delete locally**, never “Bring into Intranet”
- Deletion applies require typing **DELETE** in the confirm dialog
- Path traversal rejected on all filesystem ops; hashes revalidated before apply
- See [`docs/APPLY_TRANSACTION_PROTOCOL.md`](docs/APPLY_TRANSACTION_PROTOCOL.md) (recoverable transaction protocol — not multi-file atomicity)

## Pilot tools (Phase 2)

- **Project profiles** — save/load named destination + source pairs (Folders step)
- **Sync history** — local apply/undo/recover log under Settings → Pilot tools
- **Diagnostic export** — offline JSON report (logs + history + journal status); never uploaded
- Ops guide: [`docs/PILOT_RUNBOOK.md`](docs/PILOT_RUNBOOK.md)

## Layout

- `src-tauri/src/engine/` — sync engine (scan, classify, apply, snapshot, ignore, paths, diag)
- `src-tauri/src/commands/` — thin Tauri IPC
- `src-tauri/src/session.rs` — in-memory session state
- `src/features/` — React feature folders (pick / scan / review / result / settings)
- `src/shared/` — types, API, ErrorBoundary

## Tests

```bash
cd src-tauri && cargo test --locked
npm test
npm run build
```

Windows desktop GUI checklist: [`docs/WINDOWS_BETA_CHECKLIST.md`](docs/WINDOWS_BETA_CHECKLIST.md).  
Enterprise pilot runbook: [`docs/PILOT_RUNBOOK.md`](docs/PILOT_RUNBOOK.md).
