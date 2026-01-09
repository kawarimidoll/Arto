use dioxus::prelude::*;

use super::RightSidebarTab;

#[component]
pub fn TabBar(active_tab: RightSidebarTab, on_change: EventHandler<RightSidebarTab>) -> Element {
    rsx! {
        div {
            class: "right-sidebar-tabs",

            // Contents tab
            button {
                class: if active_tab == RightSidebarTab::Contents { "right-sidebar-tab active" } else { "right-sidebar-tab" },
                onclick: move |_| on_change.call(RightSidebarTab::Contents),
                span { "Contents" }
            }

            // Search tab
            button {
                class: if active_tab == RightSidebarTab::Search { "right-sidebar-tab active" } else { "right-sidebar-tab" },
                onclick: move |_| on_change.call(RightSidebarTab::Search),
                span { "Search" }
            }
        }
    }
}
