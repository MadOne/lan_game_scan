use std::{
    collections::VecDeque,
    sync::{Arc, Mutex, OnceLock},
    time::SystemTime,
};
use tracing_log::NormalizeEvent;

use tracing::{Event, Level, Subscriber};
use tracing_subscriber::{layer::Context, registry::LookupSpan, Layer};

const MAX_LOG_ENTRIES: usize = 5000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppLogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl From<&Level> for AppLogLevel {
    fn from(level: &Level) -> Self {
        match *level {
            Level::TRACE => Self::Trace,
            Level::DEBUG => Self::Debug,
            Level::INFO => Self::Info,
            Level::WARN => Self::Warn,
            Level::ERROR => Self::Error,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AppLogEntry {
    pub timestamp: SystemTime,
    pub level: AppLogLevel,
    pub target: String,
    pub message: String,
    pub module_path: Option<String>,
    pub file: Option<String>,
    pub line: Option<u32>,
}
impl AppLogEntry {
    pub fn formatted_time(&self) -> String {
        match self.timestamp.duration_since(std::time::UNIX_EPOCH) {
            Ok(duration) => {
                let total_seconds = duration.as_secs();
                let millis = duration.subsec_millis();

                let seconds = total_seconds % 60;
                let minutes = (total_seconds / 60) % 60;
                let hours = (total_seconds / 3600) % 24;

                format!("{hours:02}:{minutes:02}:{seconds:02}.{millis:03}")
            }

            Err(_) => "??:??:??.???".to_string(),
        }
    }

    pub fn display_target(&self, compact: bool) -> String {
        if !compact {
            return self.target.clone();
        }

        let parts: Vec<&str> = self.target.split("::").collect();

        if parts.len() <= 2 {
            return self.target.clone();
        }

        format!("{}::...::{}", parts[0], parts[parts.len() - 1])
    }
}

#[derive(Clone)]
pub struct AppLogStore {
    entries: Arc<Mutex<VecDeque<AppLogEntry>>>,
}

impl AppLogStore {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(Mutex::new(VecDeque::with_capacity(MAX_LOG_ENTRIES))),
        }
    }

    pub fn push(&self, entry: AppLogEntry) {
        let Ok(mut entries) = self.entries.lock() else {
            return;
        };

        if entries.len() >= MAX_LOG_ENTRIES {
            entries.pop_front();
        }

        entries.push_back(entry);
    }

    pub fn entries(&self) -> Vec<AppLogEntry> {
        let Ok(entries) = self.entries.lock() else {
            return Vec::new();
        };

        entries.iter().cloned().collect()
    }

    pub fn clear(&self) {
        if let Ok(mut entries) = self.entries.lock() {
            entries.clear();
        }
    }
}

struct MessageVisitor {
    message: Option<String>,
}

impl MessageVisitor {
    fn new() -> Self {
        Self { message: None }
    }
}

impl tracing::field::Visit for MessageVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = Some(format!("{value:?}"));
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = Some(value.to_string());
        }
    }
}

pub struct AppLogLayer {
    store: AppLogStore,
}

impl AppLogLayer {
    pub fn new(store: AppLogStore) -> Self {
        Self { store }
    }
}

impl<S> Layer<S> for AppLogLayer
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let normalized_metadata = event.normalized_metadata();

        let metadata = normalized_metadata
            .as_ref()
            .unwrap_or_else(|| event.metadata());

        let mut visitor = MessageVisitor::new();
        event.record(&mut visitor);

        let message = visitor
            .message
            .unwrap_or_else(|| metadata.target().to_string());

        self.store.push(AppLogEntry {
            timestamp: SystemTime::now(),
            level: AppLogLevel::from(metadata.level()),
            target: metadata.target().to_string(),
            message,
            module_path: metadata.module_path().map(str::to_string),
            file: metadata.file().map(str::to_string),
            line: metadata.line(),
        });
    }
}
static APP_LOG_STORE: OnceLock<AppLogStore> = OnceLock::new();

pub fn init_app_log() -> AppLogStore {
    APP_LOG_STORE.get_or_init(AppLogStore::new).clone()
}

pub fn app_log_store() -> AppLogStore {
    APP_LOG_STORE.get_or_init(AppLogStore::new).clone()
}
