# Architecture Overview: Config, PersistedState, State

Understanding the relationship between Config, PersistedState, and State modules.

## Single-Instance Enforcement

**Arto runs as a single process enforced via IPC:**

- First launch → Becomes primary instance, starts IPC server
- Subsequent launches → Connect to primary, send paths via JSON Lines, exit(0)
- Primary instance receives paths via IPC server → Opens files/directories in existing windows

**IPC Protocol:** Unix domain socket (`com.lambdalisue.arto.sock`) with JSON Lines messages

**Why:** Prevents multiple processes from conflicting over file watches, config writes, and state persistence.

**Implementation:** See `desktop/src/ipc.rs` for detailed documentation.

**Note:** This document focuses on the state management within the single running instance.

## Three-Layer Architecture

```
┌─────────────────────────────────────────────────────────────┐
│ User Configuration Layer (config/)                          │
│ - File: config.json                                         │
│ - Edited by: User (manual or via Preferences UI)           │
│ - Contains: Default values, behavior settings               │
│ - Example: "default_theme": "auto"                          │
│           "on_startup": "last_closed"                       │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ State Persistence Layer (state/persistence.rs)              │
│ - File: state.json                                          │
│ - Edited by: App (auto-saved on window close)              │
│ - Contains: Last closed window's state (PersistedState)     │
│ - Example: "theme": "dark"                                  │
│           "directory": "/path/to/project"                   │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ Runtime State Layer (state.rs)                              │
│ - File: None (memory only)                                  │
│ - Scope: Per-window                                         │
│ - Contains: Current UI state (tabs, zoom, sidebar, etc.)   │
│ - Example: tabs: [Tab1, Tab2], active_tab: 0               │
│           sidebar.is_visible: true                          │
└─────────────────────────────────────────────────────────────┘
```

## Module Responsibilities

### 1. Config Module (`desktop/src/config/`)

**Purpose:** User preferences and default values

**Files:**
- `config.json` - Stored in `~/Library/Application Support/arto/` (macOS)

**Example content:**
```json
{
  "theme": {
    "defaultTheme": "auto",
    "onStartup": "last_closed",
    "onNewWindow": "last_focused"
  },
  "directory": {
    "defaultDirectory": "/Users/alice/Documents",
    "onStartup": "default",
    "onNewWindow": "last_focused"
  },
  "sidebar": {
    "defaultOpen": true,
    "defaultWidth": 280.0,
    "defaultShowAllFiles": false
  }
}
```

**Key type:** `Config`

**When used:**
- On app startup (to determine default behavior)
- When user opens Preferences and saves changes
- As fallback when state.json doesn't exist

### 2. State Persistence (`desktop/src/state/persistence.rs`)

**Purpose:** Remember the last closed window's state for restoration

**Files:**
- `state.json` - Stored in `~/Library/Application Support/arto/` (macOS)

**Example content:**
```json
{
  "directory": "/Users/alice/project/docs",
  "theme": "dark",
  "sidebarOpen": true,
  "sidebarWidth": 320.0,
  "sidebarShowAllFiles": false,
  "rightSidebarOpen": false,
  "rightSidebarWidth": 280.0,
  "rightSidebarTab": "toc",
  "windowPosition": { "x": 100, "y": 100 },
  "windowSize": { "width": 1200, "height": 800 }
}
```

**Key type:** `PersistedState`

**When used:**
- Saved: When any window closes (`use_drop()` in App component)
- Loaded: On app startup (if user configured `on_startup: "last_closed"`)

### 3. State Module (`desktop/src/state.rs`)

**Purpose:** Current UI state for each window instance

**Storage:** Memory only (never saved to disk)

**Key type:** `AppState`

**Contents:**
```rust
pub struct AppState {
    pub tabs: Signal<Vec<Tab>>,              // Open tabs
    pub active_tab: Signal<usize>,           // Which tab is active
    pub current_theme: Signal<Theme>,        // Current theme
    pub zoom_level: Signal<f64>,             // Zoom level
    pub sidebar: Signal<SidebarState>,       // Sidebar state
}
```

**Lifecycle:**
- Created: When window opens
- Updated: During user interaction (opening files, changing theme, etc.)
- Destroyed: When window closes (after saving to state.json)

## Data Flow

### Startup Flow (Primary Instance)

**Single-instance enforcement happens BEFORE initialization:**

```
0. IPC Check (in main(), before any initialization)
   ├─> Try to connect to existing instance via Unix socket
   ├─> If connection succeeds → Send paths via JSON Lines → exit(0)
   └─> If connection fails → Continue as primary instance

1. Initialize primary instance
   ├─> Load .env, init tracing
   ├─> Create OpenEvent channel (tx/rx)
   ├─> Start IPC server (listens for future instances)
   └─> Send CLI paths as OpenEvents

2. Load config.json
   ├─> Config { theme.on_startup: "last_closed" }
   └─> Config { directory.on_startup: "default" }

3. Load state.json
   ├─> PersistedState { theme: "dark" }
   └─> PersistedState { directory: "/path/to/project" }

4. Apply startup behavior using window::settings helpers
   ├─> Theme: "last_closed" → Use persisted.theme ("dark")
   └─> Directory: "default" → Use config.default_directory

5. Create AppState with computed values
   └─> AppState { current_theme: "dark", sidebar.root_directory: "/Users/alice/Documents" }
```

**Secondary Instance Flow:**

```
0. IPC Check (in main())
   ├─> Try to connect to existing instance
   ├─> Connection succeeds!
   ├─> Send paths: [{"type":"file","path":"/doc.md"}]
   └─> exit(0) immediately (no initialization)

Primary instance receives:
   ├─> IPC server accepts connection
   ├─> Parse JSON Lines messages
   ├─> Convert to OpenEvents
   ├─> Send to OPEN_EVENT_RECEIVER channel
   └─> MainApp component processes events (open file/directory)
```

### New Window Flow (Second+ Window)

```
1. Load config.json
   ├─> Config { theme.on_new_window: "last_focused" }
   └─> Config { directory.on_new_window: "last_focused" }

2. Read in-memory LAST_FOCUSED_STATE
   ├─> LAST_FOCUSED_STATE.theme (from last focused window)
   └─> LAST_FOCUSED_STATE.directory (from last focused window)

3. Apply new window behavior using window::settings helpers
   ├─> Theme: "last_focused" → Use LAST_FOCUSED_STATE.theme
   └─> Directory: "last_focused" → Use LAST_FOCUSED_STATE.directory

4. Create AppState with computed values
   └─> AppState { current_theme: <from LAST_FOCUSED_STATE>, sidebar.root_directory: <from LAST_FOCUSED_STATE> }
```

### Window Close Flow

```
1. Read current AppState
   ├─> current_theme: "dark"
   ├─> sidebar.root_directory: "/path/to/project"
   ├─> sidebar.is_visible: true
   ├─> sidebar.width: 320.0
   └─> sidebar.hide_non_markdown: false

2. Construct PersistedState
   └─> let mut persisted = PersistedState::from(&state);
       persisted.window_position = window_metrics.position;
       persisted.window_size = window_metrics.size;

3. Save to state.json
   └─> persisted.save();
       → ~/Library/Application Support/arto/state.json

4. Update in-memory LAST_FOCUSED_STATE global
   └─> LAST_FOCUSED_STATE.write() updates window metrics
       → For next "new window" (used immediately if creating windows)
```

## Decision Matrix

**When adding new settings, decide:**

| Question | Config | PersistedState | State |
|----------|--------|----------------|-------|
| Should user be able to edit it? | ✓ | ✗ | ✗ |
| Should it persist between app launches? | ✓ | ✓ | ✗ |
| Is it different per window? | ✗ | ✗ | ✓ |
| Should it restore on startup? | ✓ (default) | ✓ (last used) | ✗ |

**Examples:**

- **Default theme** → Config (user sets preference, applies to all windows)
- **Last used theme** → PersistedState (app remembers, restores on startup)
- **Current theme** → State (per-window, might differ during session)
- **Open tabs** → State only (never saved)
- **Sidebar width** → Config (default), PersistedState (last used), State (current)

## Common Patterns

### Reading a value on startup

```rust
// In window/settings.rs
pub fn get_theme_preference(is_first_window: bool) -> ThemePreference {
    let cfg = CONFIG.read();
    let theme = choose_by_behavior(
        is_first_window,
        cfg.theme.on_startup,
        cfg.theme.on_new_window,
        || cfg.theme.default_theme,
        || LAST_FOCUSED_STATE.read().theme,
    );
    ThemePreference { theme }
}
```

**Priority:**
1. Check config behavior setting (`on_startup` or `on_new_window`)
2. If "default" → use `config.theme.default_theme`
3. If "last_closed"/"last_focused" → use `LAST_FOCUSED_STATE.theme` directly

**Note:** `LAST_FOCUSED_STATE.theme` is always valid (initialized from `state.json` or defaults to `Theme::default()`)

### Updating a value during runtime

```rust
// In component (e.g., ThemeSelector)
let mut state = use_context::<AppState>();

// User changes theme - update Signal directly
state.current_theme.set(Theme::Dark);

// Sync to LAST_FOCUSED_STATE via use_effect
use_effect(move || {
    let theme = state.current_theme();
    LAST_FOCUSED_STATE.write().theme = theme;
});
```

**Pattern for updating state:**
1. Update `AppState.current_theme` Signal for current window UI
2. Use `use_effect` to automatically sync to `LAST_FOCUSED_STATE` for next "new window"

**Note:** AppState methods (like `set_root_directory`, `toggle_right_sidebar`) handle both updates internally for convenience

### Saving on window close

```rust
// In App component use_drop()
let mut persisted = PersistedState::from(&state);

// Capture window metrics
let window_metrics = crate::window::metrics::capture_window_metrics(&window().window);
persisted.window_position = window_metrics.position;
persisted.window_size = window_metrics.size;

// Update LAST_FOCUSED_STATE
{
    let mut last_focused = LAST_FOCUSED_STATE.write();
    last_focused.window_position = window_metrics.position;
    last_focused.window_size = window_metrics.size;
}

// Save to disk
persisted.save();
```

**What happens:**
1. Collect values from current `AppState` via `From<&AppState>` trait
2. Add window metrics (position, size)
3. Update `LAST_FOCUSED_STATE` global
4. Save to `state.json` (blocking operation)

## In-Memory Global: LAST_FOCUSED_STATE

**Consolidated global for "last_focused" behavior:**

```rust
// In state/persistence.rs
pub static LAST_FOCUSED_STATE: LazyLock<RwLock<PersistedState>> =
    LazyLock::new(|| RwLock::new(PersistedState::load()));
```

**Purpose:** Remember the last focused window's state for "new window" behavior

**Key characteristics:**
- Single `PersistedState` instance holding ALL last-focused values
- Updated by `AppState` methods when values change (theme, directory, sidebar, etc.)
- Updated in `use_drop()` when window closes (window metrics)
- Memory-only during app session, but initialized from disk (`state.json`)

**Differences from state.json:**
- **state.json** → Last **closed** window (persisted to disk on window close)
- **LAST_FOCUSED_STATE** → Last **focused** window (updated in real-time, saved on close)

**Why this design?**
- Startup uses `state.json` (most recent state before app quit)
- New window uses `LAST_FOCUSED_STATE` (current active window's state)
- Consolidated design eliminates multiple per-field globals

## Summary

| Aspect | Config | state.json | AppState | LAST_FOCUSED_STATE |
|--------|--------|-----------|----------|-------------------|
| **Type** | `Config` | `PersistedState` | `AppState` | `PersistedState` |
| **Storage** | config.json | state.json | Memory | Memory |
| **Scope** | App-wide | App-wide | Per-window | App-wide |
| **Lifetime** | Permanent | Permanent | Window | App session |
| **Edited by** | User | App | App | App |
| **Updated** | Manual/UI | On close | Real-time | Real-time |
| **Used for** | Defaults | Last closed | Current | Last focused |
| **Example field** | `default_theme` | `theme` | `current_theme` | `theme` |
| **Example value** | `Theme::Auto` | `Theme::Dark` | `Theme::Dark` | `Theme::Dark` |
