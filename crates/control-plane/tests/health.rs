//! The two health checks, exercised through the real router.
//!
//! `oneshot` drives the application without binding a port, so these are
//! fast and never fight over one.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use axum::body::Body;
use axum::http::{Request, StatusCode};
use croncave_compute::fake::FakeDriver;
use croncave_control_plane::mail::TestMailer;
use croncave_control_plane::{State, router};
use croncave_db::Pool;
use serde_json::Value;
use std::sync::Arc;
use tower::ServiceExt as _;

fn state(pool: Pool) -> State {
    let pool_for_relay = pool.clone();
    State {
        pool,
        mailer: Arc::new(TestMailer::default()),
        app_url: "http://localhost:5173".to_owned(),
        secure_cookies: false,
        compute: Arc::new(FakeDriver::new()),
        relay: croncave_relay::Relay::new(Arc::new(
            croncave_control_plane::agents::DatabaseAuthoriser::new(pool_for_relay),
        )),
        workspace_relay_url: "ws://host.docker.internal:8080/agent".to_owned(),
    }
}

async fn get(pool: Pool, path: &str) -> (StatusCode, Value) {
    let response = router(state(pool))
        .oneshot(
            Request::builder()
                .uri(path)
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("call the router");

    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 64 * 1024)
        .await
        .expect("read body");

    (status, serde_json::from_slice(&bytes).expect("parse JSON"))
}

#[sqlx::test(migrations = "../db/migrations")]
async fn health_says_the_process_is_up(pool: Pool) {
    let (status, body) = get(pool, "/health").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
    assert_eq!(body["version"], env!("CARGO_PKG_VERSION"));
}

#[sqlx::test(migrations = "../db/migrations")]
async fn ready_says_the_database_answers(pool: Pool) {
    let (status, body) = get(pool, "/ready").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
}

#[sqlx::test(migrations = "../db/migrations")]
async fn ready_reports_down_when_the_database_has_gone(pool: Pool) {
    // Closing the pool is the closest thing to the database going away
    // without taking the test's server with it.
    pool.close().await;

    let (status, body) = get(pool, "/ready").await;

    assert_eq!(
        status,
        StatusCode::SERVICE_UNAVAILABLE,
        "a database blip should take an instance out of rotation"
    );
    assert_eq!(body["status"], "down");
}

#[sqlx::test(migrations = "../db/migrations")]
async fn health_still_says_up_when_the_database_has_gone(pool: Pool) {
    pool.close().await;

    let (status, body) = get(pool, "/health").await;

    // Liveness must not depend on the database, or a blip would have every
    // instance restarted at once.
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
}

#[sqlx::test(migrations = "../db/migrations")]
async fn an_unknown_path_is_not_found(pool: Pool) {
    let response = router(state(pool))
        .oneshot(
            Request::builder()
                .uri("/nope")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("call the router");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
