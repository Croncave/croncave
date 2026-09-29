//! The control plane: the API, and later the scheduler and orchestrator.
//!
//! [`router`] builds the application without touching the network, so tests
//! can call it directly; [`serve`] is what the binary runs.

pub mod actor;
pub mod auth;
pub mod config;
mod health;
pub mod mail;
mod tokens;
pub mod workspaces;

pub use config::Config;

use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::routing::{get, post};
use croncave_db::Pool;
use tower_http::trace::TraceLayer;

use mail::Mailer;

/// Anything that can stop the control plane starting or running.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// `DATABASE_URL` is not set, so there is nothing to connect to.
    #[error("DATABASE_URL is not set: the control plane needs a database")]
    MissingDatabaseUrl,

    /// `CRONCAVE_BIND` is not an address and port.
    #[error("invalid CRONCAVE_BIND value {0:?}: expected something like 127.0.0.1:8080")]
    InvalidBind(String),

    /// `CRONCAVE_DB_MAX_CONNECTIONS` is not a number.
    #[error("invalid CRONCAVE_DB_MAX_CONNECTIONS value {0:?}: expected a whole number")]
    InvalidMaxConnections(String),

    /// `CRONCAVE_ENV` is not one we know.
    #[error("invalid CRONCAVE_ENV value {0:?}: expected local, ci, staging or production")]
    InvalidEnvironment(String),

    /// The database could not be reached, or its schema could not be applied.
    #[error(transparent)]
    Database(#[from] croncave_db::Error),

    /// The address could not be listened on, or the server stopped badly.
    #[error("could not serve on {bind}")]
    Serve {
        /// The address we tried to listen on.
        bind: String,
        /// Why it failed.
        #[source]
        source: std::io::Error,
    },
}

/// What every request handler can reach.
#[derive(Clone)]
pub struct State {
    /// The database pool.
    pub pool: Pool,
    /// How sign-in links are delivered.
    pub mailer: Arc<dyn Mailer>,
    /// Where the browser should be sent, and what sign-in links point at.
    pub app_url: String,
    /// Whether the session cookie is marked `Secure`. False only where there
    /// is no certificate, which is local development.
    pub secure_cookies: bool,
}

impl State {
    /// Build the state from a configuration and an open pool.
    #[must_use]
    pub fn new(pool: Pool, mailer: Arc<dyn Mailer>, config: &Config) -> Self {
        Self {
            pool,
            mailer,
            app_url: config.app_url.clone(),
            secure_cookies: config.environment.is_deployed(),
        }
    }
}

/// Build the application.
///
/// Takes no network and no globals, so a test can call the routes directly.
pub fn router(state: State) -> Router {
    Router::new()
        .route("/health", get(health::health))
        .route("/ready", get(health::ready))
        .route("/auth/request-link", post(auth::request_link))
        .route("/auth/callback", get(auth::callback))
        .route("/auth/sign-out", post(auth::sign_out))
        .route("/me", get(auth::me))
        .route(
            "/workspaces",
            get(workspaces::list).post(workspaces::create),
        )
        .route("/workspaces/{id}", get(workspaces::get))
        .with_state(state)
        .layer(TraceLayer::new_for_http())
}

/// Connect to Postgres, bring the schema up to date, and serve until the
/// process is asked to stop.
///
/// Migrations run here rather than as a separate deploy step because sqlx
/// takes a lock while applying them, so several replicas starting at once is
/// safe, and a binary never serves against a schema older than it expects.
/// Migrations still have to work with the previous release's code while a
/// rollout is in progress — expand first, contract later.
///
/// # Errors
///
/// Returns an error if the database is unreachable, the migrations fail, or
/// the address cannot be listened on.
pub async fn serve(config: Config) -> Result<(), Error> {
    let pool = croncave_db::connect(&config.database_url, config.db_max_connections).await?;
    croncave_db::migrate(&pool).await?;

    let mailer = mail::for_environment(config.environment);
    tracing::info!(?mailer, "sign-in links will be delivered this way");
    let state = State::new(pool.clone(), mailer, &config);

    let listener = tokio::net::TcpListener::bind(config.bind)
        .await
        .map_err(|source| Error::Serve {
            bind: config.bind.to_string(),
            source,
        })?;

    tracing::info!(bind = %config.bind, "control plane listening");

    axum::serve(listener, router(state))
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|source| Error::Serve {
            bind: config.bind.to_string(),
            source,
        })?;

    // Let in-flight queries finish rather than dropping the pool underneath
    // them.
    pool.close().await;
    tracing::info!("control plane stopped");

    Ok(())
}

/// Resolves when the process is asked to stop, so work in flight can finish.
async fn shutdown_signal() {
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

    tracing::info!("stopping: letting work in flight finish");
}

/// How long a readiness check waits on the database before calling it down.
const READY_TIMEOUT: Duration = Duration::from_secs(2);
