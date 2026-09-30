//! Structured logging for Croncave services.
//!
//! Every binary calls [`init`] once at startup and holds the returned
//! [`Guard`] for as long as it runs. Load `.env` first, so a local run picks
//! up the settings in it (see `.env.example`):
//!
//! ```no_run
//! # fn main() -> Result<(), croncave_telemetry::Error> {
//! dotenvy::dotenv().ok();
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
//! Error reporting is on only when `SENTRY_DSN` is set, so local runs and CI
//! need no account and no secret. When it is set, every `tracing::error!`
//! becomes a Sentry event, lower-level events become breadcrumbs on it, and
//! panics are reported too.
//!
//! **Never log a secret.** Tokens, API keys, GitHub credentials and workspace
//! credentials must not appear in a field or a message. See
//! `AGENTS.md`.

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

    /// `SENTRY_DSN` was set but is not a DSN.
    #[error("invalid SENTRY_DSN")]
    InvalidDsn(#[from] sentry::types::ParseDsnError),

    /// [`init`] was called twice, or something else installed a subscriber.
    #[error("a global tracing subscriber is already installed")]
    AlreadyInitialised(#[source] tracing::subscriber::SetGlobalDefaultError),
}

/// Held for the lifetime of the process. Dropping it flushes anything
/// buffered, including error reports still in flight, so keep it alive until
/// the service exits.
#[must_use = "telemetry is torn down when the guard is dropped"]
pub struct Guard {
    sentry: Option<sentry::ClientInitGuard>,
}

impl Guard {
    /// Whether errors are being reported. False when no DSN was configured.
    #[must_use]
    pub fn reports_errors(&self) -> bool {
        self.sentry.as_ref().is_some_and(|guard| guard.is_enabled())
    }
}

impl std::fmt::Debug for Guard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Guard")
            .field("reports_errors", &self.reports_errors())
            .finish()
    }
}

/// Install logging for this process and announce that the service is starting.
///
/// # Errors
///
/// Returns an error if the configuration is invalid, or if a global
/// subscriber is already installed.
pub fn init(config: Config) -> Result<Guard, Error> {
    let subscriber = build_subscriber(&config)?;

    // Start the client before the subscriber, so an error logged during
    // startup is already being reported.
    let sentry = config
        .sentry_dsn
        .clone()
        .map(|dsn| sentry::init(sentry_options(&config, dsn)));
    let guard = Guard { sentry };

    tracing::subscriber::set_global_default(subscriber).map_err(Error::AlreadyInitialised)?;

    match config.log_format {
        // JSON events already carry the identity on every line.
        LogFormat::Json => tracing::info!(
            log_format = config.log_format.as_str(),
            filter = config.filter,
            reports_errors = guard.reports_errors(),
            "telemetry ready"
        ),
        // Pretty events don't, so this is where a person reads it.
        LogFormat::Pretty => tracing::info!(
            service = config.service,
            version = config.version,
            environment = config.environment.as_str(),
            log_format = config.log_format.as_str(),
            filter = config.filter,
            reports_errors = guard.reports_errors(),
            "telemetry ready"
        ),
    }

    if !guard.reports_errors() {
        tracing::debug!("error reporting is off: no SENTRY_DSN is set");
    }

    Ok(guard)
}

/// How the Sentry client is configured. Kept separate so the choices are
/// visible and testable.
fn sentry_options(config: &Config, dsn: sentry::types::Dsn) -> sentry::ClientOptions {
    // ClientOptions is non-exhaustive, so set the fields we care about and
    // leave the rest at their defaults (performance tracing, for one, is off
    // by default and is not part of R1).
    let mut options = sentry::ClientOptions::default();
    options.dsn = Some(dsn);
    // Tells one deployed build from another in Sentry.
    options.release = Some(format!("{}@{}", config.service, config.version).into());
    options.environment = Some(config.environment.as_str().into());
    options.attach_stacktrace = true;
    // Never send user identifiers, addresses or headers Sentry would otherwise
    // infer. We decide what a report contains, not the SDK.
    options.send_default_pii = false;
    options
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

    Ok(Registry::default()
        .with(layer)
        // Turns `tracing::error!` into Sentry events and quieter events into
        // breadcrumbs. Does nothing until a client exists.
        .with(sentry_tracing::layer())
        .with(filter))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

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

    fn test_dsn() -> sentry::types::Dsn {
        "https://sup3rsecret@o1.ingest.sentry.io/42"
            .parse()
            .unwrap()
    }

    #[test]
    fn errors_become_sentry_events_and_quieter_events_do_not() {
        let config = json_config();
        let buffer = Buffer::default();
        let subscriber = subscriber_with_writer(&config, buffer.clone()).unwrap();

        let captured = sentry::test::with_captured_events(|| {
            tracing::subscriber::with_default(subscriber, || {
                tracing::info!("workspace woke");
                tracing::error!(workspace = "demo", "could not reach the relay");
            });
        });

        assert_eq!(captured.len(), 1, "only the error should be reported");
        let event = &captured[0];
        assert_eq!(event.level, sentry::Level::Error);

        // The quieter event is still logged, and comes along as a breadcrumb
        // for context.
        assert_eq!(buffer.events().len(), 2);
        assert!(
            event
                .breadcrumbs
                .iter()
                .any(|crumb| crumb.message.as_deref() == Some("workspace woke")),
            "{:?}",
            event.breadcrumbs
        );
    }

    #[test]
    fn options_describe_the_build_and_send_no_pii() {
        let options = sentry_options(&json_config(), test_dsn());

        assert_eq!(options.release.as_deref(), Some("relay@0.2.0"));
        assert_eq!(options.environment.as_deref(), Some("staging"));
        assert!(options.attach_stacktrace);
        assert!(!options.send_default_pii);
    }

    #[test]
    fn a_guard_without_a_dsn_reports_nothing() {
        let guard = Guard { sentry: None };

        assert!(!guard.reports_errors());
        assert!(format!("{guard:?}").contains("reports_errors: false"));
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
