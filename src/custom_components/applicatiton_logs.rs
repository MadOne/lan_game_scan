use std::collections::BTreeSet;

use dioxus::prelude::*;

use crate::app_log::{app_log_store, AppLogEntry, AppLogLevel};

const MAIN_CRATES: [&str; 3] = ["lan_game_scan", "cbz_rcon", "live_log"];

fn is_crate_selected(target: &str, selected_crates: &[String]) -> bool {
    let crate_name = target.split("::").next().unwrap_or(target);

    if MAIN_CRATES.contains(&crate_name) {
        selected_crates
            .iter()
            .any(|selected| selected == crate_name)
    } else {
        selected_crates.iter().any(|selected| selected == "Rest")
    }
}

#[component]
pub fn ApplicationLogs() -> Element {
    let mut app_log_entries = use_context::<Signal<Vec<AppLogEntry>>>();

    let mut search = use_signal(String::new);

    let mut show_trace = use_signal(|| true);
    let mut show_debug = use_signal(|| true);
    let mut show_info = use_signal(|| true);
    let mut show_warn = use_signal(|| true);
    let mut show_error = use_signal(|| true);

    let mut show_timestamp = use_signal(|| true);
    let mut compact_target = use_signal(|| true);

    let mut selected_crates = use_signal(|| {
        MAIN_CRATES
            .iter()
            .map(|name| name.to_string())
            .chain(std::iter::once("Rest".to_string()))
            .collect::<Vec<_>>()
    });

    let mut selected_target = use_signal(String::new);

    let entries = app_log_entries.read();

    let search_text = search.read().to_lowercase();
    let selected_crates_value = selected_crates.read().clone();
    let selected_target_value = selected_target.read().clone();

    let targets = entries
        .iter()
        .filter(|entry| is_crate_selected(&entry.target, &selected_crates_value))
        .map(|entry| entry.target.clone())
        .collect::<BTreeSet<_>>();

    let targets_for_effect = targets.clone();

    use_effect(move || {
        let selected = selected_target.read().clone();

        if !selected.is_empty() && !targets_for_effect.contains(&selected) {
            selected_target.set(String::new());
        }
    });

    let filtered = entries
        .iter()
        .filter(|entry| {
            let level_enabled = match entry.level {
                AppLogLevel::Trace => *show_trace.read(),
                AppLogLevel::Debug => *show_debug.read(),
                AppLogLevel::Info => *show_info.read(),
                AppLogLevel::Warn => *show_warn.read(),
                AppLogLevel::Error => *show_error.read(),
            };

            if !level_enabled {
                return false;
            }

            if !is_crate_selected(&entry.target, &selected_crates_value) {
                return false;
            }

            if !selected_target_value.is_empty() && entry.target != selected_target_value {
                return false;
            }

            if search_text.is_empty() {
                return true;
            }

            entry.message.to_lowercase().contains(&search_text)
                || entry.target.to_lowercase().contains(&search_text)
        })
        .cloned()
        .collect::<Vec<_>>();

    let filtered_is_empty = filtered.is_empty();

    let mut toggle_crate = move |crate_name: String| {
        let mut selected = selected_crates.write();

        if selected.iter().any(|value| value == &crate_name) {
            selected.retain(|value| value != &crate_name);
        } else {
            selected.push(crate_name);
        }
    };

    rsx! {
        div {
            class: "flex flex-col h-full min-h-0 gap-3",

            div {
                class: "flex items-center justify-between shrink-0",

                h2 {
                    class: "text-lg font-semibold text-zinc-200",
                    "Application Logs"
                }

                button {
                    class: "px-3 py-1.5 rounded bg-zinc-800 hover:bg-zinc-700 text-zinc-300 text-sm",
                    onclick: move |_| {
                        app_log_store().clear();
                        app_log_entries.write().clear();
                    },
                    "Clear"
                }
            }

            div {
                class: "shrink-0 flex flex-col gap-3 p-3 rounded border border-zinc-800 bg-zinc-950",

                div {
                    class: "flex items-center gap-3",

                    span {
                        class: "w-20 shrink-0 text-xs font-semibold uppercase text-zinc-500",
                        "Search"
                    }

                    input {
                        class: "flex-1 min-w-0 px-3 py-1.5 rounded bg-zinc-900 border border-zinc-800 text-sm text-zinc-300 outline-none focus:border-zinc-600",
                        r#type: "text",
                        placeholder: "Search message or target...",
                        value: "{search}",
                        oninput: move |event| {
                            search.set(event.value());
                        },
                    }
                }

                div {
                    class: "flex items-center gap-3",

                    span {
                        class: "w-20 shrink-0 text-xs font-semibold uppercase text-zinc-500",
                        "Levels"
                    }

                    label {
                        class: "flex items-center gap-1.5 text-xs text-zinc-500 cursor-pointer",

                        input {
                            r#type: "checkbox",
                            checked: show_trace,
                            onchange: move |event| {
                                show_trace.set(event.checked());
                            },
                        }

                        "Trace"
                    }

                    label {
                        class: "flex items-center gap-1.5 text-xs text-blue-400 cursor-pointer",

                        input {
                            r#type: "checkbox",
                            checked: show_debug,
                            onchange: move |event| {
                                show_debug.set(event.checked());
                            },
                        }

                        "Debug"
                    }

                    label {
                        class: "flex items-center gap-1.5 text-xs text-emerald-400 cursor-pointer",

                        input {
                            r#type: "checkbox",
                            checked: show_info,
                            onchange: move |event| {
                                show_info.set(event.checked());
                            },
                        }

                        "Info"
                    }

                    label {
                        class: "flex items-center gap-1.5 text-xs text-yellow-400 cursor-pointer",

                        input {
                            r#type: "checkbox",
                            checked: show_warn,
                            onchange: move |event| {
                                show_warn.set(event.checked());
                            },
                        }

                        "Warn"
                    }

                    label {
                        class: "flex items-center gap-1.5 text-xs text-red-400 cursor-pointer",

                        input {
                            r#type: "checkbox",
                            checked: show_error,
                            onchange: move |event| {
                                show_error.set(event.checked());
                            },
                        }

                        "Error"
                    }
                }

                div {
                    class: "flex items-center gap-3",

                    span {
                        class: "w-20 shrink-0 text-xs font-semibold uppercase text-zinc-500",
                        "Crates"
                    }

                    for crate_name in MAIN_CRATES {
                        {
                            let crate_name = crate_name.to_string();
                            let checked = selected_crates_value
                                .iter()
                                .any(|selected| selected == &crate_name);

                            rsx! {
                                label {
                                    class: "flex items-center gap-1.5 text-xs text-zinc-400 cursor-pointer",

                                    input {
                                        r#type: "checkbox",
                                        checked,
                                        onchange: {
                                            let crate_name = crate_name.clone();

                                            move |_| {
                                                toggle_crate(crate_name.clone());
                                            }
                                        },
                                    }

                                    "{crate_name}"
                                }
                            }
                        }
                    }

                    label {
                        class: "flex items-center gap-1.5 text-xs text-zinc-400 cursor-pointer",

                        input {
                            r#type: "checkbox",
                            checked: selected_crates_value
                                .iter()
                                .any(|selected| selected == "Rest"),
                            onchange: move |_| {
                                toggle_crate("Rest".to_string());
                            },
                        }

                        "Rest"
                    }
                }

                div {
                    class: "flex items-center gap-3",

                    span {
                        class: "w-20 shrink-0 text-xs font-semibold uppercase text-zinc-500",
                        "Target"
                    }

                    select {
                        class: "min-w-0 flex-1 max-w-2xl px-3 py-1.5 rounded bg-zinc-900 text-zinc-300 border border-zinc-800 outline-none focus:border-zinc-600",
                        style: "color-scheme: dark;",
                        value: "{selected_target}",
                        onchange: move |event| {
                            selected_target.set(event.value());
                        },

                        option {
                            value: "",
                            "All"
                        }

                        for target in targets.iter() {
                            option {
                                value: "{target}",
                                "{target}"
                            }
                        }
                    }
                }

                div {
                    class: "flex items-center gap-3",

                    span {
                        class: "w-20 shrink-0 text-xs font-semibold uppercase text-zinc-500",
                        "Display"
                    }

                    label {
                        class: "flex items-center gap-1.5 text-xs text-zinc-400 cursor-pointer",

                        input {
                            r#type: "checkbox",
                            checked: show_timestamp,
                            onchange: move |event| {
                                show_timestamp.set(event.checked());
                            },
                        }

                        "Timestamp"
                    }

                    label {
                        class: "flex items-center gap-1.5 text-xs text-zinc-400 cursor-pointer",

                        input {
                            r#type: "checkbox",
                            checked: compact_target,
                            onchange: move |event| {
                                compact_target.set(event.checked());
                            },
                        }

                        "Compact target"
                    }
                }
            }

            div {
                class: "flex-1 min-h-0 overflow-y-auto rounded border border-zinc-800 bg-black font-mono text-xs",

                for entry in filtered.iter() {
                    LogEntry {
                        key: "{entry.timestamp:?}-{entry.message}",
                        entry: entry.clone(),
                        show_timestamp: *show_timestamp.read(),
                        compact_target: *compact_target.read(),
                    }
                }

                if filtered_is_empty {
                    div {
                        class: "flex items-center justify-center h-full text-zinc-600",
                        "No log entries"
                    }
                }
            }
        }
    }
}

#[component]
fn LogEntry(entry: AppLogEntry, show_timestamp: bool, compact_target: bool) -> Element {
    let level_class = match entry.level {
        AppLogLevel::Trace => "text-zinc-500",
        AppLogLevel::Debug => "text-blue-400",
        AppLogLevel::Info => "text-emerald-400",
        AppLogLevel::Warn => "text-yellow-400",
        AppLogLevel::Error => "text-red-400",
    };

    let target = entry.display_target(compact_target);

    let location = match (&entry.file, entry.line) {
        (Some(file), Some(line)) => format!("{file}:{line}"),
        (Some(file), None) => file.clone(),
        (None, Some(line)) => format!("line {line}"),
        (None, None) => "unknown".to_string(),
    };

    let module_path = entry.module_path.as_deref().unwrap_or("unknown");

    let tooltip = format!(
        "Target: {}\nModule: {}\nLocation: {}\nTimestamp: {}",
        entry.target,
        module_path,
        location,
        entry.formatted_time(),
    );

    rsx! {
        div {
            class: "flex items-start gap-3 px-3 py-1.5 border-b border-zinc-900 hover:bg-zinc-900/70",

            if show_timestamp {
                span {
                    class: "shrink-0 w-28 text-zinc-600 whitespace-nowrap",
                    "{entry.formatted_time()}"
                }
            }

            span {
                class: "w-14 shrink-0 font-semibold {level_class}",
                "{entry.level:?}"
            }

            span {
                class: "w-80 shrink-0 px-2 py-0.5 rounded bg-zinc-900 text-zinc-300 truncate",
                title: "{tooltip}",
                "{target}"
            }

            span {
                class: "min-w-0 flex-1 text-zinc-300 whitespace-pre-wrap break-all",
                title: "{tooltip}",
                "{entry.message}"
            }
        }
    }
}
