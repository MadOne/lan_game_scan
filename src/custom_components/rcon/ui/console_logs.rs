use crate::custom_components::rcon::code::RconLogEvent;
use crate::custom_components::ui::pretty_log;
use dioxus::prelude::*;
use live_log::parser::LogType;
use std::collections::HashSet;

#[component]
pub fn RconLogOutput(
    logs: Signal<Vec<RconLogEvent>>,
    selected_events: HashSet<LogType>,
) -> Element {
    rsx! {
        div {
            class: "flex-1 min-h-0 overflow-y-auto overflow-x-hidden p-6 space-y-1 scrollbar-thin",

            for event in logs.read().iter() {
                {
                    match event {
                        RconLogEvent::RconResponse(text) => rsx! {
                            div {
                                class: "whitespace-pre-wrap break-words text-zinc-300",
                                "{text}"
                            }

                            div {
                                class: "h-2"
                            }
                        },

                        RconLogEvent::Info(text) => rsx! {
                            div {
                                class: "whitespace-pre-wrap break-words text-zinc-500",
                                "{text}"
                            }

                            div {
                                class: "h-2"
                            }
                        },

                        RconLogEvent::LiveLog(parsed) => {
                            if selected_events.contains(&parsed.log_type) {
                                if parsed.log_type == LogType::Unknown {
                                    rsx! {
                                        div {
                                            class: "whitespace-pre-wrap break-words text-zinc-300",
                                            "{parsed.raw}"
                                        }

                                        div {
                                            class: "h-2"
                                        }
                                    }
                                } else {
                                    rsx! {
                                        div {
                                            class: "whitespace-pre-wrap break-words",
                                            {pretty_log(&parsed.event)}
                                        }

                                        div {
                                            class: "h-2"
                                        }
                                    }
                                }
                            } else {
                                rsx! {}
                            }
                        }
                    }
                }
            }
        }
    }
}
