//! Starting and stopping workspaces.
//!
//! The only thing that calls a [`ComputeDriver`], and the only thing that
//! writes `workspaces.state`. Both on purpose: state written from two places
//! is state that disagrees with itself, and a driver called from two places
//! is a provider nobody can account for.
//!
//! **The provider is the truth, not our database.** Anything can remove a
//! container or a machine behind our back, so every read asks the provider
//! and writes down what it said. A workspace whose computer vanished is
//! reported as asleep with no computer, not as awake because a row says so.

use croncave_compute::{ComputeDriver, ComputeId, Error as ComputeError, Spec, State};
use croncave_db::Pool;
use uuid::Uuid;

/// The image a workspace runs: the agent, and enough of a system to run
/// commands. Built from `images/workspace/Dockerfile`.
pub const WORKSPACE_IMAGE: &str = "croncave/workspace:dev";

/// What can go wrong running a workspace.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// No such workspace, or none this team can see.
    #[error("no such workspace")]
    NotFound,

    /// The provider could not do what was asked.
    #[error(transparent)]
    Compute(#[from] ComputeError),

    /// The database could not be reached.
    #[error("could not reach the database")]
    Database(#[source] sqlx::Error),
}

/// Note that something happened in a workspace, so the idle timer starts
/// again.
///
/// Called by everything that counts as the workspace being in use. Failing
/// to record it is not worth failing the request over — the worst case is a
/// workspace sleeping sooner than it should.
pub async fn touch(pool: &Pool, workspace_id: Uuid) {
    let touched = sqlx::query("update workspaces set last_active_at = now() where id = $1")
        .bind(workspace_id)
        .execute(pool)
        .await;

    if let Err(error) = touched {
        tracing::warn!(%error, %workspace_id, "could not record activity");
    }
}

/// Stop every workspace that has been quiet for longer than the timeout.
///
/// Returns how many were put to sleep.
///
/// # Errors
///
/// Returns an error if the database cannot be read.
pub async fn sleep_idle(
    pool: &Pool,
    driver: &dyn ComputeDriver,
    idle_for: std::time::Duration,
) -> Result<usize, Error> {
    let cutoff = time::OffsetDateTime::now_utc() - idle_for;

    // A workspace that is awake but has never been active is one whose
    // activity we never recorded; treat its creation as the last thing that
    // happened rather than leaving it awake for ever.
    let idle: Vec<(Uuid, Uuid)> = sqlx::query_as(
        "select id, team_id from workspaces
          where state = 'awake'
            and coalesce(last_active_at, updated_at) < $1",
    )
    .bind(cutoff)
    .fetch_all(pool)
    .await
    .map_err(Error::Database)?;

    let mut slept = 0;

    for (workspace_id, team_id) in idle {
        match stop(pool, driver, team_id, workspace_id).await {
            Ok(_) => {
                tracing::info!(%workspace_id, "nothing was happening, so it went to sleep");
                slept += 1;
            }
            // One workspace refusing to stop must not stop the rest being
            // swept; it will be tried again on the next pass.
            Err(error) => tracing::warn!(%error, %workspace_id, "could not put it to sleep"),
        }
    }

    Ok(slept)
}

/// What a workspace's computer is doing, as the provider sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Computer {
    /// Where it is in its life.
    pub state: State,
}

/// The stored pointer to a workspace's computer.
struct Stored {
    provider: Option<String>,
    id: Option<String>,
}

impl Stored {
    /// The compute id, when this workspace has ever had a computer made for
    /// it by the driver now in use. A pointer left by a different provider is
    /// deliberately ignored rather than handed to a driver that cannot read
    /// it.
    fn id_for(&self, driver: &dyn ComputeDriver) -> Option<ComputeId> {
        match (&self.provider, &self.id) {
            (Some(provider), Some(id)) if provider == driver.name() => {
                Some(ComputeId::new(id.clone()))
            }
            _ => None,
        }
    }
}

/// Read the stored pointer for a workspace the team owns.
async fn stored(pool: &Pool, team_id: Uuid, workspace_id: Uuid) -> Result<Stored, Error> {
    let row: Option<(Option<String>, Option<String>)> = sqlx::query_as(
        "select compute_provider, compute_id
           from workspaces
          where id = $1 and team_id = $2",
    )
    .bind(workspace_id)
    // Scoped by team here as everywhere: an id alone never reaches a row.
    .bind(team_id)
    .fetch_optional(pool)
    .await
    .map_err(Error::Database)?;

    let (provider, id) = row.ok_or(Error::NotFound)?;

    Ok(Stored { provider, id })
}

/// Write down what the provider said.
async fn record(
    pool: &Pool,
    workspace_id: Uuid,
    driver: &dyn ComputeDriver,
    compute_id: Option<&ComputeId>,
    state: State,
) -> Result<(), Error> {
    sqlx::query(
        "update workspaces
            set state = $2,
                compute_provider = $3,
                compute_id = $4,
                compute_seen_at = now(),
                updated_at = now()
          where id = $1",
    )
    .bind(workspace_id)
    .bind(workspace_state(state))
    .bind(compute_id.map(|_| driver.name()))
    .bind(compute_id.map(ComputeId::as_str))
    .execute(pool)
    .await
    .map_err(Error::Database)?;

    Ok(())
}

/// The lifecycle word stored on a workspace.
///
/// A computer the provider has lost maps to asleep: from a person's side the
/// workspace is simply not running, and the next start makes a new one.
fn workspace_state(state: State) -> &'static str {
    match state {
        State::Running => "awake",
        State::Starting => "waking",
        State::Stopping => "stopping",
        State::Stopped | State::Gone => "asleep",
    }
}

/// Start a workspace, making its computer the first time.
///
/// # Errors
///
/// Returns an error if the workspace is not the team's, or the provider
/// refuses.
pub async fn start(
    pool: &Pool,
    driver: &dyn ComputeDriver,
    relay_url: &str,
    team_id: Uuid,
    workspace_id: Uuid,
) -> Result<Computer, Error> {
    let stored = stored(pool, team_id, workspace_id).await?;

    // Every start gets a fresh identity. A container that comes back after
    // being replaced cannot rejoin on an old one, and a token left inside a
    // stopped workspace is already spent.
    let spec = || async {
        let token = crate::agents::mint_bootstrap(pool, workspace_id)
            .await
            .map_err(Error::Database)?;

        Ok::<_, Error>(
            Spec::new(workspace_id, WORKSPACE_IMAGE)
                .with_env("CRONCAVE_RELAY_URL", relay_url)
                .with_env("CRONCAVE_BOOTSTRAP_TOKEN", token)
                .with_env("CRONCAVE_LOG_FORMAT", "json"),
        )
    };

    let compute_id = match stored.id_for(driver) {
        Some(id) => id,
        None => {
            let id = driver.create(&spec().await?).await?;
            tracing::info!(%workspace_id, compute_id = %id, driver = driver.name(), "made a computer");
            id
        }
    };

    // The computer may have been removed behind our back since we last
    // looked, in which case the stored pointer is stale and a new one is
    // made rather than failing.
    let compute_id = match driver.start(&compute_id).await {
        Ok(()) => compute_id,
        Err(ComputeError::NotFound(_)) => {
            tracing::warn!(
                %workspace_id,
                "the computer we had is gone; making another"
            );
            let id = driver.create(&spec().await?).await?;
            driver.start(&id).await?;
            id
        }
        Err(other) => return Err(other.into()),
    };

    let state = driver.status(&compute_id).await?;
    record(pool, workspace_id, driver, Some(&compute_id), state).await?;

    tracing::info!(%workspace_id, %state, "workspace started");

    Ok(Computer { state })
}

/// Stop a workspace, keeping its disk.
///
/// # Errors
///
/// Returns an error if the workspace is not the team's, or the provider
/// refuses.
pub async fn stop(
    pool: &Pool,
    driver: &dyn ComputeDriver,
    team_id: Uuid,
    workspace_id: Uuid,
) -> Result<Computer, Error> {
    let stored = stored(pool, team_id, workspace_id).await?;

    // Nothing was ever made, so there is nothing to stop. Asked to stop,
    // stopped is the honest answer.
    let Some(compute_id) = stored.id_for(driver) else {
        record(pool, workspace_id, driver, None, State::Stopped).await?;
        return Ok(Computer {
            state: State::Stopped,
        });
    };

    match driver.stop(&compute_id).await {
        Ok(()) => {}
        // Already gone counts as stopped.
        Err(ComputeError::NotFound(_)) => {
            record(pool, workspace_id, driver, None, State::Stopped).await?;
            return Ok(Computer {
                state: State::Stopped,
            });
        }
        Err(other) => return Err(other.into()),
    }

    let state = driver.status(&compute_id).await?;
    record(pool, workspace_id, driver, Some(&compute_id), state).await?;

    tracing::info!(%workspace_id, %state, "workspace stopped");

    Ok(Computer { state })
}

/// What a workspace's computer is doing, asked of the provider.
///
/// # Errors
///
/// Returns an error if the workspace is not the team's, or the provider
/// cannot be reached.
pub async fn status(
    pool: &Pool,
    driver: &dyn ComputeDriver,
    team_id: Uuid,
    workspace_id: Uuid,
) -> Result<Computer, Error> {
    let stored = stored(pool, team_id, workspace_id).await?;

    let Some(compute_id) = stored.id_for(driver) else {
        return Ok(Computer {
            state: State::Stopped,
        });
    };

    let state = match driver.status(&compute_id).await {
        Ok(state) => state,
        // Removed behind our back. Forget the pointer, so the next start
        // makes a new one rather than failing for ever.
        Err(ComputeError::NotFound(_)) => {
            tracing::warn!(%workspace_id, "the computer we had is gone");
            record(pool, workspace_id, driver, None, State::Stopped).await?;
            return Ok(Computer {
                state: State::Stopped,
            });
        }
        Err(other) => return Err(other.into()),
    };

    record(pool, workspace_id, driver, Some(&compute_id), state).await?;

    Ok(Computer { state })
}

/// Remove a workspace's computer, if it has one.
///
/// # Errors
///
/// Returns an error if the provider refuses.
pub async fn destroy(
    pool: &Pool,
    driver: &dyn ComputeDriver,
    team_id: Uuid,
    workspace_id: Uuid,
) -> Result<(), Error> {
    let stored = stored(pool, team_id, workspace_id).await?;

    if let Some(compute_id) = stored.id_for(driver) {
        driver.destroy(&compute_id).await?;
        tracing::info!(%workspace_id, "removed a computer");
    }

    record(pool, workspace_id, driver, None, State::Stopped).await?;

    Ok(())
}
