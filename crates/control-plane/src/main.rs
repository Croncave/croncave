//! The control plane binary.

use anyhow::Context as _;
use croncave_control_plane::Config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env before reading any configuration; a real environment
    // variable always wins. See docs/conventions.md.
    dotenvy::dotenv().ok();

    let _telemetry = croncave_telemetry::init(croncave_telemetry::Config::from_env(
        "control-plane",
        env!("CARGO_PKG_VERSION"),
    )?)?;

    let config = Config::from_env().context("reading the configuration")?;
    tracing::info!(?config, "starting");

    croncave_control_plane::serve(config)
        .await
        .context("serving")?;

    Ok(())
}
