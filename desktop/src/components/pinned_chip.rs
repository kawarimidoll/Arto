use dioxus::prelude::*;

use crate::components::color_palette::ColorPalette;
use crate::components::icon::{Icon, IconName};
use crate::pinned_search::{HighlightColor, PinnedSearchId};

#[component]
pub fn PinnedChip(
    id: PinnedSearchId,
    pattern: String,
    color: HighlightColor,
    disabled: bool,
    on_change_color: EventHandler<HighlightColor>,
    on_toggle_disabled: EventHandler<()>,
    on_remove: EventHandler<()>,
) -> Element {
    let mut show_palette = use_signal(|| false);

    let chip_class = if disabled {
        format!("pinned-chip pinned-chip--disabled {}", color.css_class())
    } else {
        format!("pinned-chip {}", color.css_class())
    };

    rsx! {
        div {
            class: "{chip_class}",

            // Chip body (clickable to show palette)
            button {
                class: "pinned-chip-body",
                onclick: move |_| show_palette.toggle(),

                span { class: "pinned-chip-pattern", "{pattern}" }
            }

            // Quick remove button
            button {
                class: "pinned-chip-remove",
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
                    current_color: color,
                    disabled: disabled,
                    on_select: move |c| on_change_color.call(c),
                    on_toggle_disabled: move |_| on_toggle_disabled.call(()),
                    on_remove: move |_| on_remove.call(()),
                    on_close: move |_| show_palette.set(false),
                }
            }
        }
    }
}
