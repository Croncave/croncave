//! Workspaces: the records everything else in Croncave hangs off.
//!
//! A workspace has no computer yet — that arrives in step 2 — so for now it
//! is a name, a team and a state that stays `asleep`.
//!
//! Every query here is scoped by the team on the [`Actor`], never by an id
//! that arrived in the request. Asking for someone else's workspace is
//! answered with "not found" rather than "not allowed", because the second
//! answer tells you the thing exists.

use axum::Json;
use axum::extract::{Path, State as AxumState};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::actor::Actor;
use crate::auth::Error as AuthError;
use crate::{State, orchestrator};

/// The longest a workspace name may be.
const MAX_NAME: usize = 100;

/// How long a command may run before it is stopped.
///
/// A workspace runs code we did not write. Long work belongs to sessions and
/// runs, which are built for it; a command typed into a box is not.
const COMMAND_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);

/// What creating a workspace carries.
#[derive(Debug, Deserialize)]
pub struct NewWorkspace {
    /// What to call it.
    pub name: String,
}

/// What a workspace should be asked to run.
#[derive(Debug, Deserialize)]
pub struct RunCommand {
    /// The program.
    pub program: String,
    /// Its arguments.
    #[serde(default)]
    pub args: Vec<String>,
}

/// What running it did.
#[derive(Debug, Serialize)]
pub struct RanCommand {
    /// Everything it printed, both streams as they arrived.
    pub output: String,
    /// The word for how it ended: done, failed, timed out, stopped.
    pub outcome: String,
    /// Whether it worked.
    pub succeeded: bool,
}

/// A workspace, as the app sees one.
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct WorkspaceView {
    /// Its id.
    pub id: Uuid,
    /// Its name.
    pub name: String,
    /// Where it is in the lifecycle.
    pub state: String,
    /// When it was made.
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    /// Whether its agent is connected right now.
    ///
    /// Not stored: asked of the relay, because a row cannot know whether a
    /// connection is still there.
    #[sqlx(default)]
    pub connected: bool,
}

/// What can go wrong.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The name is empty, or longer than [`MAX_NAME`].
    #[error("a workspace needs a name, of at most {MAX_NAME} characters")]
    InvalidName,

    /// No such workspace — or none this team can see. One answer for both.
    #[error("no such workspace")]
    NotFound,

    /// Signed out, or the session is no longer valid.
    #[error(transparent)]
    Auth(#[from] AuthError),

    /// The compute provider could not do what was asked. Separate from
    /// `Internal` because a person can act on it: try again, or look at
    /// whether Docker is running.
    #[error("the workspace's computer could not be reached")]
    Compute(#[source] croncave_compute::Error),

    /// The workspace is not connected, so there is nothing to ask. A person
    /// can act on this by waking it.
    #[error("that workspace is asleep, or still waking up")]
    NotConnected,

    /// Something on our side failed.
    #[error("something went wrong")]
    Internal,
}

impl axum::response::IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        // Signing-out mid-request should look the same as never having been
        // signed in, so hand that case straight to the auth error.
        if let Self::Auth(error) = self {
            return error.into_response();
        }

        let status = match self {
            Self::InvalidName => StatusCode::BAD_REQUEST,
            Self::NotFound => StatusCode::NOT_FOUND,
            // The request was fine; the provider is the problem.
            Self::Compute(_) => StatusCode::BAD_GATEWAY,
            // Nothing is wrong: it is simply not there to ask.
            Self::NotConnected => StatusCode::CONFLICT,
            Self::Auth(_) | Self::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        };

        (
            status,
            Json(serde_json::json!({ "error": self.to_string() })),
        )
            .into_response()
    }
}

/// Tidy a name, or refuse it.
fn clean_name(raw: &str) -> Result<String, Error> {
    let name = raw.trim();

    if name.is_empty() || name.chars().count() > MAX_NAME {
        return Err(Error::InvalidName);
    }

    Ok(name.to_owned())
}

/// Create a workspace in the caller's team.
///
/// # Errors
///
/// Returns an error if the name is unusable or the database refuses.
pub async fn create(
    AxumState(state): AxumState<State>,
    actor: Actor,
    Json(body): Json<NewWorkspace>,
) -> Result<(StatusCode, Json<WorkspaceView>), Error> {
    let name = clean_name(&body.name)?;

    let workspace: WorkspaceView = sqlx::query_as(
        "insert into workspaces (id, team_id, name, created_by)
         values ($1, $2, $3, $4)
         returning id, name, state, created_at",
    )
    .bind(Uuid::now_v7())
    .bind(actor.team_id)
    .bind(&name)
    // Attribution, from the first record onwards.
    .bind(actor.user_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|error| {
        tracing::error!(%error, "creating the workspace");
        Error::Internal
    })?;

    tracing::info!(
        workspace_id = %workspace.id,
        team_id = %actor.team_id,
        "created a workspace"
    );

    Ok((StatusCode::CREATED, Json(workspace)))
}

/// Every workspace the caller's team owns, newest first.
///
/// # Errors
///
/// Returns an error if the database refuses.
pub async fn list(
    AxumState(state): AxumState<State>,
    actor: Actor,
) -> Result<Json<Vec<WorkspaceView>>, Error> {
    let workspaces: Vec<WorkspaceView> = sqlx::query_as(
        "select id, name, state, created_at
           from workspaces
          where team_id = $1
       order by created_at desc, id desc",
    )
    .bind(actor.team_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|error| {
        tracing::error!(%error, "listing workspaces");
        Error::Internal
    })?;

    Ok(Json(workspaces))
}

/// One workspace, if it belongs to the caller's team.
///
/// # Errors
///
/// Returns [`Error::NotFound`] both when no such workspace exists and when it
/// belongs to someone else.
pub async fn get(
    AxumState(state): AxumState<State>,
    actor: Actor,
    Path(id): Path<Uuid>,
) -> Result<Json<WorkspaceView>, Error> {
    // Ask the provider rather than trusting the row: a container can be
    // removed behind our back, and reporting a stale "awake" would be a lie
    // someone acts on. A provider that cannot be reached leaves the stored
    // state alone rather than failing the whole read.
    // Someone looking at a workspace counts as using it, which is what
    // stops it sleeping while a person watches.
    orchestrator::touch(&state.pool, id).await;

    match orchestrator::status(&state.pool, state.compute.as_ref(), actor.team_id, id).await {
        Ok(_) => {}
        Err(orchestrator::Error::NotFound) => return Err(Error::NotFound),
        Err(error) => tracing::warn!(%error, workspace_id = %id, "could not refresh the state"),
    }

    read(&state, actor, id).await
}

impl From<orchestrator::Error> for Error {
    fn from(error: orchestrator::Error) -> Self {
        match error {
            orchestrator::Error::NotFound => Self::NotFound,
            orchestrator::Error::Compute(error) => Self::Compute(error),
            orchestrator::Error::Database(error) => {
                tracing::error!(%error, "the database refused");
                Self::Internal
            }
        }
    }
}

/// Start a workspace, making its computer the first time.
///
/// # Errors
///
/// Returns [`Error::NotFound`] if the workspace is not this team's, or
/// [`Error::Compute`] if the provider refuses.
pub async fn start(
    AxumState(state): AxumState<State>,
    actor: Actor,
    Path(id): Path<Uuid>,
) -> Result<Json<WorkspaceView>, Error> {
    orchestrator::start(
        &state.pool,
        state.compute.as_ref(),
        &state.workspace_relay_url,
        actor.team_id,
        id,
    )
    .await?;

    orchestrator::touch(&state.pool, id).await;

    read(&state, actor, id).await
}

/// Stop a workspace, keeping its disk.
///
/// # Errors
///
/// Returns [`Error::NotFound`] if the workspace is not this team's, or
/// [`Error::Compute`] if the provider refuses.
pub async fn stop(
    AxumState(state): AxumState<State>,
    actor: Actor,
    Path(id): Path<Uuid>,
) -> Result<Json<WorkspaceView>, Error> {
    orchestrator::stop(&state.pool, state.compute.as_ref(), actor.team_id, id).await?;

    read(&state, actor, id).await
}

/// The workspace as it stands now, after the orchestrator has written down
/// what the provider said.
async fn read(state: &State, actor: Actor, id: Uuid) -> Result<Json<WorkspaceView>, Error> {
    let workspace: Option<WorkspaceView> = sqlx::query_as(
        "select id, name, state, created_at
           from workspaces
          where id = $1 and team_id = $2",
    )
    .bind(id)
    .bind(actor.team_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|error| {
        tracing::error!(%error, "reading the workspace");
        Error::Internal
    })?;

    let mut workspace = workspace.ok_or(Error::NotFound)?;
    workspace.connected = state.relay.is_connected(id).await;

    Ok(Json(workspace))
}

/// Run a command inside a workspace.
///
/// # Errors
///
/// [`Error::NotFound`] if the workspace is not this team's, or
/// [`Error::NotConnected`] if its agent is not there — which is a different
/// thing from a failure, and the caller can act on it by waking it.
pub async fn run(
    AxumState(state): AxumState<State>,
    actor: Actor,
    Path(id): Path<Uuid>,
    Json(body): Json<RunCommand>,
) -> Result<Json<RanCommand>, Error> {
    // Scoped first: an id alone must never reach a workspace.
    let _ = read(&state, actor, id).await?;

    let program = body.program.trim();
    if program.is_empty() {
        return Err(Error::InvalidName);
    }

    // Running something is the clearest sign a workspace is in use.
    orchestrator::touch(&state.pool, id).await;

    let ran = state
        .relay
        .run(id, program, &body.args, COMMAND_TIMEOUT)
        .await
        .map_err(|error| match error {
            croncave_relay::Error::NotConnected => Error::NotConnected,
            other => {
                tracing::warn!(%other, workspace_id = %id, "a command went wrong");
                Error::Internal
            }
        })?;

    Ok(Json(RanCommand {
        output: ran.output,
        outcome: ran.outcome.word().to_owned(),
        succeeded: ran.outcome.succeeded(),
    }))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    #[test]
    fn a_name_is_stored_trimmed() {
        assert_eq!(clean_name("  Stock Watcher  ").unwrap(), "Stock Watcher");
    }

    #[test]
    fn a_workspace_needs_a_name() {
        assert!(clean_name("").is_err());
        assert!(clean_name("   ").is_err());
        assert!(clean_name("\t\n").is_err());
    }

    #[test]
    fn a_name_has_a_limit_counted_in_characters_not_bytes() {
        // Emoji are several bytes each; the limit is what a person sees.
        let long_but_fine = "🌙".repeat(MAX_NAME);
        assert!(clean_name(&long_but_fine).is_ok());

        let one_too_many = "🌙".repeat(MAX_NAME + 1);
        assert!(clean_name(&one_too_many).is_err());
    }
}
