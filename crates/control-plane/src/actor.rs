//! Who is making a request, and which team their work belongs to.
//!
//! Every handler that touches real records takes an [`Actor`]. Asking for one
//! is what enforces sign-in, and carrying the team with it is what makes
//! scoping a query the default rather than something to remember: a handler
//! that forgets `where team_id = $1` has to go out of its way to get the
//! team's id in the first place.

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum_extra::extract::cookie::CookieJar;
use uuid::Uuid;

use crate::auth::{COOKIE_NAME, Error};
use crate::{State, tokens};

/// A signed-in person, and the team that owns what they do.
#[derive(Debug, Clone, Copy)]
pub struct Actor {
    /// Who they are. Recorded on everything they create.
    pub user_id: Uuid,
    /// Their team in R1: a personal team of one.
    pub team_id: Uuid,
}

impl FromRequestParts<State> for Actor {
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &State) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_headers(&parts.headers);
        let cookie = jar.get(COOKIE_NAME).ok_or(Error::NotSignedIn)?;

        let row: Option<(Uuid, Uuid)> = sqlx::query_as(
            "select users.id, teams.id
               from sessions
               join users on users.id = sessions.user_id
               join memberships on memberships.user_id = users.id
               join teams on teams.id = memberships.team_id
              where sessions.token_hash = $1
                and sessions.expires_at > now()
                and teams.kind = 'personal'
              limit 1",
        )
        .bind(tokens::hash(cookie.value()))
        .fetch_optional(&state.pool)
        .await
        .map_err(|error| {
            tracing::error!(%error, "reading the session");
            Error::Internal
        })?;

        let (user_id, team_id) = row.ok_or(Error::NotSignedIn)?;

        Ok(Self { user_id, team_id })
    }
}
