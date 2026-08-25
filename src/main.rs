// -----------------------------------------------------------------------------
// main.rs
// -----------------------------------------------------------------------------

#![allow(non_snake_case)]

mod app;
mod network;
//mod components;
mod custom_components;
mod misc;
pub mod scanner;
mod state;

use crate::app::{App, ShutdownSignal};
use crate::scanner::*;
use crate::server::GameServer;

use dioxus::desktop::tao::window::Icon;
use dioxus::desktop::{tao, Config, LogicalSize, WindowBuilder, WindowCloseBehaviour};
use dioxus::prelude::*;

use std::sync::Arc;
use tokio::sync::Notify;

// --- ROUTING ---

#[derive(PartialEq, Clone, Copy)]
enum TableMode {
    Lan,
    Fav,
}

fn main() {
    let icon_bytes = include_bytes!("../assets/icon.png");

    let icon = image::load_from_memory(icon_bytes)
        .map(|img| {
            let rgba = img.to_rgba8();
            let (width, height) = rgba.dimensions();

            Icon::from_rgba(rgba.into_raw(), width, height).unwrap()
        })
        .ok();

    let window = WindowBuilder::new()
        .with_title("LAN GAME SCAN")
        .with_inner_size(LogicalSize::new(1200.0, 800.0))
        .with_window_icon(icon);

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
                println!("[SHUTDOWN] Window close requested");

                shutdown_handler.notify_one();
            }
        });

    LaunchBuilder::desktop()
        .with_context(ShutdownSignal(shutdown))
        .with_cfg(config)
        .launch(App);
}
