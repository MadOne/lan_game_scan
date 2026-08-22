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

use crate::scanner::*;
use crate::server::GameServer;

use app::App;
use dioxus::desktop::tao::window::Icon;
use dioxus::desktop::{Config, LogicalSize, WindowBuilder};
use dioxus::prelude::*;

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

    LaunchBuilder::desktop()
        .with_cfg(Config::new().with_window(window))
        .launch(App);
}
