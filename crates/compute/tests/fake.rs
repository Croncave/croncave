//! The fake provider, held to the same contract as a real one.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use croncave_compute::fake::FakeDriver;

croncave_compute::driver_suite!(async { FakeDriver::new() });
