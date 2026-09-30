//! The agent that lives inside a workspace.
//!
//! It opens one connection outward to the relay and does what comes back
//! down it. **It never listens.** There is no port to connect to, no address
//! to reach, and nothing in this crate binds one — which is the whole
//! security model of Croncave in one sentence.
//!
//! If the connection drops, work carries on: a command already running is a
//! process in the workspace, not something held up by a socket. The agent
//! reconnects with backoff and carries on.

pub mod run;

use std::time::Duration;

use croncave_protocol::{
    Credential, Hello, Refusal, Request, Response, StreamOpen, VERSION, Welcome, decode, encode,
};
use futures::io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader};
use tokio_tungstenite::tungstenite;

mod transport;

/// How long to wait before the first reconnection attempt.
const BACKOFF_START: Duration = Duration::from_millis(500);

/// The longest we ever wait between attempts. A workspace that has been
/// unreachable for a while should still notice quickly when the relay is
/// back, so this stays short.
const BACKOFF_MAX: Duration = Duration::from_secs(30);

/// What the agent needs to know to start.
#[derive(Clone)]
pub struct Config {
    /// Where the relay is.
    pub relay_url: String,
    /// The one-time token this workspace was started with.
    pub bootstrap_token: String,
    /// This build, so the platform knows which workspaces need updating.
    pub agent_version: String,
}

impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("relay_url", &self.relay_url)
            .field("bootstrap_token", &"<secret>")
            .field("agent_version", &self.agent_version)
            .finish()
    }
}

/// Why the agent could not do its job.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The relay could not be reached.
    #[error("could not reach the relay")]
    Unreachable(#[source] Box<dyn std::error::Error + Send + Sync>),

    /// The relay said no. Reconnecting will not help until something changes.
    #[error("the relay refused this workspace: {0}")]
    Refused(Refusal),

    /// The connection misbehaved.
    #[error("the connection failed: {0}")]
    Connection(String),
}

impl Error {
    /// Whether trying again could work.
    ///
    /// A refusal is not worth retrying in a tight loop: the credential will
    /// not become valid on its own, and hammering the relay helps nobody.
    #[must_use]
    pub fn worth_retrying(&self) -> bool {
        !matches!(self, Self::Refused(_))
    }
}

/// Connect, and keep reconnecting for as long as the process lives.
///
/// Returns only when the connection is refused in a way that retrying cannot
/// fix, or when the process is asked to stop.
pub async fn run_forever(config: Config) {
    let mut backoff = BACKOFF_START;
    // Swapped in once a bootstrap token has been traded, so a reconnection
    // does not try to spend a one-time token twice.
    let mut credential = Credential::Bootstrap {
        token: config.bootstrap_token.clone(),
    };

    loop {
        match connect_once(&config, &credential).await {
            Ok(fresh) => {
                if let Some(token) = fresh {
                    credential = Credential::Workspace { token };
                }
                tracing::info!("the connection to the relay ended; reconnecting");
                backoff = BACKOFF_START;
            }
            Err(error) if error.worth_retrying() => {
                tracing::warn!(%error, backoff_ms = backoff.as_millis(), "retrying");
            }
            Err(error) => {
                tracing::error!(%error, "giving up");
                return;
            }
        }

        tokio::time::sleep(backoff).await;
        backoff = (backoff * 2).min(BACKOFF_MAX);
    }
}

/// One connection, from dialling to the far side hanging up.
///
/// Returns the credential to use next time, when a bootstrap token was
/// traded in.
async fn connect_once(config: &Config, credential: &Credential) -> Result<Option<String>, Error> {
    let (socket, _) = tokio_tungstenite::connect_async(&config.relay_url)
        .await
        .map_err(|error| Error::Unreachable(Box::new(error)))?;

    let mut bytes = transport::Bytes::new(socket);

    let hello = Hello {
        protocol_version: VERSION,
        agent_version: config.agent_version.clone(),
        credential: credential.clone(),
    };

    write_line(&mut bytes, &hello).await?;

    let welcome: Welcome = {
        let mut reader = BufReader::new(&mut bytes);
        read_line(&mut reader)
            .await?
            .ok_or_else(|| Error::Connection("the relay said nothing".to_owned()))?
    };

    let fresh = match welcome {
        Welcome::Accepted {
            workspace_id,
            credential,
            ..
        } => {
            tracing::info!(%workspace_id, "connected to the relay");
            credential
        }
        Welcome::Refused { reason } => return Err(Error::Refused(reason)),
    };

    serve(bytes).await;

    Ok(fresh)
}

/// Answer whatever the relay asks, until the connection ends.
async fn serve(bytes: transport::Bytes) {
    let mut connection =
        yamux::Connection::new(bytes, yamux::Config::default(), yamux::Mode::Client);

    loop {
        let inbound = std::future::poll_fn(|cx| connection.poll_next_inbound(cx)).await;

        match inbound {
            Some(Ok(stream)) => {
                // Each request gets its own task, so a long command does not
                // stop the workspace answering anything else.
                tokio::spawn(async move {
                    if let Err(error) = handle(stream).await {
                        tracing::warn!(%error, "a request went wrong");
                    }
                });
            }
            Some(Err(error)) => {
                tracing::debug!(%error, "the connection ended");
                return;
            }
            None => return,
        }
    }
}

/// One stream: what it is for, then what to do.
async fn handle(stream: yamux::Stream) -> Result<(), Error> {
    let mut reader = BufReader::new(stream);

    let open: StreamOpen = read_line(&mut reader)
        .await?
        .ok_or_else(|| Error::Connection("a stream said nothing".to_owned()))?;

    if !open.kind.is_supported() {
        // Named in the protocol but not built here: say so plainly rather
        // than leaving the relay waiting.
        let mut stream = reader.into_inner();
        return write_line(
            &mut stream,
            &Response::Failed {
                id: open.subject,
                message: format!("this agent does not serve {:?} streams", open.kind),
            },
        )
        .await;
    }

    let Some(request) = read_line::<Request>(&mut reader).await? else {
        return Ok(());
    };

    let mut stream = reader.into_inner();

    match request {
        Request::Ping => write_line(&mut stream, &Response::Pong).await,
        Request::Run {
            id,
            program,
            args,
            timeout_seconds,
        } => {
            let outcome = run::run(
                &program,
                &args,
                Duration::from_secs(timeout_seconds),
                &mut stream,
            )
            .await;

            write_line(&mut stream, &Response::Finished { id, outcome }).await
        }
        Request::Cancel { id } => {
            // Cancelling is per-connection today: a command's own stream is
            // where it is stopped, by dropping it. Nothing to look up yet.
            write_line(
                &mut stream,
                &Response::Failed {
                    id: Some(id),
                    message: "nothing to cancel on this stream".to_owned(),
                },
            )
            .await
        }
    }
}

/// Which relay to talk to, and with what, from the environment.
///
/// # Errors
///
/// Returns a description of what is missing.
pub fn config_from_env(agent_version: &str) -> Result<Config, String> {
    let relay_url = std::env::var("CRONCAVE_RELAY_URL")
        .map_err(|_| "CRONCAVE_RELAY_URL is not set".to_owned())?;
    let bootstrap_token = std::env::var("CRONCAVE_BOOTSTRAP_TOKEN")
        .map_err(|_| "CRONCAVE_BOOTSTRAP_TOKEN is not set".to_owned())?;

    Ok(Config {
        relay_url,
        bootstrap_token,
        agent_version: agent_version.to_owned(),
    })
}

async fn write_line<W, T>(writer: &mut W, message: &T) -> Result<(), Error>
where
    W: futures::AsyncWrite + Unpin,
    T: serde::Serialize,
{
    let bytes = encode(message).map_err(|error| Error::Connection(error.to_string()))?;
    writer
        .write_all(&bytes)
        .await
        .map_err(|error| Error::Connection(error.to_string()))?;
    writer
        .flush()
        .await
        .map_err(|error| Error::Connection(error.to_string()))
}

async fn read_line<T>(
    reader: &mut BufReader<impl futures::AsyncRead + Unpin>,
) -> Result<Option<T>, Error>
where
    T: for<'de> serde::Deserialize<'de>,
{
    let mut line = Vec::new();
    let read = reader
        .read_until(b'\n', &mut line)
        .await
        .map_err(|error| Error::Connection(error.to_string()))?;

    if read == 0 {
        return Ok(None);
    }

    while line.last() == Some(&b'\n') || line.last() == Some(&b'\r') {
        line.pop();
    }

    decode(&line)
        .map(Some)
        .map_err(|error| Error::Connection(error.to_string()))
}

/// The tungstenite error type, re-exported so binaries can match on it.
pub use tungstenite::Error as WebSocketError;
