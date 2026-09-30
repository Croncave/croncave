//! The agent binary, which runs as a workspace's main process.

use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    // No .env here: a workspace is not a developer machine, and everything
    // the agent needs is put in its environment by the orchestrator.
    let telemetry = croncave_telemetry::Config::from_env("agent", env!("CARGO_PKG_VERSION"));
    let _telemetry = match telemetry.and_then(croncave_telemetry::init) {
        Ok(guard) => guard,
        Err(error) => {
            eprintln!("could not set up logging: {error}");
            return ExitCode::FAILURE;
        }
    };

    let config = match croncave_agent::config_from_env(env!("CARGO_PKG_VERSION")) {
        Ok(config) => config,
        Err(missing) => {
            tracing::error!("{missing}");
            return ExitCode::FAILURE;
        }
    };

    tracing::info!(?config, "the workspace agent is starting");

    tokio::select! {
        () = croncave_agent::run_forever(config) => {
            tracing::error!("the agent gave up");
            ExitCode::FAILURE
        }
        () = stopped() => {
            // A workspace is stopped by the platform far more often than it
            // crashes, so leaving promptly is the normal path — and a
            // process that ignores SIGTERM makes every stop wait out the
            // grace period.
            tracing::info!("asked to stop");
            ExitCode::SUCCESS
        }
    }
}

/// Resolves when the platform asks this workspace to stop.
async fn stopped() {
    let interrupt = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut signal) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            signal.recv().await;
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = interrupt => {},
        () = terminate => {},
    }
}
