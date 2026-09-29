//! Signing in with a link, and the session that follows.
//!
//! There is no password anywhere in Croncave. Someone gives us an email
//! address, we send a link holding a one-time secret, and following it opens
//! a session. The account is created the first time a link is followed.
//!
//! Three rules shape the code below:
//!
//! - **Asking for a link never reveals whether an account exists.** The
//!   response is identical either way, so this endpoint can't be used to find
//!   out who has signed up.
//! - **A link works exactly once.** Consuming it is a single conditional
//!   UPDATE, so two browsers racing the same link cannot both win.
//! - **Nothing usable is stored.** Only hashes of the link and of the session
//!   cookie reach the database.

use std::time::Duration;

use axum::Json;
use axum::extract::{Query, State as AxumState};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::State;
use crate::actor::Actor;
use crate::mail::SignInLink;
use crate::tokens::{self, Token};

/// The cookie holding the session secret.
pub const COOKIE_NAME: &str = "croncave_session";

/// How long a sign-in link stays usable. Long enough to switch to an email
/// client, short enough that a link left in an inbox is not a standing key.
const LINK_TTL: Duration = Duration::from_secs(15 * 60);

/// How long a session lasts without signing in again.
const SESSION_TTL: Duration = Duration::from_secs(30 * 24 * 60 * 60);

/// How many links one address may ask for inside [`RATE_WINDOW`], so the
/// endpoint can't be used to flood someone's inbox.
const MAX_LINKS_PER_EMAIL: i64 = 5;

/// The window the limit above applies to.
const RATE_WINDOW: Duration = Duration::from_secs(15 * 60);

/// What a request to sign in carries.
#[derive(Debug, Deserialize)]
pub struct RequestLink {
    /// The address to send the link to.
    pub email: String,
}

/// What the callback carries.
#[derive(Debug, Deserialize)]
pub struct Callback {
    /// The secret from the emailed link.
    pub token: String,
}

/// Who is signed in, and the team their work belongs to.
#[derive(Debug, Serialize)]
pub struct Me {
    /// The signed-in user.
    pub user: UserView,
    /// Their personal team in R1.
    pub team: TeamView,
}

/// A user, as the app sees one.
#[derive(Debug, Serialize)]
pub struct UserView {
    /// Its id.
    pub id: Uuid,
    /// Its email address.
    pub email: String,
    /// Its display name, if set.
    pub name: Option<String>,
}

/// A team, as the app sees one.
#[derive(Debug, Serialize)]
pub struct TeamView {
    /// Its id.
    pub id: Uuid,
    /// Its name.
    pub name: String,
}

/// What can go wrong in a way the browser should hear about.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The address doesn't look like one.
    #[error("that does not look like an email address")]
    InvalidEmail,

    /// The link is unknown, already used, or past its expiry. Deliberately
    /// one error for all three: which it was is no one's business.
    #[error("this sign-in link is no longer valid")]
    LinkNotUsable,

    /// Nobody is signed in.
    #[error("not signed in")]
    NotSignedIn,

    /// Something on our side failed.
    #[error("something went wrong")]
    Internal,
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let status = match self {
            Self::InvalidEmail => StatusCode::BAD_REQUEST,
            Self::LinkNotUsable => StatusCode::UNAUTHORIZED,
            Self::NotSignedIn => StatusCode::UNAUTHORIZED,
            Self::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        };

        (
            status,
            Json(serde_json::json!({ "error": self.to_string() })),
        )
            .into_response()
    }
}

/// Tidy an address into the one form we store: trimmed and lowercased.
///
/// # Errors
///
/// Returns [`Error::InvalidEmail`] if it cannot be one.
pub fn normalise_email(raw: &str) -> Result<String, Error> {
    let email = raw.trim().to_lowercase();

    let (local, domain) = email.split_once('@').ok_or(Error::InvalidEmail)?;
    let plausible = !local.is_empty()
        && !domain.is_empty()
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && !email.contains(char::is_whitespace)
        && email.len() <= 320;

    if plausible {
        Ok(email)
    } else {
        Err(Error::InvalidEmail)
    }
}

/// Ask for a sign-in link.
///
/// Always answers the same way, whether or not the address has an account.
///
/// # Errors
///
/// Returns an error if the address is malformed, or if the link could not be
/// sent — which is independent of whether the account exists, so it reveals
/// nothing.
pub async fn request_link(
    AxumState(state): AxumState<State>,
    Json(body): Json<RequestLink>,
) -> Result<StatusCode, Error> {
    let email = normalise_email(&body.email)?;

    // Count recent requests before minting another, so one address cannot be
    // used to flood an inbox.
    let recent: i64 = sqlx::query_scalar(
        "select count(*) from login_tokens
         where email = $1 and created_at > $2",
    )
    .bind(&email)
    .bind(OffsetDateTime::now_utc() - RATE_WINDOW)
    .fetch_one(&state.pool)
    .await
    .map_err(|error| {
        tracing::error!(%error, "counting recent sign-in links");
        Error::Internal
    })?;

    if recent >= MAX_LINKS_PER_EMAIL {
        // Answer as though it worked: saying "too many" would confirm the
        // address is being used.
        tracing::warn!(
            recent,
            "refusing to send another sign-in link to this address yet"
        );
        return Ok(StatusCode::ACCEPTED);
    }

    let is_new: bool = !sqlx::query_scalar::<_, bool>("select true from users where email = $1")
        .bind(&email)
        .fetch_optional(&state.pool)
        .await
        .map_err(|error| {
            tracing::error!(%error, "looking up the account");
            Error::Internal
        })?
        .unwrap_or(false);

    let token = Token::generate().map_err(|error| {
        tracing::error!(%error, "the system would not supply randomness");
        Error::Internal
    })?;

    sqlx::query(
        "insert into login_tokens (id, email, token_hash, expires_at)
         values ($1, $2, $3, $4)",
    )
    .bind(Uuid::now_v7())
    .bind(&email)
    .bind(&token.hash)
    .bind(OffsetDateTime::now_utc() + LINK_TTL)
    .execute(&state.pool)
    .await
    .map_err(|error| {
        tracing::error!(%error, "storing the sign-in link");
        Error::Internal
    })?;

    let link = SignInLink {
        email: email.clone(),
        url: format!("{}/auth/callback?token={}", state.app_url, token.secret),
        is_new,
    };

    state
        .mailer
        .send_sign_in_link(&link)
        .await
        .map_err(|error| {
            tracing::error!(%error, "sending the sign-in link");
            Error::Internal
        })?;

    // Nothing here says whether the account existed.
    Ok(StatusCode::ACCEPTED)
}

/// Follow a sign-in link: consume it, sign the person in, send them to the app.
///
/// # Errors
///
/// Returns [`Error::LinkNotUsable`] if the link is unknown, already used or
/// expired — one answer for all three.
pub async fn callback(
    AxumState(state): AxumState<State>,
    jar: CookieJar,
    Query(query): Query<Callback>,
) -> Result<(CookieJar, Redirect), Error> {
    let hash = tokens::hash(&query.token);

    // Consuming and checking in one statement is what makes a link single
    // use: two browsers racing the same link cannot both match `consumed_at
    // is null`.
    let email: Option<String> = sqlx::query_scalar(
        "update login_tokens
            set consumed_at = now()
          where token_hash = $1
            and consumed_at is null
            and expires_at > now()
      returning email",
    )
    .bind(&hash)
    .fetch_optional(&state.pool)
    .await
    .map_err(|error| {
        tracing::error!(%error, "consuming the sign-in link");
        Error::Internal
    })?;

    let Some(email) = email else {
        tracing::info!("a sign-in link was unknown, already used or expired");
        return Err(Error::LinkNotUsable);
    };

    let user_id = find_or_create_account(&state, &email).await?;
    let session = open_session(&state, user_id).await?;

    tracing::info!(%user_id, "signed in");

    Ok((
        jar.add(session_cookie(&state, session)),
        Redirect::to(&state.app_url),
    ))
}

/// Sign out of this browser, leaving any other session alone.
///
/// # Errors
///
/// Returns an error only if the database is unreachable. Signing out when
/// already signed out succeeds.
pub async fn sign_out(
    AxumState(state): AxumState<State>,
    jar: CookieJar,
) -> Result<(CookieJar, StatusCode), Error> {
    if let Some(cookie) = jar.get(COOKIE_NAME) {
        sqlx::query("delete from sessions where token_hash = $1")
            .bind(tokens::hash(cookie.value()))
            .execute(&state.pool)
            .await
            .map_err(|error| {
                tracing::error!(%error, "ending the session");
                Error::Internal
            })?;
    }

    // Remove the cookie whether or not it matched anything.
    let mut removal = Cookie::from(COOKIE_NAME);
    removal.set_path("/");

    Ok((jar.remove(removal), StatusCode::NO_CONTENT))
}

/// Who is signed in.
///
/// # Errors
///
/// Returns [`Error::NotSignedIn`] if the cookie is missing, unknown or
/// expired.
pub async fn me(AxumState(state): AxumState<State>, actor: Actor) -> Result<Json<Me>, Error> {
    let row: Option<(String, Option<String>, String)> = sqlx::query_as(
        "select users.email, users.name, teams.name
           from users
           join teams on teams.id = $2
          where users.id = $1",
    )
    .bind(actor.user_id)
    .bind(actor.team_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|error| {
        tracing::error!(%error, "reading the account");
        Error::Internal
    })?;

    // The extractor already proved the session; a missing row here would mean
    // the account vanished mid-request.
    let Some((email, name, team_name)) = row else {
        return Err(Error::NotSignedIn);
    };

    Ok(Json(Me {
        user: UserView {
            id: actor.user_id,
            email,
            name,
        },
        team: TeamView {
            id: actor.team_id,
            name: team_name,
        },
    }))
}

/// Find the account for an address, creating it and its personal team the
/// first time. One transaction, so a half-made account cannot exist.
async fn find_or_create_account(state: &State, email: &str) -> Result<Uuid, Error> {
    let mut tx = state.pool.begin().await.map_err(|error| {
        tracing::error!(%error, "starting a transaction");
        Error::Internal
    })?;

    let existing: Option<Uuid> = sqlx::query_scalar("select id from users where email = $1")
        .bind(email)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|error| {
            tracing::error!(%error, "looking up the account");
            Error::Internal
        })?;

    if let Some(user_id) = existing {
        return Ok(user_id);
    }

    let user_id = Uuid::now_v7();
    let team_id = Uuid::now_v7();

    sqlx::query("insert into users (id, email) values ($1, $2)")
        .bind(user_id)
        .bind(email)
        .execute(&mut *tx)
        .await
        .map_err(|error| {
            tracing::error!(%error, "creating the account");
            Error::Internal
        })?;

    // Everything belongs to a team, so a team of one exists from the first
    // moment. It becomes a shared team later without a migration.
    sqlx::query("insert into teams (id, name, kind) values ($1, $2, 'personal')")
        .bind(team_id)
        .bind(email)
        .execute(&mut *tx)
        .await
        .map_err(|error| {
            tracing::error!(%error, "creating the personal team");
            Error::Internal
        })?;

    sqlx::query(
        "insert into memberships (id, user_id, team_id, role) values ($1, $2, $3, 'owner')",
    )
    .bind(Uuid::now_v7())
    .bind(user_id)
    .bind(team_id)
    .execute(&mut *tx)
    .await
    .map_err(|error| {
        tracing::error!(%error, "joining the account to its team");
        Error::Internal
    })?;

    tx.commit().await.map_err(|error| {
        tracing::error!(%error, "committing the new account");
        Error::Internal
    })?;

    tracing::info!(%user_id, %team_id, "created an account and its personal team");

    Ok(user_id)
}

/// Open a session and return the secret that names it.
async fn open_session(state: &State, user_id: Uuid) -> Result<String, Error> {
    let token = Token::generate().map_err(|error| {
        tracing::error!(%error, "the system would not supply randomness");
        Error::Internal
    })?;

    sqlx::query(
        "insert into sessions (id, user_id, token_hash, expires_at)
         values ($1, $2, $3, $4)",
    )
    .bind(Uuid::now_v7())
    .bind(user_id)
    .bind(&token.hash)
    .bind(OffsetDateTime::now_utc() + SESSION_TTL)
    .execute(&state.pool)
    .await
    .map_err(|error| {
        tracing::error!(%error, "opening the session");
        Error::Internal
    })?;

    Ok(token.secret)
}

/// The cookie carrying a session.
fn session_cookie(state: &State, secret: String) -> Cookie<'static> {
    let mut cookie = Cookie::new(COOKIE_NAME, secret);
    // Script can never read it, so a cross-site scripting bug cannot steal it.
    cookie.set_http_only(true);
    // Lax, not Strict: following a sign-in link from an email client is a
    // cross-site navigation, and Strict would drop the cookie on arrival.
    cookie.set_same_site(SameSite::Lax);
    cookie.set_path("/");
    // HTTPS only anywhere real. Local development has no certificate.
    cookie.set_secure(state.secure_cookies);
    cookie.set_max_age(time::Duration::seconds(
        SESSION_TTL.as_secs().try_into().unwrap_or(i64::MAX),
    ));
    cookie
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    #[test]
    fn an_address_is_stored_trimmed_and_lowercased() {
        assert_eq!(
            normalise_email("  Founder@Example.COM ").unwrap(),
            "founder@example.com"
        );
    }

    #[test]
    fn things_that_are_not_addresses_are_refused() {
        for bad in [
            "",
            "founder",
            "@example.com",
            "founder@",
            "founder@example",
            "founder@.com",
            "founder@example.",
            "foun der@example.com",
        ] {
            assert!(
                normalise_email(bad).is_err(),
                "{bad:?} should not be accepted"
            );
        }
    }

    #[test]
    fn a_plus_address_is_kept() {
        assert_eq!(
            normalise_email("founder+croncave@example.com").unwrap(),
            "founder+croncave@example.com"
        );
    }
}
