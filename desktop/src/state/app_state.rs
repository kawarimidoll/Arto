use dioxus::desktop::tao::dpi::{LogicalPosition, LogicalSize};
use dioxus::prelude::*;
use std::path::PathBuf;

use super::persistence::LAST_FOCUSED_STATE;
use crate::components::right_sidebar::RightSidebarTab;
use crate::markdown::HeadingInfo;
use crate::pinned_search::PinnedSearch;
use crate::search_match::MatchInfo;
use crate::theme::Theme;

mod sidebar;
mod tabs;

pub use sidebar::Sidebar;
pub use tabs::{Tab, TabContent};

/// Per-window application state.
///
/// # Copy Semantics
///
/// This struct implements `Copy` because all fields are `Signal<T>`, which are cheap to copy
/// (they contain only Arc pointers internally). This allows passing `AppState` to closures
/// and async blocks without explicit `.clone()` calls, making the code cleaner.
///
/// **This aligns with Dioxus design philosophy**: `Signal<T>` is intentionally `Copy` to enable
/// ergonomic state passing in reactive UIs. Wrapping `Signal` fields in a `Copy` struct is the
/// recommended pattern in Dioxus applications.
///
/// # Why Per-field Signals?
///
/// We use per-field `Signal<T>` instead of `Signal<AppState>` for fine-grained reactivity:
/// - Changing `current_theme` doesn't trigger re-renders in components that only watch `tabs`
/// - Different components can update different fields concurrently without conflicts
/// - Components subscribe only to the fields they need (e.g., Header watches theme, TabBar watches tabs)
///
/// If we used `Signal<AppState>`, any field change would trigger re-renders in ALL components
/// that access the state, causing unnecessary performance overhead.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AppState {
    pub tabs: Signal<Vec<Tab>>,
    pub active_tab: Signal<usize>,
    pub current_theme: Signal<Theme>,
    pub zoom_level: Signal<f64>,
    pub sidebar: Signal<Sidebar>,
    pub right_sidebar_open: Signal<bool>,
    pub right_sidebar_width: Signal<f64>,
    pub right_sidebar_tab: Signal<RightSidebarTab>,
    pub toc_headings: Signal<Vec<HeadingInfo>>,
    pub position: Signal<LogicalPosition<i32>>,
    pub size: Signal<LogicalSize<u32>>,
    // Search state (not persisted, managed via JavaScript for IME compatibility)
    pub search_open: Signal<bool>,
    pub search_match_count: Signal<usize>,
    pub search_current_index: Signal<usize>,
    /// Initial search text to populate when opening search bar
    pub search_initial_text: Signal<Option<String>>,
    /// Pinned searches (window global, synced across all windows via broadcast)
    pub pinned_searches: Signal<Vec<PinnedSearch>>,
    /// All search matches (updated by JavaScript callback)
    pub search_matches: Signal<Vec<MatchInfo>>,
}

impl Default for AppState {
    fn default() -> Self {
        let persisted = LAST_FOCUSED_STATE.read();
        // Load pinned searches from disk
        let pinned_searches = crate::pinned_search::PinnedSearchStorage::load();

        Self {
            tabs: Signal::new(vec![Tab::default()]),
            active_tab: Signal::new(0),
            current_theme: Signal::new(persisted.theme),
            zoom_level: Signal::new(1.0),
            sidebar: Signal::new(Sidebar::default()),
            right_sidebar_open: Signal::new(persisted.right_sidebar_open),
            right_sidebar_width: Signal::new(persisted.right_sidebar_width),
            right_sidebar_tab: Signal::new(persisted.right_sidebar_tab),
            toc_headings: Signal::new(Vec::new()),
            position: Signal::new(Default::default()),
            size: Signal::new(Default::default()),
            // Search state
            search_open: Signal::new(false),
            search_match_count: Signal::new(0),
            search_current_index: Signal::new(0),
            search_initial_text: Signal::new(None),
            // Pinned searches (loaded from disk, synced via broadcast)
            pinned_searches: Signal::new(pinned_searches),
            // Search matches (updated by JavaScript callback)
            search_matches: Signal::new(Vec::new()),
        }
    }
}

impl AppState {
    /// Set the root directory and add to history
    /// Note: The directory is persisted to state file when window closes
    pub fn set_root_directory(&mut self, path: impl Into<PathBuf>) {
        let path = path.into();
        let mut sidebar = self.sidebar.write();
        sidebar.root_directory = Some(path.clone());
        sidebar.expanded_dirs.clear();
        sidebar.push_to_history(path.clone());
        LAST_FOCUSED_STATE.write().directory = Some(path);
    }

    /// Set the root directory without adding to history (used for history navigation)
    fn set_root_directory_no_history(&mut self, path: PathBuf) {
        let mut sidebar = self.sidebar.write();
        sidebar.root_directory = Some(path.clone());
        sidebar.expanded_dirs.clear();
        LAST_FOCUSED_STATE.write().directory = Some(path);
    }

    /// Go back in directory history
    pub fn go_back_directory(&mut self) {
        let path = self.sidebar.write().go_back();
        if let Some(path) = path {
            self.set_root_directory_no_history(path);
        }
    }

    /// Go forward in directory history
    pub fn go_forward_directory(&mut self) {
        let path = self.sidebar.write().go_forward();
        if let Some(path) = path {
            self.set_root_directory_no_history(path);
        }
    }

    /// Navigate to parent directory
    pub fn go_to_parent_directory(&mut self) {
        let parent = {
            let sidebar = self.sidebar.read();
            sidebar
                .root_directory
                .as_ref()
                .and_then(|d| d.parent().map(|p| p.to_path_buf()))
        };
        if let Some(parent) = parent {
            self.set_root_directory(parent);
        }
    }

    /// Toggle right sidebar visibility
    pub fn toggle_right_sidebar(&mut self) {
        let new_state = !*self.right_sidebar_open.read();
        self.right_sidebar_open.set(new_state);
        LAST_FOCUSED_STATE.write().right_sidebar_open = new_state;
    }

    /// Set right sidebar width
    pub fn set_right_sidebar_width(&mut self, width: f64) {
        self.right_sidebar_width.set(width);
        LAST_FOCUSED_STATE.write().right_sidebar_width = width;
    }

    /// Set right sidebar active tab
    pub fn set_right_sidebar_tab(&mut self, tab: RightSidebarTab) {
        self.right_sidebar_tab.set(tab);
        LAST_FOCUSED_STATE.write().right_sidebar_tab = tab;
    }

    /// Toggle search bar visibility
    pub fn toggle_search(&mut self) {
        let new_state = !*self.search_open.read();
        self.search_open.set(new_state);
        if !new_state {
            // Clear match count when closing
            self.search_match_count.set(0);
            self.search_current_index.set(0);
        }
    }

    /// Update search results from JavaScript callback
    pub fn update_search_results(&mut self, count: usize, current: usize) {
        self.search_match_count.set(count);
        self.search_current_index.set(current);
    }

    /// Update search matches (called from JavaScript callback)
    pub fn update_search_matches(&mut self, matches: Vec<MatchInfo>) {
        tracing::debug!(count = matches.len(), "Updating search matches");
        self.search_matches.set(matches);
    }

    /// Open search bar and populate with given text
    pub fn open_search_with_text(&mut self, text: Option<String>) {
        // Set initial text for SearchBar to pick up
        self.search_initial_text.set(text);
        // Open search bar
        self.search_open.set(true);
    }

    /// Add a new pinned search (with automatic color assignment)
    pub fn add_pinned_search(&mut self, pattern: impl Into<String>, case_sensitive: bool) {
        let pattern = pattern.into();
        tracing::debug!(pattern = %pattern, case_sensitive, "add_pinned_search called");
        let pinned_clone = {
            let mut pinned = self.pinned_searches.write();

            // Auto-assign color based on current usage
            let color = crate::pinned_search::HighlightColor::next_color(&pinned);

            let new_search =
                crate::pinned_search::PinnedSearch::new(pattern, color, case_sensitive);
            tracing::debug!(?new_search, "Created new pinned search");
            pinned.push(new_search);

            // Clone before dropping write lock
            pinned.clone()
        };

        tracing::debug!(count = pinned_clone.len(), "Syncing pinned searches");
        // Save to disk and broadcast to other windows
        self.sync_pinned_searches(&pinned_clone);
    }

    /// Remove a pinned search by ID
    pub fn remove_pinned_search(&mut self, id: impl AsRef<str>) {
        let id = id.as_ref();
        let pinned_clone = {
            let mut pinned = self.pinned_searches.write();
            pinned.retain(|p| p.id != id);
            pinned.clone()
        };

        // Save to disk and broadcast to other windows
        self.sync_pinned_searches(&pinned_clone);
    }

    /// Update the color of a pinned search
    pub fn update_pinned_search_color(
        &mut self,
        id: impl AsRef<str>,
        color: crate::pinned_search::HighlightColor,
    ) {
        let id = id.as_ref();
        tracing::debug!(%id, ?color, "Updating pinned search color");
        let pinned_clone = {
            let mut pinned = self.pinned_searches.write();
            if let Some(search) = pinned.iter_mut().find(|p| p.id == id) {
                search.color = color;
            }
            pinned.clone()
        };

        // Save to disk and broadcast to other windows
        self.sync_pinned_searches(&pinned_clone);
    }

    /// Toggle the disabled state of a pinned search
    pub fn toggle_pinned_search(&mut self, id: impl AsRef<str>) {
        let id = id.as_ref();
        let pinned_clone = {
            let mut pinned = self.pinned_searches.write();
            if let Some(search) = pinned.iter_mut().find(|p| p.id == id) {
                search.disabled = !search.disabled;
            }
            pinned.clone()
        };

        // Save to disk and broadcast to other windows
        self.sync_pinned_searches(&pinned_clone);
    }

    /// Sync pinned searches to disk and broadcast to other windows
    fn sync_pinned_searches(&self, pinned: &[PinnedSearch]) {
        tracing::debug!(count = pinned.len(), "Syncing pinned searches");
        // Save to disk
        if let Err(e) = crate::pinned_search::PinnedSearchStorage::save(pinned.to_vec()) {
            tracing::error!(?e, "Failed to save pinned searches");
        }

        // Broadcast to other windows
        let _ = crate::events::PINNED_SEARCH_CHANGED.send(pinned.to_vec());
    }
}
