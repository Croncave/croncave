//! Proves logging works, by hand.
//!
//! ```text
//! cargo run -p croncave-telemetry --example smoke
//! CRONCAVE_LOG_FORMAT=json cargo run -p croncave-telemetry --example smoke | jq .
//! RUST_LOG=debug cargo run -p croncave-telemetry --example smoke
//! SENTRY_DSN=<your dsn> cargo run -p croncave-telemetry --example smoke
//! ```
//!
//! With a DSN set, the error below turns up in Sentry, with the quieter
//! events as breadcrumbs.

use croncave_telemetry::{Config, Error};

fn main() -> Result<(), Error> {
    // Every binary starts this way: load `.env` if there is one, before
    // reading any configuration. Real environment variables win over the
    // file, and in staging and production there is no file at all.
    dotenvy::dotenv().ok();

    let config = Config::from_env("telemetry-smoke", env!("CARGO_PKG_VERSION"))?;
    let telemetry = croncave_telemetry::init(config)?;

    // Hidden at the default `info` filter; set RUST_LOG=debug to see them.
    tracing::trace!("trace events are the finest grain");
    tracing::debug!(step = "connect", "debug events carry detail");

    let span = tracing::info_span!("run", run_id = 7);
    let _span = span.enter();

    tracing::info!(workspace = "demo", awake_ms = 412, "workspace woke");
    tracing::warn!(idle_minutes = 10, "workspace is going to sleep");
    let failure = std::io::Error::new(
        std::io::ErrorKind::ConnectionRefused,
        "relay.croncave.com:443",
    );
    tracing::error!(
        error = &failure as &dyn std::error::Error,
        "could not reach the relay, retrying"
    );

    if telemetry.reports_errors() {
        tracing::info!("that error was reported to Sentry");
    } else {
        tracing::info!("error tracking is off: set SENTRY_DSN to report that error");
    }

    Ok(())
}
