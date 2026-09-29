//! The JSON log format: one object per line, each carrying the service's
//! identity so events from different services and environments never blur
//! together in a collector.

use std::fmt;

use serde_json::{Map, Value};
use tracing::field::{Field, Visit};
use tracing::{Event, Subscriber};
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::time::{FormatTime as _, SystemTime};
use tracing_subscriber::fmt::{FmtContext, FormatEvent, FormatFields};
use tracing_subscriber::registry::LookupSpan;

use crate::Config;

/// Keys the formatter writes itself. An event field with the same name is
/// kept, under an `event.` prefix, rather than silently dropped or allowed to
/// overwrite the service's identity.
const RESERVED: [&str; 7] = [
    "timestamp",
    "level",
    "target",
    "service",
    "version",
    "environment",
    "spans",
];

/// Writes each event as a single JSON object.
#[derive(Clone, Debug)]
pub(crate) struct JsonFormat {
    service: String,
    version: String,
    environment: &'static str,
}

impl JsonFormat {
    pub(crate) fn new(config: &Config) -> Self {
        Self {
            service: config.service.clone(),
            version: config.version.clone(),
            environment: config.environment.as_str(),
        }
    }
}

impl<S, N> FormatEvent<S, N> for JsonFormat
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        let metadata = event.metadata();
        let mut object = Map::new();

        let mut timestamp = String::new();
        if SystemTime
            .format_time(&mut Writer::new(&mut timestamp))
            .is_ok()
        {
            object.insert("timestamp".to_owned(), Value::String(timestamp));
        }

        object.insert(
            "level".to_owned(),
            Value::String(metadata.level().as_str().to_ascii_lowercase()),
        );
        object.insert(
            "target".to_owned(),
            Value::String(metadata.target().to_owned()),
        );
        object.insert("service".to_owned(), Value::String(self.service.clone()));
        object.insert("version".to_owned(), Value::String(self.version.clone()));
        object.insert(
            "environment".to_owned(),
            Value::String(self.environment.to_owned()),
        );

        if let Some(scope) = ctx.event_scope() {
            let spans: Vec<Value> = scope
                .from_root()
                .map(|span| Value::String(span.name().to_owned()))
                .collect();
            if !spans.is_empty() {
                object.insert("spans".to_owned(), Value::Array(spans));
            }
        }

        let mut fields = Map::new();
        event.record(&mut FieldCollector {
            fields: &mut fields,
        });
        for (name, value) in fields {
            if RESERVED.contains(&name.as_str()) {
                object.insert(format!("event.{name}"), value);
            } else {
                object.insert(name, value);
            }
        }

        writeln!(writer, "{}", Value::Object(object))
    }
}

/// Copies an event's fields into a JSON object.
struct FieldCollector<'a> {
    fields: &'a mut Map<String, Value>,
}

impl FieldCollector<'_> {
    fn insert(&mut self, field: &Field, value: Value) {
        self.fields.insert(field.name().to_owned(), value);
    }
}

impl Visit for FieldCollector<'_> {
    fn record_str(&mut self, field: &Field, value: &str) {
        self.insert(field, Value::String(value.to_owned()));
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.insert(field, Value::Bool(value));
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.insert(field, Value::Number(value.into()));
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.insert(field, Value::Number(value.into()));
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        // Infinity and NaN have no JSON representation; keep them as text
        // rather than losing the value.
        let value = serde_json::Number::from_f64(value)
            .map_or_else(|| Value::String(value.to_string()), Value::Number);
        self.insert(field, value);
    }

    fn record_error(&mut self, field: &Field, value: &(dyn std::error::Error + 'static)) {
        self.insert(field, Value::String(value.to_string()));
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.insert(field, Value::String(format!("{value:?}")));
    }
}
