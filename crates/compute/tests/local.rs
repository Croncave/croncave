//! The Docker driver, held to exactly the same contract as the fake.
//!
//! These need a Docker daemon. They are skipped when there is none, so the
//! checks stay runnable without one; CI always has Docker, and the check
//! script makes a skip there a failure.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use croncave_compute::local::LocalDriver;

/// A connected driver, or `None` when there is no daemon to talk to.
async fn driver() -> Option<LocalDriver> {
    LocalDriver::connect().await.ok()
}

/// Run one case, or skip it loudly.
macro_rules! with_docker {
    ($case:ident) => {
        #[tokio::test]
        async fn $case() {
            let Some(driver) = driver().await else {
                eprintln!("skipped {}: no Docker daemon", stringify!($case));
                return;
            };
            croncave_compute::testing::$case(&driver).await;
        }
    };
}

with_docker!(a_new_computer_is_stopped);
with_docker!(it_starts_and_stops);
with_docker!(starting_twice_is_fine);
with_docker!(stopping_twice_is_fine);
with_docker!(destroying_is_final_and_repeatable);
with_docker!(an_unknown_computer_is_not_found);
with_docker!(computers_are_independent);
