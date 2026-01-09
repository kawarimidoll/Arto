use dioxus::document;
use dioxus::prelude::*;

use crate::components::icon::{Icon, IconName};
use crate::components::pinned_chip::PinnedChip;
use crate::state::AppState;

/// JavaScript to trigger search from input value
const JS_FIND: &str = r#"
    const input = document.querySelector('.search-input');
    if (input) window.Arto.search.find(input.value);
"#;

/// JavaScript to clear search input and highlights
const JS_CLEAR: &str = r#"
    const input = document.querySelector('.search-input');
    if (input) {
        input.value = '';
        input.focus();
    }
    window.Arto.search.clear();
"#;

/// Navigate to next or previous match
fn navigate(direction: &'static str) {
    spawn(async move {
        let js = format!("window.Arto.search.navigate('{direction}')");
        let _ = document::eval(&js).await;
    });
}

#[component]
pub fn SearchBar() -> Element {
    let mut state = use_context::<AppState>();
    let is_open = *state.search_open.read();
    let match_count = *state.search_match_count.read();
    let current_index = *state.search_current_index.read();
    let initial_text = state.search_initial_text.read().clone();
    let pinned_searches = state.pinned_searches.read().clone();
    let mut current_query = use_signal(|| String::new());

    // Handle initial text when search bar opens
    use_effect(use_reactive!(|is_open, initial_text| {
        if is_open {
            if let Some(ref text) = initial_text {
                if !text.is_empty() {
                    current_query.set(text.clone());
                    // Use JSON encoding to safely escape the string for JavaScript
                    let json_encoded = serde_json::to_string(text).unwrap_or_default();
                    let js = format!(
                        r#"
                        const input = document.querySelector('.search-input');
                        if (input) {{
                            input.value = {};
                            input.focus();
                            input.select();
                            window.Arto.search.find(input.value);
                        }}
                        "#,
                        json_encoded
                    );
                    spawn(async move {
                        let _ = document::eval(&js).await;
                    });
                }
                // Clear the initial text after using it
                state.search_initial_text.set(None);
            }
        }
    }));

    // Pin current search query
    let handle_pin = move |_| {
        let query = current_query.read().clone();
        if !query.is_empty() {
            tracing::debug!(pattern = %query, "Pinning search query");
            state.add_pinned_search(query, false);

            // Clear the query after pinning
            current_query.set(String::new());
            state.update_search_results(0, 0);
            spawn(async move {
                let _ = document::eval(JS_CLEAR).await;
            });
        } else {
            tracing::warn!("Query is empty, not pinning");
        }
    };

    rsx! {
        div {
            class: if is_open { "search-bar search-bar--open" } else { "search-bar" },

            // First row: search input and controls
            div {
                class: "search-bar-row",

                Icon { name: IconName::Search, size: 16 }

                // Input wrapper for positioning clear button
                div {
                    class: "search-input-wrapper",

                    // Uncontrolled input to preserve IME (SKK, Japanese input) state
                    input {
                        r#type: "text",
                        class: "search-input",
                        placeholder: "Search...",
                        autofocus: true,
                        autocorrect: "off",
                        autocapitalize: "off",
                        spellcheck: "false",
                        oninput: move |evt| {
                            current_query.set(evt.value());
                            spawn(async move {
                                let _ = document::eval(JS_FIND).await;
                            });
                        },
                        onkeydown: move |evt| {
                            match evt.key() {
                                Key::Enter => {
                                    let direction = if evt.modifiers().shift() { "prev" } else { "next" };
                                    navigate(direction);
                                }
                                Key::Escape => state.toggle_search(),
                                _ => {}
                            }
                        },
                    }

                    // Clear button (only shown when there's input)
                    if !current_query.read().is_empty() {
                        button {
                            class: "search-clear-button",
                            title: "Clear",
                            onclick: move |_| {
                                current_query.set(String::new());
                                state.update_search_results(0, 0);
                                spawn(async move {
                                    let _ = document::eval(JS_CLEAR).await;
                                });
                            },
                            Icon { name: IconName::Close, size: 14 }
                        }
                    }
                }

                // Pin button
                button {
                    class: "search-pin-button",
                    disabled: current_query.read().is_empty(),
                    title: "Pin this search",
                    onclick: handle_pin,
                    Icon { name: IconName::Pin, size: 16 }
                }

                button {
                    class: "search-nav-button",
                    disabled: match_count == 0,
                    title: "Previous match (Shift+Enter)",
                    onclick: move |_| navigate("prev"),
                    Icon { name: IconName::ChevronUp, size: 16 }
                }

                button {
                    class: "search-nav-button",
                    disabled: match_count == 0,
                    title: "Next match (Enter)",
                    onclick: move |_| navigate("next"),
                    Icon { name: IconName::ChevronDown, size: 16 }
                }

                span {
                    class: "search-match-count",
                    "{current_index}/{match_count}"
                }

                button {
                    class: "search-close-button",
                    title: "Close (Escape)",
                    onclick: move |_| state.toggle_search(),
                    Icon { name: IconName::Close, size: 16 }
                }
            }

            // Second row: pinned searches (only shown if there are any)
            if !pinned_searches.is_empty() {
                div {
                    class: "search-bar-pinned",
                    span { class: "search-bar-pinned-label", "Pinned:" }
                    for (i, pinned) in pinned_searches.iter().enumerate() {
                        {
                            let id = pinned.id.clone();
                            let id_for_color = id.clone();
                            let id_for_toggle = id.clone();
                            let id_for_remove = id.clone();

                            rsx! {
                                PinnedChip {
                                    key: "{i}",
                                    id: id.clone(),
                                    pattern: pinned.pattern.clone(),
                                    color: pinned.color,
                                    disabled: pinned.disabled,
                                    on_change_color: move |color| {
                                        state.update_pinned_search_color(id_for_color.clone(), color);
                                    },
                                    on_toggle_disabled: move |_| {
                                        state.toggle_pinned_search(id_for_toggle.clone());
                                    },
                                    on_remove: move |_| {
                                        state.remove_pinned_search(id_for_remove.clone());
                                    },
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
