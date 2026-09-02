use crate::custom_components::cvar::CvarFlag;
use dioxus::prelude::*;
use std::collections::HashSet;

// =============================================================================
// CVAR FILTERS
// =============================================================================
//
// User-controlled filters for RCON command suggestions.
//
// These filters are optional exclusions. Fundamental RCON validity is still
// handled by CvarDatabase::is_rcon_valid().
//
// Default:
// - MenuBarItem
// - VConsoleFuzzy
// - VConsoleSetFocus
// - DevelopmentOnly
// =============================================================================

#[component]
pub fn CvarFilters(filters: Signal<HashSet<CvarFlag>>, popup_open: Signal<bool>) -> Element {
    let categories = [
        (
            "CORE",
            vec![
                (CvarFlag::Server, "Server"),
                (CvarFlag::Client, "Client"),
                (CvarFlag::Release, "Release"),
                (CvarFlag::Replicated, "Replicated"),
                (CvarFlag::Cheat, "Cheat"),
            ],
        ),
        (
            "USER / CONFIG",
            vec![
                (CvarFlag::Archive, "Archive"),
                (CvarFlag::Notify, "Notify"),
                (CvarFlag::User, "User"),
                (CvarFlag::PerUser, "Per User"),
                (CvarFlag::Demo, "Demo"),
            ],
        ),
        (
            "SECURITY",
            vec![
                (CvarFlag::Protected, "Protected"),
                (CvarFlag::ServerCantQuery, "Server Can't Query"),
                (CvarFlag::ServerCanExecute, "Server Can Execute"),
                (CvarFlag::ClientCanExecute, "Client Can Execute"),
                (CvarFlag::NoRecord, "No Record"),
                (CvarFlag::Defensive, "Defensive"),
            ],
        ),
        (
            "PLUGINS",
            vec![(CvarFlag::Linked, "Linked"), (CvarFlag::Special, "Special")],
        ),
        (
            "DEVELOPER / INTERNAL",
            vec![
                (CvarFlag::MenuBarItem, "Menu Bar Item"),
                (CvarFlag::VConsoleFuzzy, "VConsole Fuzzy"),
                (CvarFlag::VConsoleSetFocus, "VConsole Set Focus"),
                (CvarFlag::DevelopmentOnly, "Development Only"),
                (CvarFlag::Hidden, "Hidden"),
            ],
        ),
    ];

    let active_count = filters.read().len();

    rsx! {
        div {
            class: "relative",

            button {
                class: "px-3 py-1.5 text-[9px] font-black tracking-widest border border-zinc-700 rounded bg-zinc-900 text-zinc-500 hover:text-zinc-300 hover:border-zinc-600 transition-colors",

                onclick: move |_| {
                    popup_open.set(!popup_open());
                },

                "CVAR FILTERS"

                if active_count > 0 {
                    span {
                        class: "ml-2 text-indigo-400",
                        "{active_count}"
                    }
                }
            }

            if popup_open() {
                div {
                    class: "absolute z-50 top-full left-0 mt-2 w-80 bg-zinc-950 border border-zinc-700 rounded-lg shadow-2xl",

                    // =========================================================
                    // HEADER
                    // =========================================================

                    div {
                        class: "flex items-center justify-between px-3 py-2 border-b border-zinc-800",

                        span {
                            class: "text-[9px] font-black tracking-widest text-zinc-500",
                            "CVAR FILTERS"
                        }

                        button {
                            class: "text-[9px] text-zinc-600 hover:text-zinc-300",

                            onclick: move |_| {
                                filters.write().clear();
                            },

                            "CLEAR"
                        }
                    }

                    // =========================================================
                    // FILTERS
                    // =========================================================

                    div {
                        class: "p-3 max-h-96 overflow-y-auto space-y-4",

                        for (category, entries) in categories.iter() {
                            div {
                                key: "{category}",

                                div {
                                    class: "mb-2 text-[8px] font-black tracking-widest text-zinc-700",
                                    "{category}"
                                }

                                div {
                                    class: "space-y-1",

                                    for (flag, label) in entries.iter() {
                                        {
                                            let flag = *flag;
                                            let label = *label;
                                            let checked = filters.read().contains(&flag);

                                            rsx! {
                                                button {
                                                    key: "{label}",

                                                    class: "w-full flex items-center gap-2 px-2 py-1.5 rounded text-left hover:bg-zinc-900 transition-colors",

                                                    onclick: move |_| {
                                                        let mut filters = filters.write();

                                                        if filters.contains(&flag) {
                                                            filters.remove(&flag);
                                                        } else {
                                                            filters.insert(flag);
                                                        }
                                                    },

                                                    div {
                                                        class: if checked {
                                                            "w-3 h-3 rounded-sm border border-indigo-500 bg-indigo-500 flex items-center justify-center"
                                                        } else {
                                                            "w-3 h-3 rounded-sm border border-zinc-700 bg-black"
                                                        },

                                                        if checked {
                                                            span {
                                                                class: "text-[8px] text-white font-black",
                                                                "✓"
                                                            }
                                                        }
                                                    }

                                                    span {
                                                        class: if checked {
                                                            "text-[10px] text-zinc-300"
                                                        } else {
                                                            "text-[10px] text-zinc-600"
                                                        },

                                                        "{label}"
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
                    // FOOTER
                    // =========================================================

                    div {
                        class: "px-3 py-2 border-t border-zinc-800 text-[8px] text-zinc-700",

                        "Selected flags are hidden from command suggestions."
                    }
                }
            }
        }
    }
}
