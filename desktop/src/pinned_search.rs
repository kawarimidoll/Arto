// Pinned Search module - keyword highlighting that persists across windows

mod storage;
mod types;

pub use storage::PinnedSearchStorage;
pub use types::{HighlightColor, PinnedSearch, PinnedSearchId};
