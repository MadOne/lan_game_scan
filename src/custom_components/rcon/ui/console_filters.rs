use dioxus::prelude::*;
use live_log::parser::LogType;
use std::collections::HashSet;

#[component]
pub fn ConsoleFilters(
    visible_events: Signal<HashSet<LogType>>,
    filter_popup_open: Signal<bool>,
) -> Element {
    let selected_events = visible_events.read().clone();

    let all_enabled = LogType::all().all(|event_type| selected_events.contains(&event_type));

    let none_enabled = selected_events.is_empty();

    let essential_events = [
        LogType::Chat,
        LogType::Connection,
        LogType::RoundWin,
        LogType::GameOver,
    ];

    let essential_enabled = essential_events
        .iter()
        .all(|event_type| selected_events.contains(event_type))
        && selected_events.len() == essential_events.len();

    let customize_enabled = !all_enabled && !none_enabled && !essential_enabled;

    rsx! {
        div {
            class: "relative shrink-0 bg-zinc-900/80 border-b border-zinc-800 px-4 py-2",

            div {
                class: "flex items-center gap-2",

                span {
                    class: "text-[9px] font-black tracking-widest text-zinc-600 mr-1",
                    "FILTER"
                }

                // =============================================================
                // ALL
                // =============================================================

                button {
                    class: if all_enabled {
                        "px-3 py-1 rounded text-[9px] font-black bg-indigo-600 text-white border border-indigo-500"
                    } else {
                        "px-3 py-1 rounded text-[9px] font-black bg-zinc-800 text-zinc-500 border border-zinc-700 hover:text-zinc-300"
                    },

                    onclick: move |_| {
                        visible_events.with_mut(|set| {
                            set.clear();
                            set.extend(LogType::all());
                        });

                        filter_popup_open.set(false);
                    },

                    "ALL"
                }

                // =============================================================
                // NONE
                // =============================================================

                button {
                    class: if none_enabled {
                        "px-3 py-1 rounded text-[9px] font-black bg-indigo-600 text-white border border-indigo-500"
                    } else {
                        "px-3 py-1 rounded text-[9px] font-black bg-zinc-800 text-zinc-500 border border-zinc-700 hover:text-zinc-300"
                    },

                    onclick: move |_| {
                        visible_events.with_mut(|set| {
                            set.clear();
                        });

                        filter_popup_open.set(false);
                    },

                    "NONE"
                }

                div {
                    class: "w-px h-4 bg-zinc-800 mx-1"
                }

                // =============================================================
                // ESSENTIAL PRESET
                // =============================================================

                button {
                    class: if essential_enabled {
                        "px-3 py-1 rounded text-[9px] font-black bg-indigo-600 text-white border border-indigo-500"
                    } else {
                        "px-3 py-1 rounded text-[9px] font-black bg-zinc-800 text-zinc-500 border border-zinc-700 hover:text-zinc-300"
                    },

                    onclick: move |_| {
                        visible_events.with_mut(|set| {
                            set.clear();

                            set.insert(LogType::Chat);
                            set.insert(LogType::Connection);
                            set.insert(LogType::RoundWin);
                            set.insert(LogType::GameOver);
                        });

                        filter_popup_open.set(false);
                    },

                    "ESSENTIAL"
                }

                // =============================================================
                // CUSTOMIZE
                // =============================================================

                button {
                    class: if filter_popup_open() || customize_enabled {
                        "px-3 py-1 rounded text-[9px] font-black bg-indigo-950 text-indigo-300 border border-indigo-700"
                    } else {
                        "px-3 py-1 rounded text-[9px] font-black bg-zinc-800 text-zinc-500 border border-zinc-700 hover:text-zinc-300"
                    },

                    onclick: move |_| {
                        let open = filter_popup_open();
                        filter_popup_open.set(!open);
                    },

                    "CUSTOMIZE"
                }

                // =============================================================
                // FILTER SUMMARY
                // =============================================================

                span {
                    class: "ml-auto text-[9px] text-zinc-600",

                    "{selected_events.len()}/{LogType::all().count()}"
                }
            }

            // =============================================================
            // CUSTOMIZE POPUP
            // =============================================================

            if filter_popup_open() {
                div {
                    class: "absolute z-50 top-full left-4 mt-2 w-80 bg-zinc-950 border border-zinc-700 rounded-lg shadow-2xl",

                    // ---------------------------------------------------------
                    // Popup header
                    // ---------------------------------------------------------

                    div {
                        class: "px-4 py-3 border-b border-zinc-800 flex items-center justify-between",

                        div {
                            class: "text-[10px] font-black tracking-widest text-zinc-300",
                            "CUSTOMIZE FILTER"
                        }

                        button {
                            class: "text-zinc-600 hover:text-zinc-300 text-sm px-1",

                            onclick: move |_| {
                                filter_popup_open.set(false);
                            },

                            "×"
                        }
                    }

                    // ---------------------------------------------------------
                    // Event list
                    // ---------------------------------------------------------

                    div {
                        class: "p-3 max-h-80 overflow-y-auto",

                        div {
                            class: "grid grid-cols-2 gap-1",

                            for event_type in LogType::all() {
                                {
                                    let enabled =
                                        selected_events.contains(&event_type);

                                    let label = event_type.label();

                                    let class = if enabled {
                                        "w-full px-3 py-2 rounded text-left text-[9px] font-black bg-indigo-950 text-indigo-300 border border-indigo-800 hover:bg-indigo-900 transition-colors"
                                    } else {
                                        "w-full px-3 py-2 rounded text-left text-[9px] font-black bg-zinc-900 text-zinc-600 border border-zinc-800 hover:text-zinc-400 hover:border-zinc-700 transition-colors"
                                    };

                                    rsx! {
                                        button {
                                            key: "{label}",
                                            class: "{class}",

                                            onclick: move |_| {
                                                visible_events.with_mut(|set| {
                                                    if set.contains(&event_type) {
                                                        set.remove(&event_type);
                                                    } else {
                                                        set.insert(event_type);
                                                    }
                                                });
                                            },

                                            div {
                                                class: "flex items-center gap-2",

                                                div {
                                                    class: if enabled {
                                                        "w-2 h-2 rounded-sm bg-indigo-500"
                                                    } else {
                                                        "w-2 h-2 rounded-sm border border-zinc-700"
                                                    }
                                                }

                                                "{label}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // ---------------------------------------------------------
                    // Popup footer
                    // ---------------------------------------------------------

                    div {
                        class: "px-3 py-2 border-t border-zinc-800 flex justify-between",

                        button {
                            class: "px-2 py-1 text-[9px] font-black text-zinc-600 hover:text-zinc-300",

                            onclick: move |_| {
                                visible_events.with_mut(|set| {
                                    set.clear();
                                });
                            },

                            "CLEAR"
                        }

                        button {
                            class: "px-2 py-1 text-[9px] font-black text-indigo-500 hover:text-indigo-300",

                            onclick: move |_| {
                                visible_events.with_mut(|set| {
                                    set.clear();
                                    set.extend(LogType::all());
                                });
                            },

                            "SELECT ALL"
                        }
                    }
                }
            }
        }
    }
}
