# CodeSetu v0.1.x Windows beta — manual GUI checklist

Automated coverage lives in `cargo test` (Linux + Windows CI) and Vitest.
True desktop folder-picker / Tauri dialog flows still need a Windows machine.

## Install

1. Install from CI `codesetu-windows` artifact (NSIS or MSI) on a clean Windows 10/11 x64 PC.
2. Launch CodeSetu offline (no network required after install).

## Happy path

1. Pick Intranet (writable) and Internet (read-only) folders for the same project.
2. Scan — progress shows both trees; cancel works mid-scan.
3. Review — identical hidden; Needs you / Bring in / Deletions filters work.
4. Confirm apply modal shows creates/overwrites/deletes; first sync warns explicitly.
5. Apply succeeds; result shows counts + journal id.
6. Undo restores file bytes and next scan matches pre-apply baseline.

## Safety scenarios

1. **Crash mid-apply:** kill process during a large apply; relaunch — recovery note appears; tree restored.
2. **Incomplete source copy:** after a full sync baseline, scan against a partial Internet/source tree — deletions suppressed with warning.
3. **Ignore rules:** enable/disable categories; custom pattern; rescan required before apply; ignored paths never listed as deletions.
4. **Conflict:** edit same file on both sides independently — Needs you with conflict hint; no silent overwrite.
5. **Destructive delete:** Internet-deleted file — chip says **Delete locally**; confirm modal requires acknowledgment.
6. **Unusual names:** file with spaces/Unicode applies and undoes cleanly.
7. **Wrong folder pair:** first sync of unrelated trees still requires confirm before mass creates.

## Persistence

1. Close app after a successful apply; reopen and rescan — baseline still drives auto BringIn/Keep.
2. Project ignore overrides survive restart (`codesetu` ignore config under app data / project).
3. `codesetu.toml` zones still respected and never overwritten by ignore migration.

## Pass criteria for beta sign-off

- [ ] Installer runs on clean Windows x64
- [ ] Happy path apply + undo verified
- [ ] Crash recovery verified once
- [ ] Incomplete copy does not offer mass deletions
- [ ] No silent overwrites of local edits after scan

---

## Phase 2 pilot additions

Ops detail: [`PILOT_RUNBOOK.md`](PILOT_RUNBOOK.md).

1. **Project profile:** save current folder pair → restart app → Load profile → paths restore.
2. **Typed delete confirm:** Internet-deleted file → Apply → must type `DELETE` before confirm enables.
3. **Failure UX:** force a locked/unwritable destination file during apply (optional) → Result shows failures + Undo + Export diagnostics.
4. **Diagnostics export:** Settings → Pilot tools → export JSON → open offline; contains `reportKind: pilot-diagnostics` and recent history/log events.
5. **Sync history:** after an apply, Settings history list shows an Apply row (local only).
6. **Path change:** relocate or re-point one folder path, re-pick or update profile, scan without false mass deletions.
