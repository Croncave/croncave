//! Apply the migrations to the database in `DATABASE_URL`.
//!
//! The control plane does this at startup; this is the same thing by hand,
//! for a developer who wants their local database up to date on its own.
//!
//! ```text
//! docker compose up -d
//! cargo run -p croncave-db --example migrate
//! ```

use croncave_telemetry::Config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let _telemetry =
        croncave_telemetry::init(Config::from_env("db-migrate", env!("CARGO_PKG_VERSION"))?)?;

    let database_url =
        std::env::var("DATABASE_URL").map_err(|_| "DATABASE_URL is not set. See .env.example.")?;

    let pool = croncave_db::connect(&database_url, 1).await?;
    croncave_db::migrate(&pool).await?;

    Ok(())
}
