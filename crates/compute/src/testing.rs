//! The contract every compute provider must satisfy.
//!
//! One suite, run unmodified against every driver. A new provider — Fly, our
//! own Firecracker hosts, whatever replaces them — is finished when this is
//! green, which is a more useful definition than "it compiles".
//!
//! Use it with [`crate::driver_suite`]:
//!
//! ```ignore
//! croncave_compute::driver_suite!(async { FakeDriver::new() });
//! ```
//!
//! Each case becomes its own `#[tokio::test]`, so a failure names the promise
//! that broke rather than just "the suite".

// This module is test support: every `expect` here is an assertion, and the
// panic it produces is the failure report. It is deliberately not behind
// `#[cfg(test)]`, so that a driver in another crate can be held to the same
// contract.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use uuid::Uuid;

use crate::{ComputeDriver, ComputeId, Error, Spec, State};

/// An image that starts and keeps running, and is small enough that pulling
/// it does not dominate a test run.
pub const TEST_IMAGE: &str = "alpine:3.21";

/// A spec for a workspace nothing else is using.
#[must_use]
pub fn spec() -> Spec {
    Spec::new(Uuid::now_v7(), TEST_IMAGE)
}

/// A new computer exists, and is not running.
///
/// Workspaces sleep by default: creating one must not start it, or every
/// signup would begin costing money immediately.
pub async fn a_new_computer_is_stopped(driver: &dyn ComputeDriver) {
    let id = driver.create(&spec()).await.expect("create");

    let state = driver.status(&id).await.expect("status");
    assert_eq!(state, State::Stopped, "a new computer must not be running");

    driver.destroy(&id).await.expect("destroy");
}

/// Starting it makes it run; stopping it stops it.
pub async fn it_starts_and_stops(driver: &dyn ComputeDriver) {
    let id = driver.create(&spec()).await.expect("create");

    driver.start(&id).await.expect("start");
    assert_eq!(driver.status(&id).await.expect("status"), State::Running);

    driver.stop(&id).await.expect("stop");
    assert_eq!(driver.status(&id).await.expect("status"), State::Stopped);

    driver.destroy(&id).await.expect("destroy");
}

/// Starting something already running succeeds.
///
/// An orchestrator whose connection drops mid-call has to be able to simply
/// try again. If this were an error, every caller would first have to work
/// out how far the last attempt got.
pub async fn starting_twice_is_fine(driver: &dyn ComputeDriver) {
    let id = driver.create(&spec()).await.expect("create");

    driver.start(&id).await.expect("start");
    driver
        .start(&id)
        .await
        .expect("starting something already running must succeed");

    assert_eq!(driver.status(&id).await.expect("status"), State::Running);

    driver.destroy(&id).await.expect("destroy");
}

/// Stopping something already stopped succeeds, for the same reason.
pub async fn stopping_twice_is_fine(driver: &dyn ComputeDriver) {
    let id = driver.create(&spec()).await.expect("create");

    driver.stop(&id).await.expect("stopping a stopped computer");
    driver.stop(&id).await.expect("and again");

    assert_eq!(driver.status(&id).await.expect("status"), State::Stopped);

    driver.destroy(&id).await.expect("destroy");
}

/// Destroying it removes it, and destroying it again succeeds.
pub async fn destroying_is_final_and_repeatable(driver: &dyn ComputeDriver) {
    let id = driver.create(&spec()).await.expect("create");
    driver.start(&id).await.expect("start");

    // Destroying a running computer must work: that is what deleting a
    // workspace does.
    driver.destroy(&id).await.expect("destroy while running");
    driver
        .destroy(&id)
        .await
        .expect("destroying something already gone must succeed");

    match driver.status(&id).await {
        Err(Error::NotFound(_)) | Ok(State::Gone) => {}
        other => panic!("a destroyed computer must be gone, got {other:?}"),
    }
}

/// An identifier the provider never issued is reported as not found.
///
/// Not a panic, and not a silent success: the orchestrator needs to tell "it
/// is gone, make another" apart from "the provider is having trouble", and
/// only this distinction lets it.
pub async fn an_unknown_computer_is_not_found(driver: &dyn ComputeDriver) {
    let id = ComputeId::new("croncave-test-does-not-exist-0000");

    for result in [
        driver.start(&id).await,
        driver.stop(&id).await,
        driver.status(&id).await.map(|_| ()),
    ] {
        match result {
            Err(Error::NotFound(_)) => {}
            other => panic!("an unknown id must be NotFound, got {other:?}"),
        }
    }
}

/// Two workspaces get two computers, and operating on one leaves the other
/// alone.
pub async fn computers_are_independent(driver: &dyn ComputeDriver) {
    let first = driver.create(&spec()).await.expect("create first");
    let second = driver.create(&spec()).await.expect("create second");

    assert_ne!(
        first.as_str(),
        second.as_str(),
        "each workspace gets its own computer"
    );

    driver.start(&first).await.expect("start first");

    assert_eq!(driver.status(&first).await.expect("status"), State::Running);
    assert_eq!(
        driver.status(&second).await.expect("status"),
        State::Stopped,
        "starting one workspace must not start another"
    );

    driver.destroy(&first).await.expect("destroy first");
    driver.destroy(&second).await.expect("destroy second");
}

/// Generate the suite as individual tests for one driver.
///
/// The argument is an expression producing a driver, awaited once per test so
/// cases cannot leak state into each other.
#[macro_export]
macro_rules! driver_suite {
    ($make:expr) => {
        $crate::driver_suite!(
            $make,
            a_new_computer_is_stopped,
            it_starts_and_stops,
            starting_twice_is_fine,
            stopping_twice_is_fine,
            destroying_is_final_and_repeatable,
            an_unknown_computer_is_not_found,
            computers_are_independent
        );
    };
    ($make:expr, $($case:ident),+ $(,)?) => {
        $(
            #[tokio::test]
            async fn $case() {
                let driver = $make.await;
                $crate::testing::$case(&driver).await;
            }
        )+
    };
}
