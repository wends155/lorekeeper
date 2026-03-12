//! Data models and validation for memory entries.

/// Memory entry modules.
pub mod entry;
/// Entry types and metadata schemas.
pub mod types;
/// Business logic for entry validation.
pub mod validation;

pub use entry::{Entry, EntryId, NewEntry, UpdateEntry};
pub use types::EntryType;
pub use validation::{validate_new_entry, validate_role, validate_update};
