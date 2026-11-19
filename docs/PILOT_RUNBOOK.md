# CodeSetu enterprise pilot runbook

Local-first ops guide for Phase 2 pilots (3–5 developers). CodeSetu never requires a server, account, or network after install.

## Goals

- Sync the same project across air-gapped and internet-connected trees via USB / approved media.
- Keep destination (This PC / Intranet) writes explicit, reviewable, and undoable.
- Collect enough local diagnostics to triage issues without shipping secrets automatically.

## Install

1. Use a Windows 10/11 x64 build from CI artifact `codesetu-windows` (NSIS or MSI) or a signed-off release.
2. Install offline if required. Launch CodeSetu with no network.
3. Confirm Settings → Theme and Ignore Rules load (Tauri desktop only).

## Daily sync loop

1. **Profiles (optional):** on Folders, save a named project profile for the destination + source pair.
2. **Folders:** pick Destination (writable) and Source (read-only), or load a profile.
3. **Scan:** wait for both trees; cancel is safe.
4. **Review:** resolve **Needs you**; use selective checkboxes when applying a subset.
5. **Apply:** confirm counts. If deletions are present, type **DELETE** to enable confirm.
6. **Result:** verify counts / journal id. Use **Undo last apply** if the tree looks wrong.
7. Copy the updated tree back over USB if the other environment needs it.

## Incomplete USB / removable media

- After a baseline exists, a partial brought tree should **not** offer mass deletions (warning on Review).
- If a USB letter changes between sessions, re-select folders or reload the profile after updating paths.
- Prefer copying the whole project tree before scanning; do not scan mid-copy.

## Crash / recovery

- Mid-apply crash → relaunch CodeSetu. Incomplete journals roll back automatically; a recovery banner appears on Folders.
- Committed apply that you regret → **Undo last apply** on Result (restores file bytes **and** snapshot baseline).
- If Undo is unavailable (“Nothing to undo”), rescan and inspect the destination tree manually.

## Diagnostics export

1. Settings → **Pilot tools** → **Export diagnostic report**, or Result → **Export diagnostics**.
2. Choose a local `.json` path (USB ok). The file is **not** uploaded.
3. Review before sharing: it includes recent log events, sync history, journal status, OS/arch, and local data-dir path (may reveal project locations).
4. Hand the file to your pilot contact over approved channels.

## Failure triage (short)

| Symptom | First action |
| --- | --- |
| Session expired / ignore rules dirty | Rescan from Folders |
| Partial apply failures | Undo → fix locks/permissions → rescan → selective apply |
| USB path missing | Re-pick folders; update profile |
| Mass unexpected creates (first sync) | Cancel; confirm you picked the correct pair |
| Unexpected deletions offered | Check incomplete-copy warning; do not confirm DELETE |

## What pilots should report

- Project type (Java/Spring, React/Node, multi-service) and approximate file count.
- Whether Apply / Undo / recovery behaved as expected.
- Scan time and any memory/UI stalls on large trees.
- Diagnostic JSON for reproducible failures (after redacting sensitive path names if required by policy).

## Non-goals for this pilot

- No cloud sync, accounts, or remote update channel.
- No Git push/pull — optional Git attribution remains later roadmap work.
- No multi-file atomic apply (recoverable journal protocol only; see `APPLY_TRANSACTION_PROTOCOL.md`).

## Related

- Manual GUI checklist: [`WINDOWS_BETA_CHECKLIST.md`](WINDOWS_BETA_CHECKLIST.md)
- Apply protocol: [`APPLY_TRANSACTION_PROTOCOL.md`](APPLY_TRANSACTION_PROTOCOL.md)
