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

    let sessions = state.rcon_sessions.read();

    let mut session_list: Vec<SocketAddr> = sessions.keys().copied().collect();

    session_list.sort_by_key(|addr| addr.to_string());

    let sel = state.selected_rcon.read();

    let active_val = sel
        .as_ref()
        .map(SocketAddr::to_string)
        .unwrap_or_else(|| "overview".to_string());

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
                // TAB BAR
                // =========================================================

                TabList {
                    class: "
                        shrink-0
                        flex
                        items-center
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
                                .and_then(|server| server.hostname.clone())
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

                                            state.rcon_sessions.with_mut(|sessions| {
                                                sessions.remove(&addr_val);
                                            });

                                            if state.selected_rcon.read().as_ref()
                                                == Some(&addr_val)
                                            {
                                                state.selected_rcon.set(None);
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
