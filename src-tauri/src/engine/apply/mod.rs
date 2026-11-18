//! Recoverable apply protocol (NOT multi-file atomicity).
//!
//! Transaction protocol:
//! 1. Persist `journal.inprogress.json` with pre-apply snapshot backup BEFORE any mutation.
//! 2. For each row: backup → mutate → append journal entry → flush journal.
//! 3. On success: write committed snapshot, promote journal to `{id}.json`, set `latest.txt`.
//! 4. On crash: `recover_incomplete()` rolls back using backups + restores snapshot.
//!
//! Individual file replaces use temp+fsync+rename (Windows-safe). The whole batch is
//! recoverable, not all-or-nothing atomic across files.

mod journal;
mod ops;
mod plan;
mod preview;
mod recover;
mod report;

#[cfg(test)]
mod tests;

pub use plan::apply_plan;
pub use preview::{preview_apply, ApplyPreview};
pub use recover::{recover_incomplete, undo_last};
pub use report::ApplyReport;
