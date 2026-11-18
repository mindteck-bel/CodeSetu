# CodeSetu apply transaction protocol

CodeSetu does **not** provide multi-file atomicity. An apply is a **recoverable
sequence** of per-file replacements with a durable journal and a pre-apply
snapshot backup.

## Guarantees

- Each file write uses temp → fsync → replace (Windows-safe overwrite).
- Before the first mutation, CodeSetu writes:
  - `journals/<session>.inprogress.json` with status `inProgress`
  - `backups/<session>/pre-apply-snapshot.json`
- After each successful mutation, the in-progress journal is rewritten with the
  new entry (including move source backups).
- On success:
  - committed snapshot is saved (generation bump)
  - journal promoted to `journals/<session>.json` with status `committed`
  - `journals/latest.txt` points at the session
  - in-progress file removed
- On crash / kill mid-apply:
  - startup and `recover_journals` roll back using backups
  - pre-apply snapshot is restored
- Undo restores file bytes **and** the pre-apply snapshot.

## Non-guarantees

- The batch is not all-or-nothing across files while in flight.
- A crash after some files are written leaves a partial tree until recovery runs.
- Power loss after a file replace but before the journal flush can leave that
  single file changed; recovery still restores the baseline snapshot and other
  journaled entries.

## Destructive ops

- Overwrites, deletes, and moves require UI confirmation (`confirmDestructive`).
- First sync (empty baseline) with new file creates also requires confirmation.
- Deletion decisions are labeled **Delete locally**, never “Bring into Intranet”.
- Incomplete / unverified brought-over trees never emit deletion classes.

## Diagnostics

- Local offline JSONL at `<data>/CodeSetu/logs/codesetu.jsonl` (scan/apply/undo/recover).
- Never leaves the machine; no network.
