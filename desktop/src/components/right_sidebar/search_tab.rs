use dioxus::document;
use dioxus::prelude::*;

use crate::components::color_palette::ColorPalette;
use crate::components::icon::{Icon, IconName};
use crate::pinned_search::{HighlightColor, PinnedSearch};
use crate::state::AppState;

#[component]
pub fn SearchTab() -> Element {
    let mut state = use_context::<AppState>();
    let pinned = state.pinned_searches.read().clone();
    let all_matches = state.search_matches.read().clone();

    tracing::debug!(
        all_matches_count = all_matches.len(),
        pinned_count = pinned.len(),
        "Rendering SearchTab"
    );

    rsx! {
        div {
            class: "search-tab",

            // Matches list
            if !all_matches.is_empty() {
                div {
                    class: "matches-section",

                    h3 {
                        class: "right-sidebar-section-title",
                        "MATCHES ({all_matches.len()})"
                    }

                    ul {
                        class: "matches-list",
                        for (i, m) in all_matches.iter().enumerate() {
                            {
                                let pattern = m.pattern.clone();
                                let index = m.index;
                                let context = m.context.clone();
                                let color = m.color.clone();

                                rsx! {
                                    MatchItem {
                                        key: "{i}",
                                        pattern: pattern.clone(),
                                        index,
                                        context: context.clone(),
                                        color: color.clone(),
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Pinned searches management section
            if !pinned.is_empty() {
                div {
                    class: "pinned-section",

                    h3 {
                        class: "right-sidebar-section-title",
                        "PINNED"
                    }

                    ul {
                        class: "pinned-list",
                        for p in pinned.iter() {
                            {
                                let id = p.id.clone();
                                let id_for_color = id.clone();
                                let id_for_toggle = id.clone();
                                let id_for_remove = id.clone();

                                rsx! {
                                    PinnedItem {
                                        key: "{id}",
                                        pinned: p.clone(),
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

            // Empty state
            if all_matches.is_empty() && pinned.is_empty() {
                div {
                    class: "search-empty-state",
                    p { "No active search" }
                    p {
                        class: "search-empty-hint",
                        "Press ⌘F to search and click "
                        Icon { name: IconName::Pin, size: 12 }
                        " to pin"
                    }
                }
            }
        }
    }
}

#[component]
fn MatchItem(pattern: String, index: usize, context: String, color: String) -> Element {
    // Clone for use in both closure and rsx
    let pattern_for_display = pattern.clone();
    let context_for_display = context.clone();
    let color_for_display = color.clone();

    let handle_click = move |_| {
        let pattern = pattern.clone();
        spawn(async move {
            let js = format!(
                "window.Arto.search.scrollToMatch('{}', {})",
                pattern.replace('\'', "\\'"),
                index
            );
            let _ = document::eval(&js).await;
        });
    };

    rsx! {
        li {
            class: "match-item",
            onclick: handle_click,

            div {
                class: "match-item-header",
                span {
                    class: format!("match-color-indicator highlight-{}", color_for_display),
                }
                span {
                    class: "match-pattern",
                    "{pattern_for_display}"
                }
            }

            div {
                class: "match-context",
                "{context_for_display}"
            }
        }
    }
}

#[component]
fn PinnedItem(
    pinned: PinnedSearch,
    on_change_color: EventHandler<HighlightColor>,
    on_toggle_disabled: EventHandler<()>,
    on_remove: EventHandler<()>,
) -> Element {
    let mut show_palette = use_signal(|| false);

    rsx! {
        li {
            class: format!(
                "pinned-item {} {}",
                pinned.color.css_class(),
                if pinned.disabled { "disabled" } else { "" }
            ),

            button {
                class: "pinned-item-body",
                onclick: move |_| show_palette.toggle(),

                span {
                    class: format!("pinned-color-dot {}", pinned.color.css_class()),
                }

                span {
                    class: "pinned-pattern",
                    "{pinned.pattern}"
                }
            }

            button {
                class: "pinned-item-remove",
                title: "Remove",
                onclick: move |evt| {
                    evt.stop_propagation();
                    on_remove.call(());
                },
                Icon { name: IconName::Close, size: 12 }
            }

            // Color palette popover
            if *show_palette.read() {
                ColorPalette {
                    current_color: pinned.color,
                    disabled: pinned.disabled,
                    on_select: move |c| {
                        on_change_color.call(c);
                        show_palette.set(false);
                    },
                    on_toggle_disabled: move |_| {
                        on_toggle_disabled.call(());
                    },
                    on_remove: move |_| {
                        on_remove.call(());
                        show_palette.set(false);
                    },
                    on_close: move |_| show_palette.set(false),
                }
            }
        }
    }
}
