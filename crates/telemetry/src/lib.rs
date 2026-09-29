//! Structured logging for Croncave services.
//!
//! Every binary calls [`init`] once at startup and holds the returned
//! [`Guard`] for as long as it runs:
//!
//! ```no_run
//! # fn main() -> Result<(), croncave_telemetry::Error> {
//! let config = croncave_telemetry::Config::from_env("relay", env!("CARGO_PKG_VERSION"))?;
//! let _telemetry = croncave_telemetry::init(config)?;
//! tracing::info!(port = 443, "relay listening");
//! # Ok(())
//! # }
//! ```
//!
//! In JSON mode every event carries the service's name, version and
//! environment. In pretty mode (a developer's terminal) they are written once,
//! in the startup line, rather than repeated on every line.
//!
//! **Never log a secret.** Tokens, API keys, GitHub credentials and workspace
//! credentials must not appear in a field or a message. See
//! `docs/conventions.md`.

pub mod config;
mod json;

pub use config::{Config, Environment, LogFormat};

use tracing::Subscriber;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::layer::{Layer, SubscriberExt as _};
use tracing_subscriber::registry::Registry;

/// Anything that can stop telemetry being set up.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// `CRONCAVE_ENV` held a value we don't recognise.
    #[error("unknown CRONCAVE_ENV value {0:?}: expected local, ci, staging or production")]
    UnknownEnvironment(String),

    /// `CRONCAVE_LOG_FORMAT` held a value we don't recognise.
    #[error("unknown CRONCAVE_LOG_FORMAT value {0:?}: expected pretty or json")]
    UnknownLogFormat(String),

    /// The `RUST_LOG` filter could not be parsed.
    #[error("invalid RUST_LOG filter")]
    InvalidFilter(#[from] tracing_subscriber::filter::ParseError),

    /// [`init`] was called twice, or something else installed a subscriber.
    #[error("a global tracing subscriber is already installed")]
    AlreadyInitialised(#[source] tracing::subscriber::SetGlobalDefaultError),
}

/// Held for the lifetime of the process. Dropping it flushes anything
/// buffered, so keep it alive until the service exits.
#[must_use = "telemetry is torn down when the guard is dropped"]
#[derive(Debug)]
pub struct Guard {
    _private: (),
}

/// Install logging for this process and announce that the service is starting.
///
/// # Errors
///
/// Returns an error if the configuration is invalid, or if a global
/// subscriber is already installed.
pub fn init(config: Config) -> Result<Guard, Error> {
    let subscriber = build_subscriber(&config)?;
    tracing::subscriber::set_global_default(subscriber).map_err(Error::AlreadyInitialised)?;

    match config.log_format {
        // JSON events already carry the identity on every line.
        LogFormat::Json => tracing::info!(
            log_format = config.log_format.as_str(),
            filter = config.filter,
            "telemetry ready"
        ),
        // Pretty events don't, so this is where a person reads it.
        LogFormat::Pretty => tracing::info!(
            service = config.service,
            version = config.version,
            environment = config.environment.as_str(),
            log_format = config.log_format.as_str(),
            filter = config.filter,
            "telemetry ready"
        ),
    }

    Ok(Guard { _private: () })
}

/// Build the subscriber [`init`] installs, writing to standard output.
///
/// Exposed separately so it can be composed or inspected without touching
/// process-wide state.
///
/// # Errors
///
/// Returns an error if the filter in the configuration is invalid.
pub fn build_subscriber(config: &Config) -> Result<impl Subscriber + Send + Sync, Error> {
    subscriber_with_writer(config, std::io::stdout)
}

/// The body of [`build_subscriber`], with the destination injected so tests
/// can read what was written.
fn subscriber_with_writer<W>(
    config: &Config,
    writer: W,
) -> Result<impl Subscriber + Send + Sync, Error>
where
    W: for<'w> MakeWriter<'w> + Send + Sync + 'static,
{
    let filter = EnvFilter::try_new(&config.filter)?;

    let layer: Box<dyn Layer<Registry> + Send + Sync> = match config.log_format {
        LogFormat::Json => Box::new(
            tracing_subscriber::fmt::layer()
                .with_writer(writer)
                .event_format(json::JsonFormat::new(config)),
        ),
        LogFormat::Pretty => Box::new(
            tracing_subscriber::fmt::layer()
                .with_writer(writer)
                .with_target(true),
        ),
    };

    Ok(Registry::default().with(layer).with(filter))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use std::io;
    use std::sync::{Arc, Mutex};

    use serde_json::Value;

    use super::*;

    /// Collects log output so a test can read it back.
    #[derive(Clone, Default)]
    struct Buffer(Arc<Mutex<Vec<u8>>>);

    impl Buffer {
        fn events(&self) -> Vec<Value> {
            let bytes = self.0.lock().unwrap().clone();
            String::from_utf8(bytes)
                .unwrap()
                .lines()
                .filter(|line| !line.trim().is_empty())
                .map(|line| serde_json::from_str(line).unwrap())
                .collect()
        }

        fn text(&self) -> String {
            String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
        }
    }

    impl io::Write for Buffer {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl<'a> MakeWriter<'a> for Buffer {
        type Writer = Self;

        fn make_writer(&'a self) -> Self::Writer {
            self.clone()
        }
    }

    fn record(config: &Config, body: impl FnOnce()) -> Buffer {
        let buffer = Buffer::default();
        let subscriber = subscriber_with_writer(config, buffer.clone()).unwrap();
        tracing::subscriber::with_default(subscriber, body);
        buffer
    }

    fn json_config() -> Config {
        Config::new("relay", "0.2.0")
            .with_log_format(LogFormat::Json)
            .with_environment(Environment::Staging)
    }

    #[test]
    fn json_events_carry_the_service_identity() {
        let buffer = record(&json_config(), || {
            tracing::info!(task = "smoke", attempt = 2, "hello");
        });

        let events = buffer.events();
        assert_eq!(events.len(), 1);
        let event = &events[0];

        assert_eq!(event["service"], "relay");
        assert_eq!(event["version"], "0.2.0");
        assert_eq!(event["environment"], "staging");
        assert_eq!(event["level"], "info");
        assert_eq!(event["message"], "hello");
        assert_eq!(event["task"], "smoke");
        assert_eq!(event["attempt"], 2);
        assert!(event["target"].is_string());
        assert!(event["timestamp"].is_string());
    }

    #[test]
    fn json_events_name_the_spans_they_happened_in() {
        let buffer = record(&json_config(), || {
            let outer = tracing::info_span!("run");
            let _outer = outer.enter();
            let inner = tracing::info_span!("step");
            let _inner = inner.enter();
            tracing::warn!("slow");
        });

        let events = buffer.events();
        assert_eq!(events[0]["spans"], serde_json::json!(["run", "step"]));
    }

    #[test]
    fn an_event_field_never_overwrites_the_service_identity() {
        let buffer = record(&json_config(), || {
            tracing::info!(service = "impostor", "hello");
        });

        let event = &buffer.events()[0];
        assert_eq!(event["service"], "relay");
        assert_eq!(event["event.service"], "impostor");
    }

    #[test]
    fn the_filter_decides_what_is_written() {
        let config = json_config().with_filter("warn");
        let buffer = record(&config, || {
            tracing::info!("quiet");
            tracing::warn!("loud");
        });

        let events = buffer.events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0]["message"], "loud");
    }

    #[test]
    fn an_invalid_filter_is_rejected() {
        let config = json_config().with_filter("croncave=notalevel");
        // The success type is opaque, so match rather than unwrap_err.
        match subscriber_with_writer(&config, Buffer::default()) {
            Err(Error::InvalidFilter(_)) => {}
            Err(other) => panic!("expected an invalid filter, got {other}"),
            Ok(_) => panic!("expected an invalid filter to be rejected"),
        }
    }

    #[test]
    fn pretty_output_is_human_readable() {
        let config = Config::new("relay", "0.2.0").with_log_format(LogFormat::Pretty);
        let buffer = record(&config, || {
            tracing::info!(task = "smoke", "hello");
        });

        let text = buffer.text();
        assert!(text.contains("hello"), "{text}");
        assert!(text.contains("task"), "{text}");
        assert!(
            serde_json::from_str::<Value>(text.trim()).is_err(),
            "pretty output should not be JSON: {text}"
        );
    }
}
