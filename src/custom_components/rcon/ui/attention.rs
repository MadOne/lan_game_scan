use dioxus::prelude::*;

use crate::{app::Route, state::AppState};

#[component]
pub fn RconAttentionIndicator() -> Element {
    let state = use_context::<AppState>();
    let nav = use_navigator();

    let attention_count = state
        .rcon_sessions
        .read()
        .values()
        .filter(|session| (session.need_attention)())
        .count();

    if attention_count == 0 {
        return rsx! {};
    }

    rsx! {
        div {
            class: "fixed inset-0 pointer-events-none z-[9999] border-2 border-red-500/80",

            div {
                class: "absolute top-3 right-3 pointer-events-auto",

                button {
                    class: "flex items-center gap-2 bg-red-600 hover:bg-red-500 text-white px-3 py-2 rounded-lg shadow-xl shadow-red-950/50 transition-all animate-pulse",

                    onclick: move |_| {
                        nav.push(Route::RconTab {});
                    },

                    span {
                        class: "flex items-center justify-center w-5 h-5 bg-white text-red-600 rounded-full text-[10px] font-black",
                        "{attention_count}"
                    }

                    span {
                        class: "text-[10px] font-black uppercase tracking-widest",
                        "RCON ATTENTION"
                    }
                }
            }
        }
    }
}
