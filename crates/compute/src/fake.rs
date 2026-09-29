//! A provider that exists only in memory.
//!
//! For tests that care about what the orchestrator does, not about whether a
//! container really started. It holds itself to exactly the same contract as
//! a real driver, and proves it by passing the same suite.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::{ComputeDriver, ComputeId, Error, Spec, State};

/// An in-memory compute provider.
#[derive(Clone, Debug, Default)]
pub struct FakeDriver {
    computers: Arc<Mutex<HashMap<String, State>>>,
    next: Arc<Mutex<u64>>,
    /// When set, every operation fails this way. For testing what callers do
    /// when a provider is having a bad day.
    failing: Arc<Mutex<bool>>,
}

impl FakeDriver {
    /// A provider with nothing in it.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Make every later call fail as though the provider were unreachable.
    pub fn break_it(&self) {
        *self.locked_flag() = true;
    }

    /// Remove everything, as though something outside Croncave had.
    pub fn destroy_all(&self) {
        self.locked().clear();
    }

    /// How many computers it is holding, including stopped ones.
    #[must_use]
    pub fn count(&self) -> usize {
        self.locked().len()
    }

    fn locked(&self) -> std::sync::MutexGuard<'_, HashMap<String, State>> {
        self.computers
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn locked_flag(&self) -> std::sync::MutexGuard<'_, bool> {
        self.failing
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn check_up(&self) -> Result<(), Error> {
        if *self.locked_flag() {
            return Err(Error::Unreachable(
                "the fake provider is switched off".into(),
            ));
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl ComputeDriver for FakeDriver {
    fn name(&self) -> &'static str {
        "fake"
    }

    async fn create(&self, spec: &Spec) -> Result<ComputeId, Error> {
        self.check_up()?;

        let mut next = self
            .next
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *next += 1;
        let id = format!("fake-{}-{}", spec.workspace_id.simple(), *next);
        drop(next);

        self.locked().insert(id.clone(), State::Stopped);

        Ok(ComputeId::new(id))
    }

    async fn start(&self, id: &ComputeId) -> Result<(), Error> {
        self.check_up()?;

        match self.locked().get_mut(id.as_str()) {
            // Already running is success, not an error.
            Some(state) => {
                *state = State::Running;
                Ok(())
            }
            None => Err(Error::NotFound(id.clone())),
        }
    }

    async fn stop(&self, id: &ComputeId) -> Result<(), Error> {
        self.check_up()?;

        match self.locked().get_mut(id.as_str()) {
            Some(state) => {
                *state = State::Stopped;
                Ok(())
            }
            None => Err(Error::NotFound(id.clone())),
        }
    }

    async fn status(&self, id: &ComputeId) -> Result<State, Error> {
        self.check_up()?;

        self.locked()
            .get(id.as_str())
            .copied()
            .ok_or_else(|| Error::NotFound(id.clone()))
    }

    async fn destroy(&self, id: &ComputeId) -> Result<(), Error> {
        self.check_up()?;

        // Destroying something already gone is success.
        self.locked().remove(id.as_str());

        Ok(())
    }
}
