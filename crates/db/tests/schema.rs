//! What the schema guarantees, tested against a real Postgres.
//!
//! `#[sqlx::test]` creates a fresh database per test and applies the
//! migrations, so every test starts from the schema a new deployment gets.
//! These need a database: `docker compose up -d`, then `DATABASE_URL` set.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use croncave_db::Pool;
use uuid::Uuid;

/// A user, their personal team, and the membership joining the two — what
/// signing up creates.
async fn sign_up(pool: &Pool, email: &str) -> (Uuid, Uuid) {
    let user_id = Uuid::now_v7();
    let team_id = Uuid::now_v7();

    sqlx::query("insert into users (id, email) values ($1, $2)")
        .bind(user_id)
        .bind(email)
        .execute(pool)
        .await
        .expect("insert user");

    sqlx::query("insert into teams (id, name, kind) values ($1, $2, 'personal')")
        .bind(team_id)
        .bind(email)
        .execute(pool)
        .await
        .expect("insert team");

    sqlx::query(
        "insert into memberships (id, user_id, team_id, role) values ($1, $2, $3, 'owner')",
    )
    .bind(Uuid::now_v7())
    .bind(user_id)
    .bind(team_id)
    .execute(pool)
    .await
    .expect("insert membership");

    (user_id, team_id)
}

async fn create_workspace(pool: &Pool, team_id: Uuid, created_by: Uuid, name: &str) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query("insert into workspaces (id, team_id, name, created_by) values ($1, $2, $3, $4)")
        .bind(id)
        .bind(team_id)
        .bind(name)
        .bind(created_by)
        .execute(pool)
        .await
        .expect("insert workspace");
    id
}

#[sqlx::test(migrations = "./migrations")]
async fn signing_up_twice_with_one_email_is_refused(pool: Pool) {
    sign_up(&pool, "founder@example.com").await;

    let again = sqlx::query("insert into users (id, email) values ($1, $2)")
        .bind(Uuid::now_v7())
        .bind("founder@example.com")
        .execute(&pool)
        .await;

    assert!(again.is_err(), "one email, one account");
}

#[sqlx::test(migrations = "./migrations")]
async fn a_new_workspace_is_asleep(pool: Pool) {
    let (user_id, team_id) = sign_up(&pool, "founder@example.com").await;
    let id = create_workspace(&pool, team_id, user_id, "Stock Watcher").await;

    let state: String = sqlx::query_scalar("select state from workspaces where id = $1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .expect("read state");

    // It has no compute until step 2, so asleep is the only honest state.
    assert_eq!(state, "asleep");
}

#[sqlx::test(migrations = "./migrations")]
async fn a_workspace_state_outside_the_lifecycle_is_refused(pool: Pool) {
    let (user_id, team_id) = sign_up(&pool, "founder@example.com").await;
    let id = create_workspace(&pool, team_id, user_id, "Stock Watcher").await;

    let bad = sqlx::query("update workspaces set state = 'exploded' where id = $1")
        .bind(id)
        .execute(&pool)
        .await;

    assert!(bad.is_err(), "only the lifecycle's states are allowed");
}

#[sqlx::test(migrations = "./migrations")]
async fn a_team_kind_outside_the_two_is_refused(pool: Pool) {
    let bad = sqlx::query("insert into teams (id, name, kind) values ($1, 'Acme', 'enterprise')")
        .bind(Uuid::now_v7())
        .execute(&pool)
        .await;

    assert!(bad.is_err(), "a team is personal or shared");
}

#[sqlx::test(migrations = "./migrations")]
async fn a_person_joins_a_team_once(pool: Pool) {
    let (user_id, team_id) = sign_up(&pool, "founder@example.com").await;

    let again = sqlx::query(
        "insert into memberships (id, user_id, team_id, role) values ($1, $2, $3, 'member')",
    )
    .bind(Uuid::now_v7())
    .bind(user_id)
    .bind(team_id)
    .execute(&pool)
    .await;

    assert!(again.is_err(), "one membership per person per team");
}

#[sqlx::test(migrations = "./migrations")]
async fn deleting_a_team_takes_its_workspaces_with_it(pool: Pool) {
    let (user_id, team_id) = sign_up(&pool, "founder@example.com").await;
    create_workspace(&pool, team_id, user_id, "Stock Watcher").await;

    sqlx::query("delete from teams where id = $1")
        .bind(team_id)
        .execute(&pool)
        .await
        .expect("delete team");

    let left: i64 = sqlx::query_scalar("select count(*) from workspaces where team_id = $1")
        .bind(team_id)
        .fetch_one(&pool)
        .await
        .expect("count workspaces");

    assert_eq!(left, 0, "a team owns its workspaces");
}

#[sqlx::test(migrations = "./migrations")]
async fn a_user_who_made_something_cannot_simply_be_deleted(pool: Pool) {
    let (user_id, team_id) = sign_up(&pool, "founder@example.com").await;
    create_workspace(&pool, team_id, user_id, "Stock Watcher").await;

    let deleted = sqlx::query("delete from users where id = $1")
        .bind(user_id)
        .execute(&pool)
        .await;

    // Attribution outlives the account: deleting a person has to deal with
    // what they made, rather than silently dropping who made it.
    assert!(deleted.is_err(), "created_by must survive");
}

#[sqlx::test(migrations = "./migrations")]
async fn signing_out_everywhere_is_deleting_the_users_sessions(pool: Pool) {
    let (user_id, _) = sign_up(&pool, "founder@example.com").await;

    sqlx::query(
        "insert into sessions (id, user_id, token_hash, expires_at)
         values ($1, $2, $3, now() + interval '30 days')",
    )
    .bind(Uuid::now_v7())
    .bind(user_id)
    .bind(vec![1_u8; 32])
    .execute(&pool)
    .await
    .expect("insert session");

    sqlx::query("delete from users where id = $1")
        .bind(user_id)
        .execute(&pool)
        .await
        .expect("delete user");

    let left: i64 = sqlx::query_scalar("select count(*) from sessions where user_id = $1")
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .expect("count sessions");

    assert_eq!(left, 0, "sessions go with the user");
}

#[sqlx::test(migrations = "./migrations")]
async fn one_sign_in_link_cannot_be_stored_twice(pool: Pool) {
    let hash = vec![7_u8; 32];

    for _ in 0..2 {
        let inserted = sqlx::query(
            "insert into login_tokens (id, email, token_hash, expires_at)
             values ($1, 'founder@example.com', $2, now() + interval '15 minutes')",
        )
        .bind(Uuid::now_v7())
        .bind(&hash)
        .execute(&pool)
        .await;

        if inserted.is_err() {
            return;
        }
    }

    panic!("the same token hash must not exist twice");
}
