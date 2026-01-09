use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use super::context_menu::ContextMenuData;
use super::context_menu_state::{open_context_menu, ContentContextMenuState};
use crate::markdown::render_to_html_with_toc;
use crate::state::{AppState, TabContent};
use crate::utils::file::is_markdown_file;
use crate::watcher::FILE_WATCHER;

/// Data structure for markdown link clicks from JavaScript
#[derive(Serialize, Deserialize)]
struct LinkClickData {
    path: String,
    button: u32,
}

/// Mouse button constants
const LEFT_CLICK: u32 = 0;
const MIDDLE_CLICK: u32 = 1;

#[component]
pub fn FileViewer(file: PathBuf) -> Element {
    let state = use_context::<AppState>();
    let html = use_signal(String::new);
    let reload_trigger = use_signal(|| 0usize);

    // Get base directory for link resolution
    let base_dir = file
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));

    // Setup component hooks
    use_file_loader(file.clone(), html, reload_trigger, state);
    use_file_watcher(file.clone(), reload_trigger);
    use_link_click_handler(file.clone(), state);
    use_mermaid_window_handler();
    use_context_menu_handler(file.clone(), base_dir);
    use_search_handler(state);

    rsx! {
        div {
            class: "markdown-viewer",
            article {
                class: "markdown-body",
                dangerous_inner_html: "{html}"
            }
            // Context menu is rendered at App level to avoid re-rendering content
        }
    }
}

/// Hook to load and render file content
fn use_file_loader(
    file: PathBuf,
    html: Signal<String>,
    reload_trigger: Signal<usize>,
    mut state: AppState,
) {
    use_effect(use_reactive!(|file, reload_trigger| {
        let mut html = html;
        let _ = reload_trigger();
        let file = file.clone();

        spawn(async move {
            tracing::info!("Loading and rendering file: {:?}", &file);

            // Try to read as string (UTF-8 text file)
            match tokio::fs::read_to_string(file.as_path()).await {
                Ok(content) => {
                    // Check if file has markdown extension
                    if is_markdown_file(&file) {
                        // Render as markdown with TOC heading extraction
                        match render_to_html_with_toc(&content, &file) {
                            Ok((rendered, headings)) => {
                                html.set(rendered);
                                state.toc_headings.set(headings);
                                tracing::trace!("Rendered as Markdown: {:?}", &file);
                            }
                            Err(e) => {
                                // Markdown parsing failed, render as plain text
                                tracing::warn!(
                                    "Markdown parsing failed for {:?}, rendering as plain text: {}",
                                    &file,
                                    e
                                );
                                let escaped_content = html_escape::encode_text(&content);
                                let plain_html = format!(
                                    r#"<pre class="plain-text-viewer">{}</pre>"#,
                                    escaped_content
                                );
                                html.set(plain_html);
                                state.toc_headings.set(Vec::new());
                            }
                        }
                    } else {
                        // Non-markdown file, render as plain text directly
                        tracing::info!("Rendering non-markdown file as plain text: {:?}", &file);
                        let escaped_content = html_escape::encode_text(&content);
                        let plain_html = format!(
                            r#"<pre class="plain-text-viewer">{}</pre>"#,
                            escaped_content
                        );
                        html.set(plain_html);
                        state.toc_headings.set(Vec::new());
                    }
                }
                Err(e) => {
                    // Failed to read as UTF-8 text (likely binary file)
                    tracing::error!("Failed to read file {:?} as text: {}", file, e);
                    let error_msg = format!("{:?}", e);

                    // Update tab content to FileError
                    let file_clone = file.clone();
                    state.update_current_tab(move |tab| {
                        tab.content = TabContent::FileError(file_clone, error_msg);
                    });
                    html.set(String::new());
                }
            }
        });
    }));
}

/// Hook to watch file for changes and trigger reload
fn use_file_watcher(file: PathBuf, reload_trigger: Signal<usize>) {
    use_effect(use_reactive!(|file| {
        let mut reload_trigger = reload_trigger;
        let file = file.clone();

        spawn(async move {
            let file_path = file.clone();
            let mut watcher = match FILE_WATCHER.watch(file_path.clone()).await {
                Ok(watcher) => watcher,
                Err(e) => {
                    tracing::error!(
                        "Failed to register file watcher for {:?}: {:?}",
                        file_path,
                        e
                    );
                    return;
                }
            };

            while watcher.recv().await.is_some() {
                tracing::info!("File change detected, reloading: {:?}", file_path);
                reload_trigger.set(reload_trigger() + 1);
            }

            if let Err(e) = FILE_WATCHER.unwatch(file_path.clone()).await {
                tracing::error!(
                    "Failed to unregister file watcher for {:?}: {:?}",
                    file_path,
                    e
                );
            }
        });
    }));
}

/// Hook to setup JavaScript handler for markdown link clicks
fn use_link_click_handler(file: PathBuf, state: AppState) {
    use_effect(use_reactive!(|file| {
        let file = file.clone();
        let mut eval_provider = document::eval(indoc::indoc! {r#"
            window.handleMarkdownLinkClick = (path, button) => {
                dioxus.send({ path, button });
            };
        "#});

        let base_dir = file
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));

        let mut state_clone = state;

        spawn(async move {
            while let Ok(click_data) = eval_provider.recv::<LinkClickData>().await {
                handle_link_click(click_data, &base_dir, &mut state_clone);
            }
        });
    }));
}

/// Handle a markdown link click event
fn handle_link_click(click_data: LinkClickData, base_dir: &Path, state: &mut AppState) {
    let LinkClickData { path, button } = click_data;

    tracing::info!("Markdown link clicked: {} (button: {})", path, button);

    // Resolve and normalize the path
    let target_path = base_dir.join(&path);
    let Ok(canonical_path) = target_path.canonicalize() else {
        tracing::error!("Failed to resolve path: {:?}", target_path);
        return;
    };

    tracing::info!("Opening file: {:?}", canonical_path);

    match button {
        MIDDLE_CLICK => {
            // Open in new tab (always create a new tab for middle-click)
            state.add_file_tab(canonical_path, true);
        }
        LEFT_CLICK => {
            // Navigate in current tab (in-tab navigation, no existing tab check)
            state.navigate_to_file(canonical_path);
        }
        _ => {
            tracing::debug!("Ignoring click with button: {}", button);
        }
    }
}

/// Hook to setup Mermaid window open handler
fn use_mermaid_window_handler() {
    use_effect(|| {
        let mut eval_provider = document::eval(indoc::indoc! {r#"
            window.handleMermaidWindowOpen = (source) => {
                dioxus.send({ type: "open_mermaid_window", source: source });
            };
        "#});

        spawn(async move {
            while let Ok(data) = eval_provider.recv::<serde_json::Value>().await {
                if let Some(msg_type) = data.get("type").and_then(|v| v.as_str()) {
                    if msg_type == "open_mermaid_window" {
                        if let Some(source) = data.get("source").and_then(|v| v.as_str()) {
                            let state = use_context::<AppState>();
                            let theme = *state.current_theme.read();
                            tracing::info!("Opening mermaid window for diagram");
                            crate::window::open_or_focus_mermaid_window(source.to_string(), theme);
                        }
                    }
                }
            }
        });
    });
}

/// Hook to setup context menu handler for right-clicks on content
///
/// Uses global state to avoid re-rendering FileViewer when menu state changes.
/// This preserves text selection in the content.
fn use_context_menu_handler(file: PathBuf, base_dir: PathBuf) {
    use_effect(use_reactive!(|file, base_dir| {
        let file = file.clone();
        let base_dir = base_dir.clone();

        // Setup JS context menu handler using the exported function
        let mut eval_provider = document::eval(indoc::indoc! {r#"
            // Setup context menu handler
            window.Arto.setupContextMenu((data) => {
                dioxus.send(data);
            });
        "#});

        spawn(async move {
            while let Ok(data) = eval_provider.recv::<ContextMenuData>().await {
                tracing::debug!(?data, "Context menu triggered");
                // Write to global state (doesn't subscribe FileViewer)
                open_context_menu(ContentContextMenuState {
                    data,
                    current_file: Some(file.clone()),
                    base_dir: base_dir.clone(),
                });
            }
        });
    }));
}

/// Data structure for search results from JavaScript
#[derive(Serialize, Deserialize)]
struct SearchResultData {
    count: usize,
    current: usize,
    matches: Vec<crate::search_match::MatchInfo>,
}

/// Hook to setup search result handler
fn use_search_handler(mut state: AppState) {
    use_effect(move || {
        // Setup JS search handler to receive match counts and match details
        let mut eval_provider = document::eval(indoc::indoc! {r#"
            window.Arto.search.setup((data) => {
                dioxus.send(data);
            });
        "#});

        spawn(async move {
            while let Ok(data) = eval_provider.recv::<SearchResultData>().await {
                state.update_search_results(data.count, data.current);
                state.update_search_matches(data.matches);
            }
        });
    });
}
