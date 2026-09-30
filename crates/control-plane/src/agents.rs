//! Proving which workspace an agent belongs to.
//!
//! The orchestrator puts a one-time bootstrap token inside a workspace when
//! it starts it. The agent trades that for a credential, and uses the
//! credential from then on. Neither is stored — only its hash — so reading
//! the database lets nobody pretend to be a workspace.
//!
//! This is the control plane's half of the relay's [`Authoriser`]: the relay
//! holds connections and knows nothing about who owns them, and this answers
//! that question without the relay ever touching the schema.

use std::time::Duration;

use croncave_db::Pool;
use croncave_protocol::{Credential, Refusal};
use croncave_relay::{Authorised, Authoriser};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::tokens::{self, Token};

/// How long a workspace has to use its bootstrap token.
///
/// Long enough for a slow image pull and a slow boot, short enough that a
/// token left in a stopped container is not a standing key.
const BOOTSTRAP_TTL: Duration = Duration::from_secs(10 * 60);

/// How long a credential lasts. Renewal while a connection is up belongs
/// with the long sessions of step 4; for now a reconnection after this asks
/// for a fresh bootstrap token, which the orchestrator mints on every start.
const CREDENTIAL_TTL: Duration = Duration::from_secs(24 * 60 * 60);

/// Mint a bootstrap token for a workspace about to start.
///
/// The caller puts the returned string inside the workspace. It is never
/// stored and cannot be read back.
///
/// # Errors
///
/// Returns an error if randomness or the database is unavailable.
pub async fn mint_bootstrap(pool: &Pool, workspace_id: Uuid) -> Result<String, sqlx::Error> {
    let token = Token::generate().map_err(|error| sqlx::Error::Io(std::io::Error::other(error)))?;

    // Any token this workspace had before is spent: a fresh start gets a
    // fresh identity, and an old container coming back cannot rejoin.
    sqlx::query("delete from workspace_tokens where workspace_id = $1")
        .bind(workspace_id)
        .execute(pool)
        .await?;

    sqlx::query(
        "insert into workspace_tokens (id, workspace_id, kind, token_hash, expires_at)
         values ($1, $2, 'bootstrap', $3, $4)",
    )
    .bind(Uuid::now_v7())
    .bind(workspace_id)
    .bind(&token.hash)
    .bind(OffsetDateTime::now_utc() + BOOTSTRAP_TTL)
    .execute(pool)
    .await?;

    Ok(token.secret)
}

/// Answers the relay's question: whose connection is this?
#[derive(Debug, Clone)]
pub struct DatabaseAuthoriser {
    pool: Pool,
}

impl DatabaseAuthoriser {
    /// An authoriser backed by the workspace records.
    #[must_use]
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl Authoriser for DatabaseAuthoriser {
    async fn authorise(&self, credential: &Credential) -> Result<Authorised, Refusal> {
        match credential {
            Credential::Bootstrap { token } => self.trade_bootstrap(token).await,
            Credential::Workspace { token } => self.check_credential(token).await,
        }
    }
}

impl DatabaseAuthoriser {
    /// Spend a bootstrap token and issue a credential in its place.
    ///
    /// Consuming and checking are one statement, so two agents racing the
    /// same token cannot both win — the same shape as a sign-in link.
    async fn trade_bootstrap(&self, token: &str) -> Result<Authorised, Refusal> {
        let workspace_id: Option<Uuid> = sqlx::query_scalar(
            "update workspace_tokens
                set consumed_at = now()
              where token_hash = $1
                and kind = 'bootstrap'
                and consumed_at is null
                and expires_at > now()
          returning workspace_id",
        )
        .bind(tokens::hash(token))
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| {
            tracing::error!(%error, "reading a bootstrap token");
            Refusal::Unavailable
        })?;

        let Some(workspace_id) = workspace_id else {
            tracing::info!("a bootstrap token was unknown, used or expired");
            return Err(Refusal::NotAuthorised);
        };

        let credential = Token::generate().map_err(|_| Refusal::Unavailable)?;

        sqlx::query(
            "insert into workspace_tokens (id, workspace_id, kind, token_hash, expires_at)
             values ($1, $2, 'credential', $3, $4)",
        )
        .bind(Uuid::now_v7())
        .bind(workspace_id)
        .bind(&credential.hash)
        .bind(OffsetDateTime::now_utc() + CREDENTIAL_TTL)
        .execute(&self.pool)
        .await
        .map_err(|error| {
            tracing::error!(%error, "issuing a credential");
            Refusal::Unavailable
        })?;

        tracing::info!(%workspace_id, "a workspace traded its bootstrap token");

        Ok(Authorised {
            workspace_id,
            fresh_credential: Some(credential.secret),
        })
    }

    /// Check a credential the agent already had.
    async fn check_credential(&self, token: &str) -> Result<Authorised, Refusal> {
        let workspace_id: Option<Uuid> = sqlx::query_scalar(
            "select workspace_id from workspace_tokens
              where token_hash = $1
                and kind = 'credential'
                and expires_at > now()",
        )
        .bind(tokens::hash(token))
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| {
            tracing::error!(%error, "reading a credential");
            Refusal::Unavailable
        })?;

        match workspace_id {
            Some(workspace_id) => Ok(Authorised {
                workspace_id,
                fresh_credential: None,
            }),
            None => Err(Refusal::NotAuthorised),
        }
    }
}
