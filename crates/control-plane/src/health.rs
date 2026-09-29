//! Is the process up, and can it do its job?
//!
//! Two questions, deliberately separate. `/health` answers the first: the
//! process is running and should not be restarted. `/ready` answers the
//! second: its dependencies are reachable, so it should receive traffic. A
//! database blip should take a service out of rotation, not kill it.

use axum::Json;
use axum::extract::State as AxumState;
use axum::http::StatusCode;
use serde::Serialize;

use crate::{READY_TIMEOUT, State};

/// The body both checks return.
#[derive(Debug, Serialize)]
pub struct Health {
    /// "ok" or "down".
    status: &'static str,
    /// The build serving this request.
    version: &'static str,
}

impl Health {
    fn ok() -> Self {
        Self {
            status: "ok",
            version: env!("CARGO_PKG_VERSION"),
        }
    }

    fn down() -> Self {
        Self {
            status: "down",
            version: env!("CARGO_PKG_VERSION"),
        }
    }
}

/// Liveness: the process is running. Touches nothing else on purpose.
pub async fn health() -> Json<Health> {
    Json(Health::ok())
}

/// Readiness: the database answers, so this instance can serve traffic.
pub async fn ready(AxumState(state): AxumState<State>) -> (StatusCode, Json<Health>) {
    let query = sqlx::query_scalar::<_, i32>("select 1").fetch_one(&state.pool);

    match tokio::time::timeout(READY_TIMEOUT, query).await {
        Ok(Ok(_)) => (StatusCode::OK, Json(Health::ok())),
        Ok(Err(error)) => {
            tracing::warn!(%error, "not ready: the database refused a query");
            (StatusCode::SERVICE_UNAVAILABLE, Json(Health::down()))
        }
        Err(_) => {
            tracing::warn!(
                timeout_ms = READY_TIMEOUT.as_millis(),
                "not ready: the database did not answer in time"
            );
            (StatusCode::SERVICE_UNAVAILABLE, Json(Health::down()))
        }
    }
}
