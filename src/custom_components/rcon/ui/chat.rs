use dioxus::prelude::*;
use live_log::parser::LogType;

use crate::custom_components::{code::RconLogEvent, ui::pretty_log};

#[component]
#[component]
pub fn RconChat(logs: Signal<Vec<RconLogEvent>>) -> Element {
    rsx! {
        div {
            class: "w-[360px] shrink-0 border-l border-zinc-800 bg-zinc-900/30 flex flex-col min-h-0",

            div {
                class: "shrink-0 px-4 py-3 border-b border-zinc-800",

                div {
                    class: "text-[9px] font-black text-zinc-500 tracking-widest uppercase",
                    "CHAT"
                }
            }

            div {
                class: "flex-1 min-h-0 overflow-y-auto p-4 space-y-2",

                for event in logs.read().iter() {
                    {
                        match event {
                            RconLogEvent::LiveLog(parsed) => {
                                if parsed.log_type == LogType::Chat {
                                    rsx! {
                                        div {
                                            class: "px-3 py-2 bg-zinc-900 border border-zinc-800 rounded",

                                            div {
                                                class: "text-zinc-300 whitespace-pre-wrap break-words",
                                                {pretty_log(&parsed.event)}
                                            }
                                        }
                                    }
                                } else {
                                    rsx! {}
                                }
                            }

                            RconLogEvent::RconResponse(_) => rsx! {},

                            RconLogEvent::Info(_) => rsx! {},
                        }
                    }
                }
            }
        }
    }
}
