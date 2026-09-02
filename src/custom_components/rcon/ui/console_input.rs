use crate::custom_components::cvar::{Cvar, CvarDatabase, CvarFlag};
use dioxus::prelude::*;
use std::collections::HashSet;

// =============================================================================
// CONSTANTS
// =============================================================================

const MAX_COMMAND_HISTORY: usize = 100;

// =============================================================================
// RCON COMMAND INPUT
// =============================================================================

#[component]
#[component]
pub fn RconCommandInput(
    cvar_db: Signal<Option<CvarDatabase>>,
    cvar_filters: Signal<HashSet<CvarFlag>>,
    command_history: Signal<Vec<String>>,
    on_command: EventHandler<String>,
) -> Element {
    let mut cmd_input = use_signal(String::new);
    let mut suggestions = use_signal(Vec::<Cvar>::new);
    let mut suggestion_index = use_signal(|| None::<usize>);
    let mut history_index = use_signal(|| None::<usize>);

    rsx! {
        div {
            class: "relative shrink-0 border-t border-zinc-800 bg-zinc-900 p-3",

            // =================================================================
            // AUTOCOMPLETE POPUP
            // =================================================================

            if !suggestions.read().is_empty() {
                div {
                    class: "absolute z-50 bottom-full left-3 right-3 mb-1 bg-zinc-950 border border-zinc-700 rounded-lg shadow-2xl overflow-visible",

                    div {
                        class: "px-3 py-2 border-b border-zinc-800 text-[9px] font-black tracking-widest text-zinc-600",

                        "COMMANDS"
                    }

                    // ---------------------------------------------------------
                    // CVAR SUGGESTIONS
                    // ---------------------------------------------------------

                    for (index, suggestion) in suggestions.read().iter().enumerate() {
                        {
                            let command = suggestion.name.clone();
                            let value = suggestion.value.clone();
                            let description = suggestion.description.clone();

                            let flags = suggestion
                                .flags
                                .iter()
                                .map(|flag| format!("{flag:?}"))
                                .collect::<Vec<_>>()
                                .join(", ");

                            let selected = suggestion_index() == Some(index);

                            rsx! {
                                div {
                                    class: if selected {
                                        "relative group bg-zinc-800"
                                    } else {
                                        "relative group"
                                    },

                                    button {
                                        key: "{command}",

                                        class: "w-full text-left px-3 py-2 text-[10px] text-zinc-400 hover:bg-zinc-800 hover:text-indigo-300 transition-colors",

                                        onclick: {
                                            let command = command.clone();

                                            move |_| {
                                                cmd_input.set(command.clone());
                                                suggestions.set(Vec::new());
                                                suggestion_index.set(None);
                                                history_index.set(None);
                                            }
                                        },

                                        div {
                                            class: "flex items-center gap-2",

                                            span {
                                                class: "text-indigo-300 shrink-0",
                                                "{command}"
                                            }

                                            span {
                                                class: "text-zinc-500 truncate",
                                                "{value}"
                                            }

                                            span {
                                                class: "ml-auto text-zinc-600 shrink-0",
                                                "[{flags}]"
                                            }
                                        }
                                    }

                                    // -------------------------------------------------
                                    // DESCRIPTION TOOLTIP
                                    // -------------------------------------------------

                                    div {
                                        class: "absolute z-[100] left-3 bottom-full mb-1 hidden group-hover:block w-[calc(100%-24px)] rounded-lg border border-zinc-700 bg-zinc-900 p-3 text-left shadow-2xl pointer-events-none",

                                        div {
                                            class: "text-[10px] font-bold text-indigo-300 mb-1",
                                            "{command}"
                                        }

                                        div {
                                            class: "text-[10px] text-zinc-400 whitespace-normal",
                                            "{description}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // =================================================================
            // COMMAND INPUT ROW
            // =================================================================

            div {
                class: "flex gap-2",

                span {
                    class: "text-indigo-500 font-bold py-2",
                    ">"
                }

                input {
                    class: "flex-1 bg-black border border-zinc-700 rounded px-3 py-2 text-zinc-200 outline-none focus:border-indigo-500",

                    placeholder: "Enter RCON command...",

                    value: "{cmd_input}",

                    // ---------------------------------------------------------
                    // INPUT / AUTOCOMPLETE
                    // ---------------------------------------------------------

                    oninput: move |event| {
                        let input = event.value();

                        cmd_input.set(input.clone());
                        history_index.set(None);

                        let query = input
                            .split_whitespace()
                            .next()
                            .unwrap_or("")
                            .to_string();

                        if query.is_empty()
                            || input.contains(char::is_whitespace)
                        {
                            suggestions.set(Vec::new());
                            suggestion_index.set(None);
                            return;
                        }

                        let db = cvar_db.read();

                        let Some(db) = db.as_ref() else {
                            suggestions.set(Vec::new());
                            suggestion_index.set(None);
                            return;
                        };

                        let filters = cvar_filters.read();

                        let results = db.get_suggestions(&query, &filters);

                        suggestion_index.set(None);
                        suggestions.set(results);
                    },

                    // ---------------------------------------------------------
                    // KEYBOARD HANDLING
                    // ---------------------------------------------------------

                    onkeydown: move |event| {
                        let key = event.key();

                        // =====================================================
                        // ESCAPE
                        // =====================================================

                        if key == Key::Escape {
                            suggestions.set(Vec::new());
                            suggestion_index.set(None);
                            return;
                        }

                        // =====================================================
                        // AUTOCOMPLETE NAVIGATION
                        // =====================================================

                        if !suggestions.read().is_empty() {
                            let count = suggestions.read().len();

                            if key == Key::ArrowUp {
                                let new_index = match suggestion_index() {
                                    None => count - 1,

                                    Some(index) if index == 0 => {
                                        count - 1
                                    }

                                    Some(index) => {
                                        index - 1
                                    }
                                };

                                suggestion_index.set(Some(new_index));
                                return;
                            }

                            if key == Key::ArrowDown {
                                let new_index = match suggestion_index() {
                                    None => 0,

                                    Some(index) if index + 1 >= count => {
                                        0
                                    }

                                    Some(index) => {
                                        index + 1
                                    }
                                };

                                suggestion_index.set(Some(new_index));
                                return;
                            }

                            if key == Key::Enter {
                                if let Some(index) = suggestion_index() {
                                    let command = suggestions
                                        .read()
                                        .get(index)
                                        .map(|suggestion| suggestion.name.clone());

                                    if let Some(command) = command {
                                        cmd_input.set(command);
                                        suggestions.set(Vec::new());
                                        suggestion_index.set(None);
                                        history_index.set(None);

                                        return;
                                    }
                                }
                            }
                        }

                        // =====================================================
                        // COMMAND HISTORY
                        // =====================================================

                        if key == Key::ArrowUp {
                            let history = command_history.read();

                            if history.is_empty() {
                                return;
                            }

                            let new_index = match history_index() {
                                None => {
                                    // First ArrowUp:
                                    // jump to newest command.
                                    history.len() - 1
                                }

                                Some(index) if index > 0 => {
                                    index - 1
                                }

                                Some(index) => {
                                    // Already at oldest command.
                                    index
                                }
                            };

                            if let Some(command) = history.get(new_index) {
                                cmd_input.set(command.clone());
                                history_index.set(Some(new_index));
                            }

                            return;
                        }

                        if key == Key::ArrowDown {
                            let history = command_history.read();

                            if history.is_empty() {
                                return;
                            }

                            let Some(current) = history_index() else {
                                return;
                            };

                            if current + 1 < history.len() {
                                let new_index = current + 1;

                                if let Some(command) = history.get(new_index) {
                                    cmd_input.set(command.clone());
                                    history_index.set(Some(new_index));
                                }
                            } else {
                                // Move past newest history entry.
                                cmd_input.set(String::new());
                                history_index.set(None);
                            }

                            return;
                        }

                        // =====================================================
                        // ENTER -> EXECUTE COMMAND
                        // =====================================================

                        if key == Key::Enter {
                            let cmd = cmd_input();

                            if cmd.trim().is_empty() {
                                return;
                            }

                            let cmd = cmd.trim().to_string();

                            cmd_input.set(String::new());
                            suggestions.set(Vec::new());
                            suggestion_index.set(None);
                            history_index.set(None);

                            add_to_command_history(&mut command_history, &cmd);

                            on_command.call(cmd);
                        }
                    }
                }

                // =============================================================
                // SEND BUTTON
                // =============================================================

                button {
                    class: "bg-indigo-600 hover:bg-indigo-500 text-white px-5 py-2 rounded font-bold",

                    onclick: move |_| {
                        let cmd = cmd_input();

                        if cmd.trim().is_empty() {
                            return;
                        }

                        let cmd = cmd.trim().to_string();

                        cmd_input.set(String::new());
                        suggestions.set(Vec::new());
                        suggestion_index.set(None);
                        history_index.set(None);

                        add_to_command_history(&mut command_history, &cmd);

                        on_command.call(cmd);
                    },

                    "SEND"
                }
            }
        }
    }
}

// =============================================================================
// COMMAND HISTORY
// =============================================================================

fn add_to_command_history(command_history: &mut Signal<Vec<String>>, command: &str) {
    command_history.with_mut(|history| {
        // Don't add the same command twice in a row.
        if history.last().is_some_and(|last| last == command) {
            return;
        }

        history.push(command.to_string());

        // Keep only the newest MAX_COMMAND_HISTORY entries.
        if history.len() > MAX_COMMAND_HISTORY {
            let excess = history.len() - MAX_COMMAND_HISTORY;
            history.drain(0..excess);
        }
    });
}
