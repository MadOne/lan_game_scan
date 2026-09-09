use dioxus::prelude::*;

use crate::app_log::{AppLogEntry, AppLogLevel};

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

    let search_text = search.read().to_lowercase();

    let filtered = app_log_entries
        .read()
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

            if search_text.is_empty() {
                return true;
            }

            entry.message.to_lowercase().contains(&search_text)
                || entry.target.to_lowercase().contains(&search_text)
        })
        .cloned()
        .collect::<Vec<_>>();

    rsx! {
        div {
            class: "flex flex-col h-full w-full p-4 gap-3",

            // ============================================================
            // HEADER
            // ============================================================

            div {
                class: "flex items-center justify-between shrink-0",

                h1 {
                    class: "text-lg font-semibold text-zinc-100",
                    "Application Logs"
                }

                div {
                    class: "flex items-center gap-2",

                    button {
                        class: "px-3 py-1.5 rounded bg-zinc-800 hover:bg-zinc-700 text-sm text-zinc-300",

                        onclick: move |_| {
                            app_log_entries.write().clear();
                        },

                        "Clear"
                    }
                }
            }

            // ============================================================
            // FILTER / SEARCH
            // ============================================================

            div {
                class: "flex flex-col gap-2 shrink-0",

                input {
                    class: "w-full px-3 py-2 rounded bg-zinc-900 border border-zinc-700 text-zinc-200 placeholder-zinc-600 outline-none focus:border-zinc-500",
                    placeholder: "Search logs...",
                    value: "{search}",
                    oninput: move |event| search.set(event.value()),
                }

                div {
                    class: "flex items-center gap-4 flex-wrap text-xs text-zinc-400",

                    span {
                        class: "text-zinc-500",
                        "Levels:"
                    }

                    label {
                        class: "flex items-center gap-1.5 cursor-pointer",
                        input {
                            r#type: "checkbox",
                            checked: "{show_trace}",
                            onchange: move |event| show_trace.set(event.checked()),
                        }
                        "TRACE"
                    }

                    label {
                        class: "flex items-center gap-1.5 cursor-pointer",
                        input {
                            r#type: "checkbox",
                            checked: "{show_debug}",
                            onchange: move |event| show_debug.set(event.checked()),
                        }
                        "DEBUG"
                    }

                    label {
                        class: "flex items-center gap-1.5 cursor-pointer",
                        input {
                            r#type: "checkbox",
                            checked: "{show_info}",
                            onchange: move |event| show_info.set(event.checked()),
                        }
                        "INFO"
                    }

                    label {
                        class: "flex items-center gap-1.5 cursor-pointer",
                        input {
                            r#type: "checkbox",
                            checked: "{show_warn}",
                            onchange: move |event| show_warn.set(event.checked()),
                        }
                        "WARN"
                    }

                    label {
                        class: "flex items-center gap-1.5 cursor-pointer",
                        input {
                            r#type: "checkbox",
                            checked: "{show_error}",
                            onchange: move |event| show_error.set(event.checked()),
                        }
                        "ERROR"
                    }

                    div {
                        class: "h-4 w-px bg-zinc-800"
                    }

                    span {
                        class: "text-zinc-500",
                        "Display:"
                    }

                    label {
                        class: "flex items-center gap-1.5 cursor-pointer",
                        input {
                            r#type: "checkbox",
                            checked: "{show_timestamp}",
                            onchange: move |event| show_timestamp.set(event.checked()),
                        }
                        "Timestamp"
                    }

                    label {
                        class: "flex items-center gap-1.5 cursor-pointer",
                        input {
                            r#type: "checkbox",
                            checked: "{compact_target}",
                            onchange: move |event| compact_target.set(event.checked()),
                        }
                        "Compact target"
                    }
                }
            }

            // ============================================================
            // LOG OUTPUT
            // ============================================================

            div {
                class: "flex-1 min-h-0 overflow-auto rounded border border-zinc-800 bg-zinc-950 font-mono text-xs",

                for entry in filtered {
                    LogEntry {
                        entry,
                        show_timestamp: *show_timestamp.read(),
                        compact_target: *compact_target.read(),
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

    rsx! {
        div {
            class: "flex items-start gap-3 px-3 py-1.5 border-b border-zinc-900 hover:bg-zinc-900/70",

            if show_timestamp {
                span {
                    class: "shrink-0 text-zinc-600 whitespace-nowrap",
                    "{entry.formatted_time()}"
                }
            }

            span {
                class: "w-14 shrink-0 font-semibold {level_class}",
                "{entry.level:?}"
            }

            span {
                class: "shrink-0 text-zinc-400 whitespace-nowrap",
                title: "{entry.target}\n{module_path}\n{location}",
                "{target}"
            }

            span {
                class: "min-w-0 flex-1 text-zinc-300 whitespace-pre-wrap break-all",

                title: "Target: {entry.target}\nModule: {module_path}\nLocation: {location}\nTimestamp: {entry.formatted_time()}",

                "{entry.message}"
            }
        }
    }
}
