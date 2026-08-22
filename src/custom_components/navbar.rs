use dioxus::prelude::*;
use dioxus::router::use_route;

use crate::app::Route;
use crate::custom_components::rcon::ui::attention::RconAttentionIndicator;
use crate::state::AppState;

#[component]
pub fn Navbar() -> Element {
    let state = use_context::<AppState>();

    let rcon_count = state.rcon_sessions.read().len();

    let route: Route = use_route();

    let base = "px-4 py-2 rounded-md text-sm font-medium transition-all flex items-center gap-2";

    let lan_cls = if matches!(route, Route::LAN {}) {
        format!("{} bg-zinc-800 text-white shadow-inner", base)
    } else {
        format!("{} text-zinc-500 hover:text-zinc-300", base)
    };

    let fav_cls = if matches!(route, Route::Favourites {}) {
        format!("{} bg-zinc-800 text-white shadow-inner", base)
    } else {
        format!("{} text-zinc-500 hover:text-zinc-300", base)
    };

    let rcon_cls = if matches!(route, Route::RconTab {}) {
        format!("{} bg-zinc-800 text-white shadow-inner", base)
    } else {
        format!("{} text-zinc-500 hover:text-zinc-300", base)
    };

    rsx! {
        div {
            class: "relative flex flex-col h-full",

            // ========================================================
            // HEADER
            // ========================================================

            header {
                class: "flex items-center justify-between px-6 py-4 bg-zinc-900 border-b border-zinc-800 shadow-2xl z-30",

                div {
                    class: "flex items-center gap-3",

                    div {
                        class: "w-8 h-8 bg-indigo-600 rounded flex items-center justify-center text-white font-black shadow-lg",
                        "L"
                    }

                    h1 {
                        class: "text-lg font-bold tracking-tighter text-white uppercase",
                        "LAN GAME SCAN"
                    }
                }

                nav {
                    class: "flex items-center gap-1",

                    Link {
                        to: Route::LAN {},
                        class: "{lan_cls}",
                        "📡 LAN"
                    }

                    Link {
                        to: Route::Favourites {},
                        class: "{fav_cls}",
                        "⭐ Favourites"
                    }

                    Link {
                        to: Route::RconTab {},
                        class: "{rcon_cls}",

                        span {
                            "⌨ RCON"
                        }

                        if rcon_count > 0 {
                            span {
                                class: "bg-indigo-600 text-[10px] px-1.5 py-0.5 rounded-full text-white animate-pulse",
                                "{rcon_count}"
                            }
                        }
                    }
                }
            }

            // ========================================================
            // CONTENT
            // ========================================================

            main {
                class: "flex-1 overflow-hidden",

                Outlet::<Route> {}
            }

            // ========================================================
            // GLOBAL RCON ATTENTION
            // ========================================================

            RconAttentionIndicator {}
        }
    }
}
