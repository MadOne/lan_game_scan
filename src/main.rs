mod app;
mod app_log;
mod custom_components;
mod misc;
mod network;
mod rcon_manager;
mod state;

use crate::app::{App, ShutdownSignal};
use dioxus::desktop::tao::window::Icon;
use dioxus::desktop::{tao, Config, LogicalSize, WindowBuilder, WindowCloseBehaviour};
use dioxus::prelude::*;
use tracing_subscriber::filter::Targets;
use tracing_subscriber::prelude::*;

use std::sync::Arc;
use tokio::sync::Notify;
#[derive(PartialEq, Clone, Copy)]
enum TableMode {
    Lan,
    Fav,
}
fn main() {
    // -------------------------------------------------------------------------
    // LOGGING
    // -------------------------------------------------------------------------

    let app_log = app_log::init_app_log();

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_ansi(true))
        .with(app_log::AppLogLayer::new(app_log))
        .with(
            Targets::new()
                .with_default(tracing::Level::DEBUG)
                .with_target("cbz_rcon", tracing::Level::TRACE)
                .with_target("live_log", tracing::Level::TRACE)
                .with_target("lan_game_scan", tracing::Level::TRACE),
        )
        .init();

    // -------------------------------------------------------------------------
    // WINDOW
    // -------------------------------------------------------------------------

    let icon_bytes = include_bytes!("../assets/icon.png");

    let icon = match image::load_from_memory(icon_bytes) {
        Ok(img) => {
            let rgba = img.to_rgba8();
            let (width, height) = rgba.dimensions();

            match Icon::from_rgba(rgba.into_raw(), width, height) {
                Ok(icon) => Some(icon),
                Err(error) => {
                    tracing::error!("[UI] Failed to create application icon: {}", error);
                    None
                }
            }
        }

        Err(error) => {
            tracing::error!("[UI] Failed to load application icon: {}", error);
            None
        }
    };

    let window = WindowBuilder::new()
        .with_title("LAN GAME SCAN")
        .with_inner_size(LogicalSize::new(360.0, 800.0))
        .with_window_icon(icon);

    // -------------------------------------------------------------------------
    // SHUTDOWN
    // -------------------------------------------------------------------------

    let shutdown = Arc::new(Notify::new());
    let shutdown_handler = shutdown.clone();

    let config = Config::new()
        .with_window(window)
        .with_close_behaviour(WindowCloseBehaviour::WindowHides)
        .with_custom_event_handler(move |event, _event_loop| {
            if let tao::event::Event::WindowEvent {
                event: tao::event::WindowEvent::CloseRequested,
                ..
            } = event
            {
                tracing::debug!("Window close requested");

                shutdown_handler.notify_one();
            }
        });

    // -------------------------------------------------------------------------
    // LAUNCH
    // -------------------------------------------------------------------------

    LaunchBuilder::desktop()
        .with_context(ShutdownSignal(shutdown))
        .with_cfg(config)
        .launch(App);
}
