mod assets;
mod bookmarks;
mod components;
mod config;
mod drag;
mod events;
mod history;
mod ipc;
mod markdown;
mod menu;
mod pinned_search;
mod state;
mod theme;
mod utils;
mod watcher;
mod window;

use clap::Parser;
use dioxus::desktop::tao::event::{Event, WindowEvent};
use std::path::PathBuf;
use tokio::sync::mpsc::channel;
use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::prelude::*;

/// Arto - A markdown viewer
#[derive(Parser, Debug)]
#[command(version, about)]
struct Cli {
    /// Files or directories to open
    #[arg()]
    paths: Vec<PathBuf>,
}

const DEFAULT_LOGLEVEL: &str = if cfg!(debug_assertions) {
    "debug"
} else {
    "info"
};

fn main() {
    // Parse CLI arguments first (before any other initialization)
    let cli = Cli::parse();

    // Try to send paths to existing instance via IPC
    // If successful, exit immediately without initializing anything else
    if let ipc::SendResult::Sent = ipc::try_send_to_existing_instance(&cli.paths) {
        std::process::exit(0);
    }

    // Load environment variables from .env file
    if let Ok(dotenv) = dotenvy::dotenv() {
        println!("Loaded .env file from: {}", dotenv.display());
    }
    init_tracing();

    // Create event channel and store receiver for MainApp
    let (tx, rx) = channel::<components::main_app::OpenEvent>(10);
    components::main_app::OPEN_EVENT_RECEIVER
        .lock()
        .expect("Failed to lock OPEN_EVENT_RECEIVER")
        .replace(rx);

    // Start IPC server to accept connections from future instances
    ipc::start_ipc_server(tx.clone());

    // Send CLI paths as OpenEvents (before Dioxus launches)
    for path in cli.paths {
        // Canonicalize to resolve symlinks and get absolute paths
        let path = path.canonicalize().unwrap_or(path);
        let event = if path.is_dir() {
            components::main_app::OpenEvent::Directory(path)
        } else if path.is_file() {
            components::main_app::OpenEvent::File(path)
        } else {
            tracing::warn!(?path, "Skipping invalid path");
            continue;
        };
        tracing::debug!(?event, "Sending CLI path as open event");
        tx.try_send(event).expect("Failed to send CLI path event");
    }

    let menu = menu::build_menu();

    // Get window parameters for first window from preferences
    let params = window::CreateMainWindowConfigParams::from_preferences(true);

    let config = window::create_main_window_config(&params)
        .with_custom_event_handler(move |event, _target| match event {
            Event::Opened { urls, .. } => {
                for url in urls {
                    if let Ok(path) = url.to_file_path() {
                        let open_event = if path.is_dir() {
                            components::main_app::OpenEvent::Directory(path)
                        } else if path.is_file() {
                            components::main_app::OpenEvent::File(path)
                        } else {
                            // Skip invalid paths
                            continue;
                        };
                        tx.try_send(open_event).expect("Failed to send open event");
                    }
                }
            }
            Event::Reopen { .. } => {
                // Send reopen event through channel to handle it safely in component context
                tx.try_send(components::main_app::OpenEvent::Reopen).ok();
            }
            Event::WindowEvent {
                event: WindowEvent::Focused(true),
                window_id,
                ..
            } => {
                // Skip updating LAST_FOCUSED_WINDOW while a preview window exists
                // to prevent focus from jumping to wrong window during drag.
                // This blocks all focus updates during drag, not just when the
                // preview window itself gains focus.
                if !window::has_preview_window() {
                    window::update_last_focused_window(*window_id);
                }
            }
            _ => {}
        })
        .with_menu(menu);

    // Launch MainApp (first window only)
    // Initial event will be consumed inside MainApp after Dioxus starts
    dioxus::LaunchBuilder::desktop()
        .with_cfg(config)
        .launch(components::main_app::MainApp);
}

fn init_tracing() {
    let silence_filter = tracing_subscriber::filter::filter_fn(|metadata| {
        // Filter out specific error from dioxus_core::properties:136
        // Known issue: https://github.com/DioxusLabs/dioxus/issues/3872
        metadata.target() != "dioxus_core::properties::__component_called_as_function"
    });

    let env_filter_layer =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(DEFAULT_LOGLEVEL));

    let fmt_layer = tracing_subscriber::fmt::layer()
        .pretty()
        .without_time()
        .with_target(false)
        .with_thread_ids(false)
        .with_file(true)
        .with_line_number(true)
        .with_filter(silence_filter.clone());

    let registry = tracing_subscriber::registry()
        .with(env_filter_layer)
        .with(fmt_layer);

    // On macOS, log to Console.app via oslog
    let registry = registry.with(
        tracing_oslog::OsLogger::new("com.lambdalisue.Arto", "default").with_filter(silence_filter),
    );

    registry.init();
}
