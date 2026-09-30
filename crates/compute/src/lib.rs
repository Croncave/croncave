//! Where a workspace's computer comes from.
//!
//! This crate is the **only** place in Croncave that knows a provider exists.
//! Everything else asks for a computer and is told what state it is in;
//! whether that is a Fly machine, a Docker container on a laptop, or one of
//! our own Firecracker hosts later, is settled here and nowhere else. See
//! `AGENTS.md`, "Provider-neutral outside the driver."
//!
//! Nothing in the types below uses a provider's vocabulary. There is no
//! "machine" or "container", and no region string that means something to one
//! vendor and nothing to another.
//!
//! Every driver must pass [`testing`]'s suite, unmodified. A new provider is
//! finished when that suite is green, which is a far more useful definition
//! of finished than "the code compiles".

pub mod fake;
pub mod local;
pub mod testing;

use std::fmt;

use uuid::Uuid;

/// The provider's own name for one workspace's computer.
///
/// Opaque on purpose: the control plane stores it and hands it back, and only
/// the driver that issued it knows what it means.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ComputeId(String);

impl ComputeId {
    /// Wrap a provider's identifier.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// The identifier as the provider wrote it.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ComputeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Debug for ComputeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ComputeId({})", self.0)
    }
}

/// What to build. Sizes are plain numbers because every provider expresses
/// them differently, and translating is the driver's job.
///
/// Its [`fmt::Debug`] lists the names of the environment variables but none
/// of their values: one of them is the workspace's identity.
#[derive(Clone)]
pub struct Spec {
    /// The workspace this belongs to. Drivers label the resource with it, so
    /// a stray one can always be traced back to its owner.
    pub workspace_id: Uuid,
    /// The OCI image to run. Standard images only, so a workspace is not tied
    /// to any provider.
    pub image: String,
    /// How many virtual CPUs.
    pub cpus: u8,
    /// How much memory.
    pub memory_mb: u32,
    /// What the workspace starts with in its environment: where to find the
    /// relay, and the one-time token that proves which workspace it is.
    ///
    /// These are secrets. A driver puts them in the workspace and never logs
    /// them.
    pub env: Vec<(String, String)>,
    /// What to run instead of the image's own entrypoint.
    ///
    /// A workspace image starts its agent by itself and needs none. Tests
    /// use a stock image that would otherwise exit at once, so they say what
    /// should keep it up.
    pub command: Option<Vec<String>>,
}

impl fmt::Debug for Spec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Spec")
            .field("workspace_id", &self.workspace_id)
            .field("image", &self.image)
            .field("cpus", &self.cpus)
            .field("memory_mb", &self.memory_mb)
            .field(
                "env",
                &self.env.iter().map(|(key, _)| key).collect::<Vec<_>>(),
            )
            .finish()
    }
}

impl Spec {
    /// A small workspace, which is all step 2 needs.
    #[must_use]
    pub fn new(workspace_id: Uuid, image: impl Into<String>) -> Self {
        Self {
            workspace_id,
            image: image.into(),
            cpus: 1,
            memory_mb: 512,
            env: Vec::new(),
            command: None,
        }
    }

    /// Start the workspace with this in its environment.
    #[must_use]
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.push((key.into(), value.into()));
        self
    }

    /// Run this instead of the image's entrypoint.
    #[must_use]
    pub fn with_command(mut self, command: Vec<String>) -> Self {
        self.command = Some(command);
        self
    }
}

/// Where a workspace's computer is in its life.
///
/// These are the states `docs/architecture.md` describes, with one addition:
/// [`State::Gone`] for a computer the provider no longer has, which a driver
/// must be able to report rather than pretend about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// It exists and its disk is kept, but nothing is running.
    Stopped,
    /// It was asked to start and has not finished.
    Starting,
    /// It is running.
    Running,
    /// It was asked to stop and has not finished.
    Stopping,
    /// The provider no longer has it. Something outside Croncave removed it,
    /// or it was destroyed.
    Gone,
}

impl State {
    /// The word the product uses for this state. `docs/design/system` requires
    /// a colour to be paired with a word, and these are the words.
    #[must_use]
    pub fn word(self) -> &'static str {
        match self {
            Self::Stopped => "asleep",
            Self::Starting => "waking",
            Self::Running => "awake",
            Self::Stopping => "stopping",
            Self::Gone => "gone",
        }
    }
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.word())
    }
}

/// What can go wrong asking a provider for something.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The provider has no such computer. Distinct from every other failure,
    /// because it is the one a caller can reasonably act on: make a new one.
    #[error("the provider has no computer with id {0}")]
    NotFound(ComputeId),

    /// The provider could not be reached at all. The computer may be fine;
    /// we simply do not know, so a caller must not conclude anything about it.
    #[error("could not reach the compute provider")]
    Unreachable(#[source] Box<dyn std::error::Error + Send + Sync>),

    /// The provider answered, and said no.
    #[error("the compute provider refused: {message}")]
    Refused {
        /// What it said.
        message: String,
        /// Its own error, if there was one.
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}

impl Error {
    /// A refusal with no underlying error.
    #[must_use]
    pub fn refused(message: impl Into<String>) -> Self {
        Self::Refused {
            message: message.into(),
            source: None,
        }
    }
}

/// Somewhere a workspace's computer can live.
///
/// **Every operation is idempotent.** Starting something already running
/// succeeds; stopping something already stopped succeeds; destroying
/// something already gone succeeds. An orchestrator that loses its connection
/// mid-call has to be able to simply try again, without first working out how
/// far the last attempt got.
#[async_trait::async_trait]
pub trait ComputeDriver: Send + Sync + fmt::Debug {
    /// Which driver this is, for logs and for the details layer.
    fn name(&self) -> &'static str;

    /// Make a computer for a workspace, stopped. Does not start it: a
    /// workspace sleeps until there is work.
    ///
    /// # Errors
    ///
    /// Returns an error if the provider refuses or cannot be reached.
    async fn create(&self, spec: &Spec) -> Result<ComputeId, Error>;

    /// Start it, or do nothing if it is already running.
    ///
    /// # Errors
    ///
    /// [`Error::NotFound`] if the provider has no such computer.
    async fn start(&self, id: &ComputeId) -> Result<(), Error>;

    /// Stop it, keeping its disk, or do nothing if it is already stopped.
    ///
    /// # Errors
    ///
    /// [`Error::NotFound`] if the provider has no such computer.
    async fn stop(&self, id: &ComputeId) -> Result<(), Error>;

    /// What state it is in, according to the provider rather than to us.
    ///
    /// # Errors
    ///
    /// [`Error::NotFound`] if the provider has no such computer. A computer
    /// that existed and was removed elsewhere reports [`State::Gone`] if the
    /// provider still remembers it, and `NotFound` once it does not.
    async fn status(&self, id: &ComputeId) -> Result<State, Error>;

    /// Remove it and its disk. Succeeds if it is already gone.
    ///
    /// # Errors
    ///
    /// Returns an error if the provider refuses or cannot be reached.
    async fn destroy(&self, id: &ComputeId) -> Result<(), Error>;
}
