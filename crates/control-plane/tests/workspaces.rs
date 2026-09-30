//! Creating and reading workspaces, and — the part that matters — not being
//! able to read anyone else's.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use croncave_compute::fake::FakeDriver;
use croncave_control_plane::auth::COOKIE_NAME;
use croncave_control_plane::mail::TestMailer;
use croncave_control_plane::{State, router};
use croncave_db::Pool;
use serde_json::{Value, json};
use tower::ServiceExt as _;

fn app(pool: Pool) -> (axum::Router, TestMailer) {
    let (router, mailer, _) = app_with_compute(pool);
    (router, mailer)
}

/// The app, plus the compute provider behind it, for tests that need to
/// interfere with the provider.
fn app_with_compute(pool: Pool) -> (axum::Router, TestMailer, FakeDriver) {
    let mailer = TestMailer::default();
    let compute = FakeDriver::new();
    let pool_for_relay = pool.clone();
    let state = State {
        pool,
        mailer: Arc::new(mailer.clone()),
        app_url: "http://localhost:5173".to_owned(),
        secure_cookies: false,
        compute: Arc::new(compute.clone()),
        relay: croncave_relay::Relay::new(Arc::new(
            croncave_control_plane::agents::DatabaseAuthoriser::new(pool_for_relay),
        )),
        workspace_relay_url: "ws://host.docker.internal:8080/agent".to_owned(),
    };
    (router(state), mailer, compute)
}

/// Sign someone in and return their session cookie.
async fn sign_in(app: &axum::Router, mailer: &TestMailer, email: &str) -> String {
    let asked = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/request-link")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({ "email": email }).to_string()))
                .expect("build request"),
        )
        .await
        .expect("call");
    assert_eq!(asked.status(), StatusCode::ACCEPTED);

    let url = mailer.last().expect("a link").url;
    let query = url.split_once("/auth/callback").expect("a callback").1;

    let followed = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/auth/callback{query}"))
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("call");

    followed
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .find(|v| v.starts_with(COOKIE_NAME))
        .and_then(|v| v.split(';').next())
        .and_then(|v| v.split_once('='))
        .map(|(_, value)| value.to_owned())
        .expect("a session cookie")
}

async fn send(
    app: &axum::Router,
    method: &str,
    path: &str,
    cookie: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut request = Request::builder().method(method).uri(path);
    if let Some(cookie) = cookie {
        request = request.header(header::COOKIE, format!("{COOKIE_NAME}={cookie}"));
    }

    let request = match body {
        Some(body) => request
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string())),
        None => request.body(Body::empty()),
    }
    .expect("build request");

    let response = app.clone().oneshot(request).await.expect("call");
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 64 * 1024)
        .await
        .expect("read body");

    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[sqlx::test(migrations = "../db/migrations")]
async fn a_new_workspace_belongs_to_its_team_and_starts_asleep(pool: Pool) {
    let (app, mailer) = app(pool);
    let cookie = sign_in(&app, &mailer, "founder@example.com").await;

    let (status, body) = send(
        &app,
        "POST",
        "/workspaces",
        Some(&cookie),
        Some(json!({ "name": "  Stock Watcher  " })),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["name"], "Stock Watcher", "the name is trimmed");
    assert_eq!(body["state"], "asleep", "it has no computer until step 2");
    assert!(body["id"].is_string());
    assert!(body["created_at"].is_string());
}

#[sqlx::test(migrations = "../db/migrations")]
async fn a_workspace_records_who_made_it(pool: Pool) {
    let (app, mailer) = app(pool.clone());
    let cookie = sign_in(&app, &mailer, "founder@example.com").await;

    send(
        &app,
        "POST",
        "/workspaces",
        Some(&cookie),
        Some(json!({ "name": "Stock Watcher" })),
    )
    .await;

    let (email,): (String,) = sqlx::query_as(
        "select users.email from workspaces
           join users on users.id = workspaces.created_by",
    )
    .fetch_one(&pool)
    .await
    .expect("read attribution");

    assert_eq!(email, "founder@example.com");
}

#[sqlx::test(migrations = "../db/migrations")]
async fn workspaces_are_listed_newest_first(pool: Pool) {
    let (app, mailer) = app(pool);
    let cookie = sign_in(&app, &mailer, "founder@example.com").await;

    for name in ["First", "Second", "Third"] {
        send(
            &app,
            "POST",
            "/workspaces",
            Some(&cookie),
            Some(json!({ "name": name })),
        )
        .await;
    }

    let (status, body) = send(&app, "GET", "/workspaces", Some(&cookie), None).await;

    assert_eq!(status, StatusCode::OK);
    let names: Vec<&str> = body
        .as_array()
        .expect("a list")
        .iter()
        .map(|w| w["name"].as_str().expect("a name"))
        .collect();
    assert_eq!(names, ["Third", "Second", "First"]);
}

#[sqlx::test(migrations = "../db/migrations")]
async fn one_team_never_sees_another_teams_workspaces(pool: Pool) {
    let (app, mailer) = app(pool);

    let founder = sign_in(&app, &mailer, "founder@example.com").await;
    send(
        &app,
        "POST",
        "/workspaces",
        Some(&founder),
        Some(json!({ "name": "Private Plans" })),
    )
    .await;

    let stranger = sign_in(&app, &mailer, "stranger@example.com").await;
    let (status, body) = send(&app, "GET", "/workspaces", Some(&stranger), None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body.as_array().expect("a list").len(),
        0,
        "a different team owns nothing here"
    );
}

#[sqlx::test(migrations = "../db/migrations")]
async fn asking_for_another_teams_workspace_by_id_says_not_found(pool: Pool) {
    let (app, mailer) = app(pool);

    let founder = sign_in(&app, &mailer, "founder@example.com").await;
    let (_, made) = send(
        &app,
        "POST",
        "/workspaces",
        Some(&founder),
        Some(json!({ "name": "Private Plans" })),
    )
    .await;
    let id = made["id"].as_str().expect("an id");

    // The owner can read it.
    let (mine, _) = send(
        &app,
        "GET",
        &format!("/workspaces/{id}"),
        Some(&founder),
        None,
    )
    .await;
    assert_eq!(mine, StatusCode::OK);

    // Someone else, holding the exact id, gets nothing — and "not found"
    // rather than "not allowed", which would confirm it exists.
    let stranger = sign_in(&app, &mailer, "stranger@example.com").await;
    let (theirs, body) = send(
        &app,
        "GET",
        &format!("/workspaces/{id}"),
        Some(&stranger),
        None,
    )
    .await;

    assert_eq!(theirs, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "no such workspace");
}

#[sqlx::test(migrations = "../db/migrations")]
async fn a_workspace_that_does_not_exist_says_not_found(pool: Pool) {
    let (app, mailer) = app(pool);
    let cookie = sign_in(&app, &mailer, "founder@example.com").await;

    let (status, _) = send(
        &app,
        "GET",
        "/workspaces/01920000-0000-7000-8000-000000000000",
        Some(&cookie),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test(migrations = "../db/migrations")]
async fn signed_out_you_cannot_see_or_make_anything(pool: Pool) {
    let (app, _) = app(pool);

    let (listed, _) = send(&app, "GET", "/workspaces", None, None).await;
    assert_eq!(listed, StatusCode::UNAUTHORIZED);

    let (created, _) = send(
        &app,
        "POST",
        "/workspaces",
        None,
        Some(json!({ "name": "Sneaky" })),
    )
    .await;
    assert_eq!(created, StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "../db/migrations")]
async fn a_workspace_without_a_name_is_refused(pool: Pool) {
    let (app, mailer) = app(pool);
    let cookie = sign_in(&app, &mailer, "founder@example.com").await;

    let (status, _) = send(
        &app,
        "POST",
        "/workspaces",
        Some(&cookie),
        Some(json!({ "name": "   " })),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrations = "../db/migrations")]
async fn a_signed_out_session_stops_working_at_once(pool: Pool) {
    let (app, mailer) = app(pool);
    let cookie = sign_in(&app, &mailer, "founder@example.com").await;

    send(&app, "POST", "/auth/sign-out", Some(&cookie), None).await;

    let (status, _) = send(&app, "GET", "/workspaces", Some(&cookie), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "../db/migrations")]
async fn a_workspace_starts_and_stops(pool: Pool) {
    let (app, mailer, compute) = app_with_compute(pool);
    let cookie = sign_in(&app, &mailer, "founder@example.com").await;
    let (_, made) = send(
        &app,
        "POST",
        "/workspaces",
        Some(&cookie),
        Some(json!({ "name": "Stock Watcher" })),
    )
    .await;
    let id = made["id"].as_str().expect("an id");

    assert_eq!(
        compute.count(),
        0,
        "a workspace is a record until it is run"
    );

    let (status, body) = send(
        &app,
        "POST",
        &format!("/workspaces/{id}/start"),
        Some(&cookie),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["state"], "awake");
    assert_eq!(compute.count(), 1, "one computer, made on first start");

    let (status, body) = send(
        &app,
        "POST",
        &format!("/workspaces/{id}/stop"),
        Some(&cookie),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["state"], "asleep");
}

#[sqlx::test(migrations = "../db/migrations")]
async fn starting_twice_makes_one_computer(pool: Pool) {
    let (app, mailer, compute) = app_with_compute(pool);
    let cookie = sign_in(&app, &mailer, "founder@example.com").await;
    let (_, made) = send(
        &app,
        "POST",
        "/workspaces",
        Some(&cookie),
        Some(json!({ "name": "Stock Watcher" })),
    )
    .await;
    let id = made["id"].as_str().expect("an id");

    for _ in 0..3 {
        let (status, _) = send(
            &app,
            "POST",
            &format!("/workspaces/{id}/start"),
            Some(&cookie),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
    }

    assert_eq!(
        compute.count(),
        1,
        "pressing start again must not leave computers behind"
    );
}

#[sqlx::test(migrations = "../db/migrations")]
async fn a_computer_removed_behind_our_back_is_reported_honestly(pool: Pool) {
    let (app, mailer, compute) = app_with_compute(pool);
    let cookie = sign_in(&app, &mailer, "founder@example.com").await;
    let (_, made) = send(
        &app,
        "POST",
        "/workspaces",
        Some(&cookie),
        Some(json!({ "name": "Stock Watcher" })),
    )
    .await;
    let id = made["id"].as_str().expect("an id");

    send(
        &app,
        "POST",
        &format!("/workspaces/{id}/start"),
        Some(&cookie),
        None,
    )
    .await;

    // Something outside Croncave removes it.
    compute.destroy_all();

    let (status, body) = send(
        &app,
        "GET",
        &format!("/workspaces/{id}"),
        Some(&cookie),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body["state"], "asleep",
        "reporting a stale awake would be a lie someone acts on"
    );

    // And starting it again just works, rather than failing for ever on a
    // pointer to something that no longer exists.
    let (status, body) = send(
        &app,
        "POST",
        &format!("/workspaces/{id}/start"),
        Some(&cookie),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["state"], "awake");
}

#[sqlx::test(migrations = "../db/migrations")]
async fn one_team_cannot_start_another_teams_workspace(pool: Pool) {
    let (app, mailer, compute) = app_with_compute(pool);

    let founder = sign_in(&app, &mailer, "founder@example.com").await;
    let (_, made) = send(
        &app,
        "POST",
        "/workspaces",
        Some(&founder),
        Some(json!({ "name": "Private Plans" })),
    )
    .await;
    let id = made["id"].as_str().expect("an id");

    let stranger = sign_in(&app, &mailer, "stranger@example.com").await;
    let (status, _) = send(
        &app,
        "POST",
        &format!("/workspaces/{id}/start"),
        Some(&stranger),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(compute.count(), 0, "and nothing was made for them");
}

#[sqlx::test(migrations = "../db/migrations")]
async fn a_provider_having_a_bad_day_is_reported_as_such(pool: Pool) {
    let (app, mailer, compute) = app_with_compute(pool);
    let cookie = sign_in(&app, &mailer, "founder@example.com").await;
    let (_, made) = send(
        &app,
        "POST",
        "/workspaces",
        Some(&cookie),
        Some(json!({ "name": "Stock Watcher" })),
    )
    .await;
    let id = made["id"].as_str().expect("an id");

    compute.break_it();

    let (status, _) = send(
        &app,
        "POST",
        &format!("/workspaces/{id}/start"),
        Some(&cookie),
        None,
    )
    .await;

    // Not a 500: the request was fine, the provider is the problem, and the
    // difference is what tells someone whether to retry or to look at Docker.
    assert_eq!(status, StatusCode::BAD_GATEWAY);
}

#[sqlx::test(migrations = "../db/migrations")]
async fn a_workspace_left_alone_goes_to_sleep(pool: Pool) {
    let (app, mailer, compute) = app_with_compute(pool.clone());
    let cookie = sign_in(&app, &mailer, "founder@example.com").await;
    let (_, made) = send(
        &app,
        "POST",
        "/workspaces",
        Some(&cookie),
        Some(json!({ "name": "Stock Watcher" })),
    )
    .await;
    let id = made["id"].as_str().expect("an id");

    send(
        &app,
        "POST",
        &format!("/workspaces/{id}/start"),
        Some(&cookie),
        None,
    )
    .await;

    // Nothing has happened for a while. Reaching past the clock beats
    // waiting ten minutes.
    sqlx::query("update workspaces set last_active_at = now() - interval '1 hour'")
        .execute(&pool)
        .await
        .expect("age the workspace");

    let slept = croncave_control_plane::orchestrator::sleep_idle(
        &pool,
        &compute,
        std::time::Duration::from_secs(600),
    )
    .await
    .expect("sweep");

    assert_eq!(slept, 1);

    let (_, body) = send(
        &app,
        "GET",
        &format!("/workspaces/{id}"),
        Some(&cookie),
        None,
    )
    .await;
    assert_eq!(body["state"], "asleep");
}

#[sqlx::test(migrations = "../db/migrations")]
async fn a_workspace_in_use_is_left_alone(pool: Pool) {
    let (app, mailer, compute) = app_with_compute(pool.clone());
    let cookie = sign_in(&app, &mailer, "founder@example.com").await;
    let (_, made) = send(
        &app,
        "POST",
        "/workspaces",
        Some(&cookie),
        Some(json!({ "name": "Stock Watcher" })),
    )
    .await;
    let id = made["id"].as_str().expect("an id");

    send(
        &app,
        "POST",
        &format!("/workspaces/{id}/start"),
        Some(&cookie),
        None,
    )
    .await;

    // Starting it counts as using it, so a sweep straight afterwards must
    // not take it away from under someone.
    let slept = croncave_control_plane::orchestrator::sleep_idle(
        &pool,
        &compute,
        std::time::Duration::from_secs(600),
    )
    .await
    .expect("sweep");

    assert_eq!(slept, 0, "a workspace someone is using must stay awake");

    let (_, body) = send(
        &app,
        "GET",
        &format!("/workspaces/{id}"),
        Some(&cookie),
        None,
    )
    .await;
    assert_eq!(body["state"], "awake");
}

#[sqlx::test(migrations = "../db/migrations")]
async fn running_something_in_a_workspace_that_is_asleep_says_so(pool: Pool) {
    let (app, mailer) = app(pool);
    let cookie = sign_in(&app, &mailer, "founder@example.com").await;
    let (_, made) = send(
        &app,
        "POST",
        "/workspaces",
        Some(&cookie),
        Some(json!({ "name": "Stock Watcher" })),
    )
    .await;
    let id = made["id"].as_str().expect("an id");

    let (status, _) = send(
        &app,
        "POST",
        &format!("/workspaces/{id}/run"),
        Some(&cookie),
        Some(json!({ "program": "echo", "args": ["hello"] })),
    )
    .await;

    // Not a 500 and not a hang: there is simply nothing there to ask, and
    // the answer tells a person to wake it.
    assert_eq!(status, StatusCode::CONFLICT);
}

#[sqlx::test(migrations = "../db/migrations")]
async fn one_team_cannot_run_anything_in_another_teams_workspace(pool: Pool) {
    let (app, mailer) = app(pool);

    let founder = sign_in(&app, &mailer, "founder@example.com").await;
    let (_, made) = send(
        &app,
        "POST",
        "/workspaces",
        Some(&founder),
        Some(json!({ "name": "Private Plans" })),
    )
    .await;
    let id = made["id"].as_str().expect("an id");

    let stranger = sign_in(&app, &mailer, "stranger@example.com").await;
    let (status, _) = send(
        &app,
        "POST",
        &format!("/workspaces/{id}/run"),
        Some(&stranger),
        Some(json!({ "program": "cat", "args": ["/etc/passwd"] })),
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}
