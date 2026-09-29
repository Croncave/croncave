//! A workspace as a Docker container on the machine you are sitting at.
//!
//! Containers are not virtual machines and this driver is not what runs in
//! production. From the agent's side, though, a container behaves the same:
//! it has a filesystem that survives a stop, it can be started and stopped,
//! and something inside it dials out. That is enough to build and test
//! everything above the driver without an account anywhere.
//!
//! It only ever touches containers it made. Every one is labelled, and every
//! lookup is by the id this driver issued, so a stray `docker run` on the
//! same machine is invisible to it.

use bollard::Docker;
use bollard::models::{ContainerCreateBody, ContainerStateStatusEnum, HostConfig};
use bollard::query_parameters::{
    CreateContainerOptionsBuilder, CreateImageOptionsBuilder, InspectContainerOptions,
    RemoveContainerOptionsBuilder, StartContainerOptions, StopContainerOptionsBuilder,
};
use futures_util::StreamExt as _;
use std::collections::HashMap;

use crate::{ComputeDriver, ComputeId, Error, Spec, State};

/// The label carrying the workspace a container belongs to, so a stray one
/// can be traced back to its owner.
pub const WORKSPACE_LABEL: &str = "com.croncave.workspace-id";

/// The label marking a container as ours at all.
pub const MANAGED_LABEL: &str = "com.croncave.managed";

/// How long to let a container shut down before it is killed.
const STOP_GRACE_SECONDS: i32 = 10;

/// How long to wait on the Docker daemon. Pulling an image can be slow.
const DOCKER_TIMEOUT_SECONDS: u64 = 120;

/// Docker on this machine.
#[derive(Clone)]
pub struct LocalDriver {
    docker: Docker,
}

impl std::fmt::Debug for LocalDriver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalDriver").finish_non_exhaustive()
    }
}

impl LocalDriver {
    /// Connect to the Docker daemon, and prove it answers.
    ///
    /// `DOCKER_HOST` wins if it is set. Otherwise the usual socket is tried,
    /// and then the per-user one Docker Desktop creates: on macOS there is no
    /// `/var/run/docker.sock` at all, so a driver that only looked there
    /// would report "no Docker" on a machine where Docker is plainly running.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Unreachable`] if nothing answers.
    pub async fn connect() -> Result<Self, Error> {
        let mut last: Option<bollard::errors::Error> = None;

        for candidate in Self::candidates() {
            match candidate {
                Ok(docker) => match docker.ping().await {
                    Ok(_) => return Ok(Self { docker }),
                    Err(error) => last = Some(error),
                },
                Err(error) => last = Some(error),
            }
        }

        Err(Error::Unreachable(match last {
            Some(error) => Box::new(error),
            None => "no Docker daemon was found".into(),
        }))
    }

    /// Where a daemon might be, in the order worth trying.
    fn candidates() -> Vec<Result<Docker, bollard::errors::Error>> {
        let mut candidates = vec![Docker::connect_with_defaults()];

        if std::env::var_os("DOCKER_HOST").is_none() {
            if let Some(home) = std::env::var_os("HOME") {
                let desktop = std::path::Path::new(&home).join(".docker/run/docker.sock");
                if desktop.exists() {
                    candidates.push(Docker::connect_with_socket(
                        &desktop.to_string_lossy(),
                        DOCKER_TIMEOUT_SECONDS,
                        bollard::API_DEFAULT_VERSION,
                    ));
                }
            }
        }

        candidates
    }

    /// Whether the daemon still answers.
    pub async fn is_available(&self) -> bool {
        self.docker.ping().await.is_ok()
    }

    /// Turn a Docker error into ours, keeping "no such container" distinct
    /// from everything else: it is the one a caller can act on.
    fn translate(id: &ComputeId, error: bollard::errors::Error) -> Error {
        match error {
            bollard::errors::Error::DockerResponseServerError {
                status_code: 404, ..
            } => Error::NotFound(id.clone()),
            other => Error::Refused {
                message: "Docker refused the request".to_owned(),
                source: Some(Box::new(other)),
            },
        }
    }

    /// Make sure the image is on this machine. Pulling is slow and only
    /// needed once, so it is skipped when the image is already here.
    async fn ensure_image(&self, image: &str) -> Result<(), Error> {
        if self.docker.inspect_image(image).await.is_ok() {
            return Ok(());
        }

        tracing::info!(image, "pulling the workspace image");

        let options = CreateImageOptionsBuilder::default()
            .from_image(image)
            .build();
        let mut pull = self.docker.create_image(Some(options), None, None);

        while let Some(step) = pull.next().await {
            step.map_err(|error| Error::Refused {
                message: format!("could not pull {image}"),
                source: Some(Box::new(error)),
            })?;
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl ComputeDriver for LocalDriver {
    fn name(&self) -> &'static str {
        "local"
    }

    async fn create(&self, spec: &Spec) -> Result<ComputeId, Error> {
        self.ensure_image(&spec.image).await?;

        let name = format!("croncave-{}", spec.workspace_id.simple());

        let labels = HashMap::from([
            (WORKSPACE_LABEL.to_owned(), spec.workspace_id.to_string()),
            (MANAGED_LABEL.to_owned(), "true".to_owned()),
        ]);

        let host = HostConfig {
            // Sizes are advisory on a developer machine, but honouring them
            // keeps the local driver behaving like the real one.
            nano_cpus: Some(i64::from(spec.cpus) * 1_000_000_000),
            memory: Some(i64::from(spec.memory_mb) * 1024 * 1024),
            ..Default::default()
        };

        let config = ContainerCreateBody {
            image: Some(spec.image.clone()),
            // Nothing runs in a workspace yet; it only has to stay up so it
            // can be started and stopped. The agent replaces this in step 3.
            //
            // It traps SIGTERM and exits, rather than plain `sleep infinity`,
            // which ignores signals: Docker would then wait out the whole
            // grace period and kill it, making every stop take ten seconds.
            // Whatever runs here in future must shut down on SIGTERM too.
            cmd: Some(vec![
                "/bin/sh".to_owned(),
                "-c".to_owned(),
                "trap 'exit 0' TERM INT; while :; do sleep 1; done".to_owned(),
            ]),
            labels: Some(labels),
            host_config: Some(host),
            ..Default::default()
        };

        let options = CreateContainerOptionsBuilder::default().name(&name).build();

        let created = self
            .docker
            .create_container(Some(options), config)
            .await
            .map_err(|error| Error::Refused {
                message: "could not create the container".to_owned(),
                source: Some(Box::new(error)),
            })?;

        tracing::info!(workspace_id = %spec.workspace_id, container = %created.id, "created a container");

        // Created but not started: a workspace sleeps until there is work.
        Ok(ComputeId::new(created.id))
    }

    async fn start(&self, id: &ComputeId) -> Result<(), Error> {
        match self
            .docker
            .start_container(id.as_str(), None::<StartContainerOptions>)
            .await
        {
            Ok(()) => Ok(()),
            // 304 Not Modified: already running, which is success.
            Err(bollard::errors::Error::DockerResponseServerError {
                status_code: 304, ..
            }) => Ok(()),
            Err(error) => Err(Self::translate(id, error)),
        }
    }

    async fn stop(&self, id: &ComputeId) -> Result<(), Error> {
        let options = StopContainerOptionsBuilder::default()
            .t(STOP_GRACE_SECONDS)
            .build();

        match self.docker.stop_container(id.as_str(), Some(options)).await {
            Ok(()) => Ok(()),
            // 304 Not Modified: already stopped, which is success.
            Err(bollard::errors::Error::DockerResponseServerError {
                status_code: 304, ..
            }) => Ok(()),
            Err(error) => Err(Self::translate(id, error)),
        }
    }

    async fn status(&self, id: &ComputeId) -> Result<State, Error> {
        let container = self
            .docker
            .inspect_container(id.as_str(), None::<InspectContainerOptions>)
            .await
            .map_err(|error| Self::translate(id, error))?;

        let status = container
            .state
            .and_then(|state| state.status)
            .unwrap_or(ContainerStateStatusEnum::EMPTY);

        Ok(match status {
            ContainerStateStatusEnum::RUNNING => State::Running,
            ContainerStateStatusEnum::RESTARTING => State::Starting,
            ContainerStateStatusEnum::REMOVING => State::Stopping,
            ContainerStateStatusEnum::DEAD => State::Gone,
            // Created, exited, paused and the empty case all mean the same
            // thing to us: it exists, its filesystem is there, nothing runs.
            _ => State::Stopped,
        })
    }

    async fn destroy(&self, id: &ComputeId) -> Result<(), Error> {
        let options = RemoveContainerOptionsBuilder::default()
            // A workspace being deleted does not get to refuse.
            .force(true)
            // Take its anonymous volumes with it, or a deleted workspace
            // would leave its disk behind for ever.
            .v(true)
            .build();

        match self
            .docker
            .remove_container(id.as_str(), Some(options))
            .await
        {
            Ok(()) => Ok(()),
            // Already gone is success.
            Err(bollard::errors::Error::DockerResponseServerError {
                status_code: 404, ..
            }) => Ok(()),
            Err(error) => Err(Self::translate(id, error)),
        }
    }
}
