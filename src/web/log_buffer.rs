use chrono::Utc;
use serde_json::Value;
use std::collections::VecDeque;
use std::sync::{Arc, RwLock, OnceLock};
use tracing::{Level, Subscriber};
use tracing_subscriber::Layer;

// Import LogEntry from dashboard
use super::dashboard::LogEntry;

// Global log buffer instance
static GLOBAL_LOG_BUFFER: OnceLock<Arc<LogBuffer>> = OnceLock::new();

/// In-memory log buffer that captures tracing logs
#[derive(Debug)]
pub struct LogBuffer {
    entries: Arc<RwLock<VecDeque<LogEntry>>>,
    max_size: usize,
}

impl LogBuffer {
    pub fn new(max_size: usize) -> Self {
        Self {
            entries: Arc::new(RwLock::new(VecDeque::new())),
            max_size,
        }
    }

    pub fn add_entry(&self, entry: LogEntry) {
        if let Ok(mut entries) = self.entries.write() {
            entries.push_back(entry);
            
            // Keep buffer size under control
            while entries.len() > self.max_size {
                entries.pop_front();
            }
        }
    }

    pub fn get_entries(&self, limit: usize) -> Vec<LogEntry> {
        if let Ok(entries) = self.entries.read() {
            let start = entries.len().saturating_sub(limit);
            entries.iter().skip(start).cloned().collect()
        } else {
            Vec::new()
        }
    }

    pub fn get_filtered_entries(
        &self,
        limit: usize,
        level_filter: Option<&str>,
        search_term: Option<&str>,
    ) -> Vec<LogEntry> {
        if let Ok(entries) = self.entries.read() {
            let filtered: Vec<LogEntry> = entries
                .iter()
                .rev() // Most recent first
                .filter(|entry| {
                    // Apply level filter
                    if let Some(filter_level) = level_filter {
                        if entry.level != filter_level {
                            return false;
                        }
                    }
                    
                    // Apply search filter
                    if let Some(search) = search_term {
                        let search_lower = search.to_lowercase();
                        if !entry.message.to_lowercase().contains(&search_lower) &&
                           !entry.target.to_lowercase().contains(&search_lower) {
                            return false;
                        }
                    }
                    
                    true
                })
                .take(limit)
                .cloned()
                .collect();
            
            filtered
        } else {
            Vec::new()
        }
    }
}

/// Tracing layer that captures logs into the buffer
pub struct LogBufferLayer {
    buffer: Arc<LogBuffer>,
}

impl LogBufferLayer {
    pub fn new(buffer: Arc<LogBuffer>) -> Self {
        Self { buffer }
    }
}

impl<S> Layer<S> for LogBufferLayer
where
    S: Subscriber,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let metadata = event.metadata();
        let level = match *metadata.level() {
            Level::TRACE => "trace",
            Level::DEBUG => "debug",
            Level::INFO => "info",
            Level::WARN => "warn",
            Level::ERROR => "error",
        };

        let target = metadata.target();
        let mut message = String::new();
        let mut fields = serde_json::Map::new();

        // Capture the message and fields
        let mut visitor = LogVisitor::new(&mut message, &mut fields);
        event.record(&mut visitor);

        let entry = LogEntry {
            timestamp: Utc::now(),
            level: level.to_string(),
            target: target.to_string(),
            message,
            fields: if fields.is_empty() {
                None
            } else {
                Some(Value::Object(fields))
            },
        };

        self.buffer.add_entry(entry);
    }
}

/// Visitor to extract log message and fields
struct LogVisitor<'a> {
    message: &'a mut String,
    fields: &'a mut serde_json::Map<String, Value>,
}

impl<'a> LogVisitor<'a> {
    fn new(message: &'a mut String, fields: &'a mut serde_json::Map<String, Value>) -> Self {
        Self { message, fields }
    }
}

impl<'a> tracing::field::Visit for LogVisitor<'a> {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            *self.message = format!("{:?}", value);
        } else {
            self.fields.insert(field.name().to_string(), Value::String(format!("{:?}", value)));
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            *self.message = value.to_string();
        } else {
            self.fields.insert(field.name().to_string(), Value::String(value.to_string()));
        }
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.fields.insert(field.name().to_string(), Value::Number(value.into()));
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.fields.insert(field.name().to_string(), Value::Number(value.into()));
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.fields.insert(field.name().to_string(), Value::Bool(value));
    }
}

// Global access functions
pub fn initialize_global_log_buffer(max_size: usize) -> Arc<LogBuffer> {
    let buffer = Arc::new(LogBuffer::new(max_size));
    GLOBAL_LOG_BUFFER.set(buffer.clone()).expect("Global log buffer already initialized");
    buffer
}

pub fn get_global_log_buffer() -> Option<Arc<LogBuffer>> {
    GLOBAL_LOG_BUFFER.get().cloned()
}