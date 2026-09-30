//! The control plane: the API, and later the scheduler and orchestrator.
//!
//! [`router`] builds the application without touching the network, so tests
//! can call it directly; [`serve`] is what the binary runs.

pub mod actor;
pub mod agents;
pub mod auth;
pub mod config;
mod health;
pub mod mail;
pub mod orchestrator;
mod tokens;
pub mod workspaces;

pub use config::Config;

use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::routing::{get, post};
use croncave_db::Pool;
use tower_http::trace::TraceLayer;

use croncave_compute::ComputeDriver;
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

    /// `CRONCAVE_IDLE_SECONDS` is not a number.
    #[error("invalid CRONCAVE_IDLE_SECONDS value {0:?}: expected a whole number of seconds")]
    InvalidIdleSeconds(String),

    /// `CRONCAVE_COMPUTE_DRIVER` names a driver we don't have.
    #[error("unknown CRONCAVE_COMPUTE_DRIVER value {0:?}: expected fake or local")]
    UnknownComputeDriver(String),

    /// The compute provider could not be reached at startup.
    #[error("could not reach the compute provider")]
    Compute(#[from] croncave_compute::Error),

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
    /// Where workspaces' computers come from.
    pub compute: Arc<dyn ComputeDriver>,
    /// The connections workspaces have opened to us.
    pub relay: croncave_relay::Relay,
    /// What a workspace dials to reach the relay.
    pub workspace_relay_url: String,
}

impl State {
    /// Build the state from a configuration, an open pool and a driver.
    #[must_use]
    pub fn new(
        pool: Pool,
        mailer: Arc<dyn Mailer>,
        compute: Arc<dyn ComputeDriver>,
        config: &Config,
    ) -> Self {
        // The relay asks the control plane who each connection belongs to,
        // so it never touches the schema itself.
        let relay =
            croncave_relay::Relay::new(Arc::new(agents::DatabaseAuthoriser::new(pool.clone())));

        Self {
            pool,
            mailer,
            app_url: config.app_url.clone(),
            secure_cookies: config.environment.is_deployed(),
            compute,
            relay,
            workspace_relay_url: config.workspace_relay_url.clone(),
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
        // Where workspace agents dial in. Not for browsers: a workspace's
        // own connection is the only thing that belongs here.
        .route("/agent", axum::routing::any(agent_socket))
        .route("/workspaces/{id}/start", post(workspaces::start))
        .route("/workspaces/{id}/stop", post(workspaces::stop))
        .route("/workspaces/{id}/run", post(workspaces::run))
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

    let compute = compute_driver(config.compute_driver).await?;
    tracing::info!(
        driver = compute.name(),
        "workspaces' computers come from here"
    );

    let state = State::new(pool.clone(), mailer, compute, &config);

    let listener = tokio::net::TcpListener::bind(config.bind)
        .await
        .map_err(|source| Error::Serve {
            bind: config.bind.to_string(),
            source,
        })?;

    // Sleep by default: nothing else watches for a workspace that has been
    // left alone, and a workspace awake for no reason is a workspace being
    // paid for.
    let sweeper = tokio::spawn(sweep_idle(
        pool.clone(),
        Arc::clone(&state.compute),
        config.idle_for,
    ));

    tracing::info!(bind = %config.bind, "control plane listening");

    axum::serve(listener, router(state))
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|source| Error::Serve {
            bind: config.bind.to_string(),
            source,
        })?;

    sweeper.abort();

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

/// Put workspaces to sleep once nothing is happening in them.
///
/// How often it looks is a fraction of the timeout, so a workspace sleeps
/// somewhere near when it should without the database being asked constantly.
async fn sweep_idle(pool: Pool, compute: Arc<dyn ComputeDriver>, idle_for: std::time::Duration) {
    let every = (idle_for / 4).max(std::time::Duration::from_secs(1));
    let mut ticker = tokio::time::interval(every);

    loop {
        ticker.tick().await;

        match orchestrator::sleep_idle(&pool, compute.as_ref(), idle_for).await {
            Ok(0) => {}
            Ok(slept) => tracing::info!(slept, "put idle workspaces to sleep"),
            // The sweeper must outlive a bad night: a database blip should
            // not stop workspaces ever sleeping again.
            Err(error) => tracing::warn!(%error, "could not look for idle workspaces"),
        }
    }
}

/// Accept a workspace agent's connection.
async fn agent_socket(
    axum::extract::State(state): axum::extract::State<State>,
    upgrade: axum::extract::WebSocketUpgrade,
) -> axum::response::Response {
    upgrade.on_upgrade(move |socket| croncave_relay::serve_agent(state.relay, socket))
}

/// Connect to the chosen compute provider.
///
/// Done at startup rather than per request, so a machine with no Docker finds
/// out immediately instead of when someone first presses start.
async fn compute_driver(
    choice: config::ComputeDriverChoice,
) -> Result<Arc<dyn ComputeDriver>, Error> {
    Ok(match choice {
        config::ComputeDriverChoice::Fake => Arc::new(croncave_compute::fake::FakeDriver::new()),
        config::ComputeDriverChoice::Local => {
            Arc::new(croncave_compute::local::LocalDriver::connect().await?)
        }
    })
}
