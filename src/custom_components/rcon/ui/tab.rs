use super::console::RconConsole;
use super::rcon_dashboard::RconDashboard;
use crate::state::AppState;
use dioxus::prelude::*;
use dioxus_primitives::tabs::{TabContent, TabList, TabTrigger, Tabs};
use std::net::SocketAddr;

#[derive(PartialEq, Clone, Copy)]
pub enum RconSubTab {
    Overview,
    Terminal,
    CreateConfig,
}

#[component]
pub fn RconTab() -> Element {
    let mut state = use_context::<AppState>();

    let mut mobile_popup_open = use_signal(|| false);

    let sessions = state.rcon_sessions.read();

    let mut session_list: Vec<SocketAddr> = sessions.keys().copied().collect();

    session_list.sort_by_key(|addr| addr.to_string());

    let sel = state.selected_rcon.read();

    let active_val = sel
        .as_ref()
        .map(SocketAddr::to_string)
        .unwrap_or_else(|| "overview".to_string());

    // =========================================================
    // MOBILE ACTIVE SERVER NAME
    // =========================================================

    let mobile_active_name = if active_val == "overview" {
        "RCON OVERVIEW".to_string()
    } else {
        active_val
            .parse::<SocketAddr>()
            .ok()
            .and_then(|addr| {
                state
                    .servers
                    .read()
                    .get(&addr)
                    .and_then(|server| server.scanned.hostname.clone())
            })
            .unwrap_or_else(|| active_val.clone())
    };

    // =========================================================
    // MOBILE CLOSE SESSION
    // =========================================================

    let mut close_session = {
        let mut state = state;

        move |addr: SocketAddr| {
            let session = state
                .rcon_sessions
                .with_mut(|sessions| sessions.remove(&addr));

            if state.selected_rcon.read().as_ref() == Some(&addr) {
                state.selected_rcon.set(None);
            }

            if let Some(mut session) = session {
                spawn(async move {
                    if !session.close().await {
                        eprintln!("Failed to clean up RCON session for {}", addr);
                    }
                });
            }
        }
    };

    rsx! {
        div {
            class: "flex flex-col h-full bg-zinc-950",

            Tabs {
                class: "flex flex-col h-full min-h-0",

                value: active_val.clone(),

                on_value_change: move |value: String| {
                    if value == "overview" {
                        state.selected_rcon.set(None);
                    } else if let Ok(addr) = value.parse::<SocketAddr>() {
                        state.selected_rcon.set(Some(addr));
                    }
                },

                // =========================================================
                // MOBILE SERVER SELECTOR
                // =========================================================

                div {
                    class: "md:hidden relative shrink-0 bg-zinc-900 border-b border-zinc-800 p-2",

                    // -----------------------------------------------------
                    // SELECTOR BUTTON
                    // -----------------------------------------------------

                    button {
                        class: "w-full flex items-center justify-between gap-3 bg-zinc-950 border border-zinc-700 hover:border-zinc-600 rounded-lg px-3 py-2.5 text-left transition-colors",

                        onclick: move |_| {
                            let open = mobile_popup_open();
                            mobile_popup_open.set(!open);
                        },

                        div {
                            class: "flex items-center gap-2 min-w-0",

                            if active_val != "overview" {
                                span {
                                    class: "w-2 h-2 shrink-0 rounded-full bg-emerald-500",
                                }
                            } else {
                                span {
                                    class: "w-2 h-2 shrink-0 rounded-full bg-indigo-500",
                                }
                            }

                            span {
                                class: "truncate text-[10px] font-black uppercase tracking-widest text-zinc-300",

                                "{mobile_active_name}"
                            }
                        }

                        span {
                            class: if mobile_popup_open() {
                                "text-indigo-400 text-xs transition-transform rotate-180"
                            } else {
                                "text-zinc-500 text-xs transition-transform"
                            },

                            "▼"
                        }
                    }

                    // =====================================================
                    // MOBILE SESSION POPUP
                    // =====================================================

                    if mobile_popup_open() {
                        div {
                            class: "absolute z-50 left-2 right-2 top-full mt-2 bg-zinc-950 border border-zinc-700 rounded-lg shadow-2xl overflow-hidden",

                            // -------------------------------------------------
                            // POPUP HEADER
                            // -------------------------------------------------

                            div {
                                class: "px-3 py-2.5 border-b border-zinc-800 flex items-center justify-between",

                                span {
                                    class: "text-[9px] font-black tracking-widest text-zinc-500 uppercase",

                                    "RCON SESSIONS"
                                }

                                button {
                                    class: "text-zinc-600 hover:text-zinc-300 text-sm px-1",

                                    onclick: move |_| {
                                        mobile_popup_open.set(false);
                                    },

                                    "×"
                                }
                            }

                            // -------------------------------------------------
                            // OVERVIEW ENTRY
                            // -------------------------------------------------

                            button {
                                class: "w-full flex items-center gap-3 px-3 py-3 text-left border-b border-zinc-800 hover:bg-zinc-900 transition-colors",

                                onclick: move |_| {
                                    state.selected_rcon.set(None);
                                    mobile_popup_open.set(false);
                                },

                                span {
                                    class: if active_val == "overview" {
                                        "w-2 h-2 rounded-full bg-indigo-500 shrink-0"
                                    } else {
                                        "w-2 h-2 rounded-full bg-zinc-700 shrink-0"
                                    },
                                }

                                span {
                                    class: if active_val == "overview" {
                                        "flex-1 text-[10px] font-black uppercase tracking-widest text-white"
                                    } else {
                                        "flex-1 text-[10px] font-black uppercase tracking-widest text-zinc-500"
                                    },

                                    "RCON OVERVIEW"
                                }

                                if active_val == "overview" {
                                    span {
                                        class: "text-indigo-400 text-xs",
                                        "✓"
                                    }
                                }
                            }

                            // -------------------------------------------------
                            // SERVER ENTRIES
                            // -------------------------------------------------

                            div {
                                class: "max-h-72 overflow-y-auto scrollbar-thin",

                                for addr in session_list.iter() {
                                    {
                                        let addr_val = *addr;
                                        let addr_str = addr_val.to_string();

                                        let is_active =
                                            active_val == addr_str;

                                        let hostname = state
                                            .servers
                                            .read()
                                            .get(&addr_val)
                                            .and_then(|server| {
                                                server.scanned.hostname.clone()
                                            })
                                            .unwrap_or_else(|| {
                                                addr_str.clone()
                                            });

                                        rsx! {
                                            div {
                                                key: "mobile-{addr_str}",

                                                class: if is_active {
                                                    "flex items-center bg-indigo-950/40 border-b border-zinc-800"
                                                } else {
                                                    "flex items-center bg-zinc-950 border-b border-zinc-800 hover:bg-zinc-900"
                                                },

                                                // ---------------------------------
                                                // SERVER SELECT
                                                // ---------------------------------

                                                button {
                                                    class: "flex-1 min-w-0 flex items-center gap-3 px-3 py-3 text-left",

                                                    onclick: move |_| {
                                                        state.selected_rcon
                                                            .set(Some(addr_val));

                                                        mobile_popup_open
                                                            .set(false);
                                                    },

                                                    span {
                                                        class: if is_active {
                                                            "w-2 h-2 rounded-full bg-emerald-500 shrink-0"
                                                        } else {
                                                            "w-2 h-2 rounded-full bg-zinc-700 shrink-0"
                                                        },
                                                    }

                                                    span {
                                                        class: if is_active {
                                                            "truncate text-[10px] font-black uppercase tracking-widest text-white"
                                                        } else {
                                                            "truncate text-[10px] font-black uppercase tracking-widest text-zinc-400"
                                                        },

                                                        "{hostname}"
                                                    }

                                                    if is_active {
                                                        span {
                                                            class: "text-indigo-400 text-xs ml-auto shrink-0",
                                                            "✓"
                                                        }
                                                    }
                                                }

                                                // ---------------------------------
                                                // CLOSE SESSION
                                                // ---------------------------------

                                                button {
                                                    class: "shrink-0 px-4 py-3 text-zinc-600 hover:text-red-400 hover:bg-red-950/30 transition-colors text-lg",

                                                    onclick: move |event| {
                                                        event.stop_propagation();

                                                        close_session(
                                                            addr_val
                                                        );

                                                        mobile_popup_open
                                                            .set(false);
                                                    },

                                                    title: "Close RCON session",

                                                    "×"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // =========================================================
                // DESKTOP TAB BAR
                // =========================================================

                TabList {
                    class: "
                        hidden
                        md:flex
                        shrink-0
                        items-center
                        overflow-x-auto
                        scrollbar-thin
                        border-b
                        border-zinc-800
                        bg-zinc-900
                        px-4
                    ",

                    // =====================================================
                    // OVERVIEW
                    // =====================================================

                    TabTrigger {
                        value: "overview",
                        index: 0usize,

                        class: {
                            if active_val == "overview" {
                                "px-4 py-3 text-[10px] font-black transition-all border-b-2 -mb-px border-indigo-500 text-white bg-zinc-950 uppercase tracking-tighter"
                            } else {
                                "px-4 py-3 text-[10px] font-black transition-all border-b-2 -mb-px border-transparent text-zinc-500 hover:text-zinc-300 uppercase tracking-tighter"
                            }
                        },

                        "OVERVIEW"
                    }

                    // =====================================================
                    // SERVER TABS
                    // =====================================================

                    for (idx, addr) in session_list.iter().enumerate() {
                        {
                            let addr_val = *addr;
                            let addr_str = addr_val.to_string();

                            let is_active = active_val == addr_str;

                            let tab_class = if is_active {
                                "border-indigo-500 text-white bg-zinc-950"
                            } else {
                                "border-transparent text-zinc-500 hover:text-zinc-300"
                            };

                            let hostname = state
                                .servers
                                .read()
                                .get(&addr_val)
                                .and_then(|server| server.scanned.hostname.clone())
                                .unwrap_or_else(|| addr_str.clone());

                            rsx! {
                                TabTrigger {
                                    key: "{addr_str}",

                                    value: addr_str.clone(),

                                    // Overview occupies index 0
                                    index: idx + 1,

                                    class: "px-4 py-3 text-[10px] font-black transition-all border-b-2 -mb-px flex items-center gap-2 {tab_class} uppercase tracking-tighter",

                                    span {
                                        class: "truncate max-w-[140px]",
                                        "{hostname}"
                                    }

                                    button {
                                        class: "hover:text-red-500 opacity-30 hover:opacity-100 ml-2 text-lg",

                                        onclick: move |event| {
                                            event.stop_propagation();

                                            let session = state
                                                .rcon_sessions
                                                .with_mut(|sessions| {
                                                    sessions.remove(&addr_val)
                                                });

                                            if state.selected_rcon.read().as_ref()
                                                == Some(&addr_val)
                                            {
                                                state.selected_rcon.set(None);
                                            }

                                            if let Some(mut session) = session {
                                                spawn(async move {
                                                    if !session.close().await {
                                                        eprintln!(
                                                            "Failed to clean up RCON session for {}",
                                                            addr_val
                                                        );
                                                    }
                                                });
                                            }
                                        },

                                        "×"
                                    }
                                }
                            }
                        }
                    }
                }

                // =========================================================
                // TAB CONTENT
                // =========================================================

                div {
                    class: "flex-1 min-h-0 overflow-hidden",

                    // =====================================================
                    // OVERVIEW CONTENT
                    // =====================================================

                    TabContent {
                        value: "overview",
                        index: 0usize,
                        class: "h-full min-h-0",

                        RconDashboard {}
                    }

                    // =====================================================
                    // INDIVIDUAL SERVER CONTENT
                    // =====================================================

                    for (idx, addr) in session_list.iter().enumerate() {
                        {
                            let addr_val = *addr;
                            let addr_str = addr_val.to_string();

                            rsx! {
                                TabContent {
                                    key: "content-{addr_str}",

                                    value: addr_str,

                                    // Overview occupies index 0
                                    index: idx + 1,

                                    class: "h-full min-h-0",

                                    RconConsole {
                                        addr: addr_val
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
