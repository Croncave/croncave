//! Postgres for Croncave: how services connect, and the schema they connect to.
//!
//! The migrations in `migrations/` are the schema. [`migrate`] applies them and
//! is called by the control plane at startup, so a fresh database and a
//! developer's laptop never drift from what the code expects.
//!
//! Migrations must work with both the old and the new code while a rollout is
//! in progress: expand first, contract in a later release. See
//! `docs/conventions.md`.

use std::time::Duration;

use sqlx::migrate::Migrator;
use sqlx::postgres::{PgPoolOptions, Postgres};

/// The pool every service shares.
pub type Pool = sqlx::Pool<Postgres>;

/// The migrations in `migrations/`, compiled into the binary so a deploy
/// carries its own schema and needs no files alongside it.
pub static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

/// How long to wait for a connection before giving up.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Anything that can stop the database being usable.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The pool could not be opened: the URL is wrong, or nothing answered.
    #[error("could not connect to Postgres")]
    Connect(#[source] sqlx::Error),

    /// The migrations could not be applied.
    #[error("could not apply migrations")]
    Migrate(#[source] sqlx::migrate::MigrateError),
}

/// Open a connection pool.
///
/// # Errors
///
/// Returns an error if the URL is invalid or no connection can be made within
/// [`CONNECT_TIMEOUT`].
pub async fn connect(database_url: &str, max_connections: u32) -> Result<Pool, Error> {
    let pool = PgPoolOptions::new()
        .max_connections(max_connections)
        .acquire_timeout(CONNECT_TIMEOUT)
        .connect(database_url)
        .await
        .map_err(Error::Connect)?;

    tracing::info!(max_connections, "connected to Postgres");

    Ok(pool)
}

/// Apply every migration that hasn't run yet. Safe to call on every start:
/// applying nothing is the normal case.
///
/// # Errors
///
/// Returns an error if a migration fails, or if one that has already run has
/// been edited since — which means the code and the database disagree about
/// what the schema is.
pub async fn migrate(pool: &Pool) -> Result<(), Error> {
    MIGRATOR.run(pool).await.map_err(Error::Migrate)?;

    tracing::info!(migrations = MIGRATOR.iter().len(), "schema is up to date");

    Ok(())
}
