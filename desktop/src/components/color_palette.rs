use dioxus::prelude::*;

use crate::components::icon::{Icon, IconName};
use crate::pinned_search::HighlightColor;

#[component]
pub fn ColorPalette(
    current_color: HighlightColor,
    on_select: EventHandler<HighlightColor>,
    on_remove: EventHandler<()>,
    on_toggle_disabled: EventHandler<()>,
    disabled: bool,
    on_close: EventHandler<()>,
) -> Element {
    rsx! {
        // Backdrop for outside click
        div {
            class: "color-palette-backdrop",
            onclick: move |_| on_close.call(()),
        }

        div {
            class: "color-palette-popover",

            // Color swatches
            div {
                class: "color-palette-colors",
                for color in HighlightColor::ALL {
                    button {
                        class: if color == current_color {
                            format!("color-palette-swatch selected {}", color.css_class())
                        } else {
                            format!("color-palette-swatch {}", color.css_class())
                        },
                        title: "{color:?}",
                        onclick: move |_| {
                            on_select.call(color);
                            on_close.call(());
                        },
                    }
                }
            }

            // Disabled toggle
            button {
                class: "color-palette-toggle",
                onclick: move |_| {
                    on_toggle_disabled.call(());
                    on_close.call(());
                },
                input {
                    r#type: "checkbox",
                    checked: disabled,
                    onclick: move |evt| evt.stop_propagation(),
                }
                span { "Disabled" }
            }

            // Remove button
            button {
                class: "color-palette-remove",
                onclick: move |_| {
                    on_remove.call(());
                    on_close.call(());
                },
                Icon { name: IconName::Trash, size: 14 }
                span { "Remove" }
            }
        }
    }
}
