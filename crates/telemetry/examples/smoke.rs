//! Proves logging works, by hand.
//!
//! ```text
//! cargo run -p croncave-telemetry --example smoke
//! CRONCAVE_LOG_FORMAT=json cargo run -p croncave-telemetry --example smoke | jq .
//! RUST_LOG=debug cargo run -p croncave-telemetry --example smoke
//! ```

use croncave_telemetry::{Config, Error};

fn main() -> Result<(), Error> {
    let config = Config::from_env("telemetry-smoke", env!("CARGO_PKG_VERSION"))?;
    let _telemetry = croncave_telemetry::init(config)?;

    // Hidden at the default `info` filter; set RUST_LOG=debug to see them.
    tracing::trace!("trace events are the finest grain");
    tracing::debug!(step = "connect", "debug events carry detail");

    let span = tracing::info_span!("run", run_id = 7);
    let _span = span.enter();

    tracing::info!(workspace = "demo", awake_ms = 412, "workspace woke");
    tracing::warn!(idle_minutes = 10, "workspace is going to sleep");
    tracing::error!(
        reason = "connection refused",
        "could not reach the relay, retrying"
    );

    Ok(())
}
