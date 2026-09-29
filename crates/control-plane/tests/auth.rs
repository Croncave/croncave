//! Signing in, end to end through the real router.
//!
//! These are the tests that matter most in step 1: a sign-in link is a
//! credential, and the promises below are what stop it behaving like one for
//! longer than it should.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use croncave_control_plane::auth::COOKIE_NAME;
use croncave_control_plane::mail::TestMailer;
use croncave_control_plane::{State, router};
use croncave_db::Pool;
use serde_json::{Value, json};
use tower::ServiceExt as _;

/// A router plus the mailer it sends through, so a test can read the link.
fn app(pool: Pool) -> (axum::Router, TestMailer) {
    let mailer = TestMailer::default();
    let state = State {
        pool,
        mailer: Arc::new(mailer.clone()),
        app_url: "http://localhost:5173".to_owned(),
        secure_cookies: false,
    };
    (router(state), mailer)
}

async fn post_json(app: &axum::Router, path: &str, body: Value) -> StatusCode {
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(path)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .expect("build request"),
        )
        .await
        .expect("call")
        .status()
}

/// Follow a link and return the status plus the session cookie it set.
async fn follow(app: &axum::Router, url: &str) -> (StatusCode, Option<String>) {
    let path = url.split_once("/auth/callback").expect("a callback link").1;
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/auth/callback{path}"))
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("call");

    let status = response.status();
    let cookie = response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .find(|value| value.starts_with(COOKIE_NAME))
        .map(std::borrow::ToOwned::to_owned);

    (status, cookie)
}

/// Just the cookie's value, for sending back.
fn cookie_value(set_cookie: &str) -> String {
    set_cookie
        .split(';')
        .next()
        .expect("a cookie")
        .split_once('=')
        .expect("name=value")
        .1
        .to_owned()
}

async fn me(app: &axum::Router, cookie: Option<&str>) -> (StatusCode, Value) {
    let mut request = Request::builder().uri("/me");
    if let Some(cookie) = cookie {
        request = request.header(header::COOKIE, format!("{COOKIE_NAME}={cookie}"));
    }

    let response = app
        .clone()
        .oneshot(request.body(Body::empty()).expect("build request"))
        .await
        .expect("call");

    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 64 * 1024)
        .await
        .expect("read body");

    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

/// Ask for a link and return it.
async fn request_link(app: &axum::Router, mailer: &TestMailer, email: &str) -> String {
    let status = post_json(app, "/auth/request-link", json!({ "email": email })).await;
    assert_eq!(status, StatusCode::ACCEPTED);
    mailer.last().expect("a link was sent").url
}

#[sqlx::test(migrations = "../db/migrations")]
async fn signing_in_for_the_first_time_creates_an_account_and_its_team(pool: Pool) {
    let (app, mailer) = app(pool);

    let url = request_link(&app, &mailer, "founder@example.com").await;
    assert!(mailer.last().unwrap().is_new, "this address is new");

    let (status, cookie) = follow(&app, &url).await;
    assert_eq!(
        status,
        StatusCode::SEE_OTHER,
        "the link sends you to the app"
    );

    let cookie = cookie_value(&cookie.expect("a session cookie"));
    let (status, body) = me(&app, Some(&cookie)).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["user"]["email"], "founder@example.com");
    assert_eq!(
        body["team"]["name"], "founder@example.com",
        "a personal team exists from the first moment"
    );
}

#[sqlx::test(migrations = "../db/migrations")]
async fn signing_in_again_reuses_the_same_account(pool: Pool) {
    let (app, mailer) = app(pool);

    let first = request_link(&app, &mailer, "founder@example.com").await;
    let (_, cookie) = follow(&app, &first).await;
    let (_, body) = me(&app, Some(&cookie_value(&cookie.unwrap()))).await;
    let user_id = body["user"]["id"].clone();

    let second = request_link(&app, &mailer, "founder@example.com").await;
    assert!(!mailer.last().unwrap().is_new, "this address is known now");
    let (_, cookie) = follow(&app, &second).await;
    let (_, body) = me(&app, Some(&cookie_value(&cookie.unwrap()))).await;

    assert_eq!(body["user"]["id"], user_id, "one address, one account");
}

#[sqlx::test(migrations = "../db/migrations")]
async fn a_link_works_exactly_once(pool: Pool) {
    let (app, mailer) = app(pool);
    let url = request_link(&app, &mailer, "founder@example.com").await;

    let (first, _) = follow(&app, &url).await;
    assert_eq!(first, StatusCode::SEE_OTHER);

    let (second, cookie) = follow(&app, &url).await;
    assert_eq!(
        second,
        StatusCode::UNAUTHORIZED,
        "a used link must not sign anyone in again"
    );
    assert!(cookie.is_none(), "and must not hand out a session");
}

#[sqlx::test(migrations = "../db/migrations")]
async fn a_made_up_link_signs_nobody_in(pool: Pool) {
    let (app, _) = app(pool);

    let (status, cookie) = follow(
        &app,
        "http://localhost:5173/auth/callback?token=not-a-real-token",
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(cookie.is_none());
}

#[sqlx::test(migrations = "../db/migrations")]
async fn an_expired_link_signs_nobody_in(pool: Pool) {
    let (app, mailer) = app(pool.clone());
    let url = request_link(&app, &mailer, "founder@example.com").await;

    // Reach past the clock rather than waiting fifteen minutes.
    sqlx::query("update login_tokens set expires_at = now() - interval '1 minute'")
        .execute(&pool)
        .await
        .expect("age the link");

    let (status, _) = follow(&app, &url).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "../db/migrations")]
async fn the_link_is_never_stored_where_it_could_be_used(pool: Pool) {
    let (app, mailer) = app(pool.clone());
    let url = request_link(&app, &mailer, "founder@example.com").await;
    let secret = url.split_once("token=").expect("a token").1;

    let rows: Vec<(String, Vec<u8>)> = sqlx::query_as("select email, token_hash from login_tokens")
        .fetch_all(&pool)
        .await
        .expect("read tokens");

    assert_eq!(rows.len(), 1);
    let (_, stored) = &rows[0];
    assert_eq!(stored.len(), 32, "a SHA-256, not the token");
    assert_ne!(
        stored.as_slice(),
        secret.as_bytes(),
        "the database must not hold anything that can sign someone in"
    );
}

#[sqlx::test(migrations = "../db/migrations")]
async fn the_answer_is_the_same_whether_or_not_the_account_exists(pool: Pool) {
    let (app, mailer) = app(pool);

    let known = post_json(
        &app,
        "/auth/request-link",
        json!({ "email": "founder@example.com" }),
    )
    .await;
    let url = mailer.last().unwrap().url;
    follow(&app, &url).await;

    let again = post_json(
        &app,
        "/auth/request-link",
        json!({ "email": "founder@example.com" }),
    )
    .await;
    let stranger = post_json(
        &app,
        "/auth/request-link",
        json!({ "email": "nobody@example.com" }),
    )
    .await;

    assert_eq!(known, StatusCode::ACCEPTED);
    assert_eq!(
        again, stranger,
        "the response must not reveal who has an account"
    );
}

#[sqlx::test(migrations = "../db/migrations")]
async fn one_address_cannot_be_used_to_flood_an_inbox(pool: Pool) {
    let (app, mailer) = app(pool);

    for _ in 0..8 {
        let status = post_json(
            &app,
            "/auth/request-link",
            json!({ "email": "founder@example.com" }),
        )
        .await;
        assert_eq!(status, StatusCode::ACCEPTED, "and it still looks fine");
    }

    assert_eq!(
        mailer.sent().len(),
        5,
        "sending stops at the limit, quietly"
    );
}

#[sqlx::test(migrations = "../db/migrations")]
async fn something_that_is_not_an_address_is_refused(pool: Pool) {
    let (app, mailer) = app(pool);

    let status = post_json(&app, "/auth/request-link", json!({ "email": "founder" })).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(mailer.sent().is_empty());
}

#[sqlx::test(migrations = "../db/migrations")]
async fn without_a_cookie_nobody_is_signed_in(pool: Pool) {
    let (app, _) = app(pool);

    let (status, _) = me(&app, None).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "../db/migrations")]
async fn a_made_up_cookie_signs_nobody_in(pool: Pool) {
    let (app, _) = app(pool);

    let (status, _) = me(&app, Some("not-a-real-session")).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "../db/migrations")]
async fn signing_out_ends_the_session_everywhere_it_was_stored(pool: Pool) {
    let (app, mailer) = app(pool.clone());
    let url = request_link(&app, &mailer, "founder@example.com").await;
    let (_, cookie) = follow(&app, &url).await;
    let cookie = cookie_value(&cookie.unwrap());

    let (status, _) = me(&app, Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);

    let signed_out = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/sign-out")
                .header(header::COOKIE, format!("{COOKIE_NAME}={cookie}"))
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("call");
    assert_eq!(signed_out.status(), StatusCode::NO_CONTENT);

    // Holding the old cookie is not enough: the row is gone.
    let (status, _) = me(&app, Some(&cookie)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let left: i64 = sqlx::query_scalar("select count(*) from sessions")
        .fetch_one(&pool)
        .await
        .expect("count sessions");
    assert_eq!(left, 0);
}

#[sqlx::test(migrations = "../db/migrations")]
async fn an_expired_session_signs_nobody_in(pool: Pool) {
    let (app, mailer) = app(pool.clone());
    let url = request_link(&app, &mailer, "founder@example.com").await;
    let (_, cookie) = follow(&app, &url).await;
    let cookie = cookie_value(&cookie.unwrap());

    sqlx::query("update sessions set expires_at = now() - interval '1 day'")
        .execute(&pool)
        .await
        .expect("age the session");

    let (status, _) = me(&app, Some(&cookie)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "../db/migrations")]
async fn the_session_cookie_is_locked_down(pool: Pool) {
    let (app, mailer) = app(pool);
    let url = request_link(&app, &mailer, "founder@example.com").await;

    let (_, cookie) = follow(&app, &url).await;
    let cookie = cookie.expect("a session cookie");

    assert!(
        cookie.contains("HttpOnly"),
        "script must never read it: {cookie}"
    );
    assert!(
        cookie.contains("SameSite=Lax"),
        "Strict would drop the cookie when arriving from an email: {cookie}"
    );
    assert!(cookie.contains("Path=/"), "{cookie}");
}
