//! Where workspace agents connect.
//!
//! A workspace never listens. It opens one connection outward to here, and
//! everything the platform wants from it travels over that connection as a
//! separate stream. The relay's job is to hold those connections, prove who
//! each one belongs to, and refuse anything it has not been told to allow.
//!
//! It owns no database. Who a credential belongs to is a question for the
//! control plane, asked through [`Authoriser`], so the relay stays a library
//! that can later be its own process without dragging the schema with it.

pub mod transport;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use croncave_protocol::{
    Chunk, Credential, Hello, Outcome, Refusal, Request, Response, StreamKind, StreamOpen, VERSION,
    Welcome, decode, encode,
};
use futures::io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader};
use tokio::sync::{Mutex, mpsc, oneshot};
use uuid::Uuid;

/// How often an agent should tell us it is still there.
pub const HEARTBEAT_SECONDS: u64 = 15;

/// How long to wait for an agent to answer before giving up on a command.
const REPLY_TIMEOUT: Duration = Duration::from_secs(300);

/// Who a credential belongs to.
///
/// Implemented by the control plane, which owns the records. The relay only
/// asks.
#[async_trait::async_trait]
pub trait Authoriser: Send + Sync + std::fmt::Debug {
    /// Check a credential, and say which workspace it is for.
    ///
    /// # Errors
    ///
    /// Returns a [`Refusal`] when the credential is not one we accept. The
    /// three reasons a credential can fail share one answer on purpose: a
    /// finer one would let a caller probe for valid tokens.
    async fn authorise(&self, credential: &Credential) -> Result<Authorised, Refusal>;
}

/// The answer to "who is this?".
#[derive(Debug, Clone)]
pub struct Authorised {
    /// Which workspace the connection belongs to.
    pub workspace_id: Uuid,
    /// A credential to use from now on, when a one-time bootstrap token was
    /// traded in. `None` when the agent already had one.
    pub fresh_credential: Option<String>,
}

/// What can go wrong asking a workspace to do something.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// No agent from that workspace is connected. It may be asleep, or still
    /// waking; either way there is nothing to ask.
    #[error("that workspace is not connected")]
    NotConnected,

    /// The connection is there but did not behave.
    #[error("the workspace's agent did not answer properly")]
    Unusable(String),

    /// The agent did not answer in time.
    #[error("the workspace's agent did not answer in time")]
    TimedOut,
}

/// What a command did.
#[derive(Debug, Clone)]
pub struct Ran {
    /// What it printed, both streams interleaved as they arrived.
    pub output: String,
    /// How it ended.
    pub outcome: Outcome,
}

/// A request to open a stream to an agent, answered on the oneshot.
type OpenStream = oneshot::Sender<Result<yamux::Stream, String>>;

/// One connected workspace.
#[derive(Debug)]
struct Connected {
    /// Ask the connection's own task to open a stream, because only it may
    /// touch the multiplexer.
    open: mpsc::Sender<OpenStream>,
}

/// The connections this relay is holding.
#[derive(Clone)]
pub struct Relay {
    connections: Arc<Mutex<HashMap<Uuid, Connected>>>,
    authoriser: Arc<dyn Authoriser>,
}

impl std::fmt::Debug for Relay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Relay").finish_non_exhaustive()
    }
}

impl Relay {
    /// A relay that asks this authoriser who each connection belongs to.
    #[must_use]
    pub fn new(authoriser: Arc<dyn Authoriser>) -> Self {
        Self {
            connections: Arc::new(Mutex::new(HashMap::new())),
            authoriser,
        }
    }

    /// Whether a workspace's agent is connected right now.
    pub async fn is_connected(&self, workspace_id: Uuid) -> bool {
        self.connections.lock().await.contains_key(&workspace_id)
    }

    /// How many workspaces are connected.
    pub async fn connected_count(&self) -> usize {
        self.connections.lock().await.len()
    }

    /// Run a command inside a workspace and collect what it printed.
    ///
    /// Output streams back as it happens and is gathered here; a caller that
    /// wants it line by line reads the same stream. Anything past
    /// [`croncave_protocol::control::MAX_OUTPUT_BYTES`] is dropped and the
    /// outcome says so, because a workspace runs code we did not write.
    ///
    /// # Errors
    ///
    /// [`Error::NotConnected`] if no agent is there, or [`Error::TimedOut`]
    /// if it stops answering.
    pub async fn run(
        &self,
        workspace_id: Uuid,
        program: &str,
        args: &[String],
        timeout: Duration,
    ) -> Result<Ran, Error> {
        let stream = self.open_stream(workspace_id).await?;
        let command_id = Uuid::now_v7();

        let request = Request::Run {
            id: command_id,
            program: program.to_owned(),
            args: args.to_vec(),
            timeout_seconds: timeout.as_secs(),
        };

        // The agent's own timeout should fire before ours, so a command that
        // overruns is reported as timed out rather than as a dead agent.
        let waiting = timeout.saturating_add(REPLY_TIMEOUT);

        tokio::time::timeout(waiting, converse(stream, command_id, &request))
            .await
            .map_err(|_| Error::TimedOut)?
    }

    /// Ask a workspace whether it is still there.
    ///
    /// # Errors
    ///
    /// [`Error::NotConnected`] if no agent is there.
    pub async fn ping(&self, workspace_id: Uuid) -> Result<(), Error> {
        let mut stream = self.open_stream(workspace_id).await?;

        write_line(&mut stream, &StreamOpen::new(StreamKind::Control)).await?;
        write_line(&mut stream, &Request::Ping).await?;

        let mut reader = BufReader::new(stream);
        match read_line::<Response>(&mut reader).await? {
            Some(Response::Pong) => Ok(()),
            other => Err(Error::Unusable(format!("expected a pong, got {other:?}"))),
        }
    }

    /// Open a stream to a workspace's agent.
    async fn open_stream(&self, workspace_id: Uuid) -> Result<yamux::Stream, Error> {
        let sender = {
            let connections = self.connections.lock().await;
            connections
                .get(&workspace_id)
                .ok_or(Error::NotConnected)?
                .open
                .clone()
        };

        let (reply, answer) = oneshot::channel();
        sender.send(reply).await.map_err(|_| Error::NotConnected)?;

        answer
            .await
            .map_err(|_| Error::NotConnected)?
            .map_err(Error::Unusable)
    }
}

/// Send a request and gather everything that comes back for it.
async fn converse(
    mut stream: yamux::Stream,
    command_id: Uuid,
    request: &Request,
) -> Result<Ran, Error> {
    write_line(
        &mut stream,
        &StreamOpen::about(StreamKind::Control, command_id),
    )
    .await?;
    write_line(&mut stream, request).await?;

    let mut reader = BufReader::new(stream);
    let mut output = String::new();

    loop {
        // Every line is either a piece of output or the outcome, so one
        // reader handles both and output arrives as it happens.
        let Some(line) = read_raw_line(&mut reader).await? else {
            return Err(Error::Unusable(
                "the agent stopped talking before saying how it went".to_owned(),
            ));
        };

        if let Ok(chunk) = decode::<Chunk>(&line) {
            output.push_str(&chunk.text);
            continue;
        }

        return match decode::<Response>(&line) {
            Ok(Response::Finished { outcome, .. }) => Ok(Ran { output, outcome }),
            Ok(Response::Failed { message, .. }) => Err(Error::Unusable(message)),
            Ok(other) => Err(Error::Unusable(format!("unexpected answer: {other:?}"))),
            Err(error) => Err(Error::Unusable(format!(
                "could not read the answer: {error}"
            ))),
        };
    }
}

/// Hold a connected agent until it goes away.
///
/// The hello exchange happens before the multiplexer starts, so a connection
/// that cannot prove who it is never gets one.
///
/// # Errors
///
/// Returns an error if the agent never says hello, says something we cannot
/// read, or is not authorised.
pub async fn serve_agent(relay: Relay, socket: axum::extract::ws::WebSocket) {
    let mut bytes = transport::Bytes::new(socket);

    let workspace_id = match greet(&relay, &mut bytes).await {
        Ok(id) => id,
        Err(reason) => {
            tracing::info!(%reason, "refused an agent connection");
            return;
        }
    };

    let (open_sender, open_receiver) = mpsc::channel::<OpenStream>(8);

    relay
        .connections
        .lock()
        .await
        .insert(workspace_id, Connected { open: open_sender });

    tracing::info!(%workspace_id, "a workspace connected");

    drive(bytes, open_receiver).await;

    relay.connections.lock().await.remove(&workspace_id);
    tracing::info!(%workspace_id, "a workspace disconnected");
}

/// The hello exchange, before anything else is allowed.
async fn greet(relay: &Relay, bytes: &mut transport::Bytes) -> Result<Uuid, String> {
    let mut reader = BufReader::new(&mut *bytes);

    let line = read_raw_line(&mut reader)
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "the agent said nothing".to_owned())?;

    let hello: Hello = decode(&line).map_err(|error| format!("unreadable hello: {error}"))?;

    // Version first: an agent we cannot talk to gets a clear answer rather
    // than a confusing failure later on.
    if hello.protocol_version != VERSION {
        let _ = write_line(
            bytes,
            &Welcome::Refused {
                reason: Refusal::UnsupportedVersion,
            },
        )
        .await;
        return Err(format!(
            "agent speaks protocol {}, this relay speaks {VERSION}",
            hello.protocol_version
        ));
    }

    match relay.authoriser.authorise(&hello.credential).await {
        Ok(authorised) => {
            write_line(
                bytes,
                &Welcome::Accepted {
                    workspace_id: authorised.workspace_id,
                    credential: authorised.fresh_credential,
                    heartbeat_seconds: HEARTBEAT_SECONDS,
                },
            )
            .await
            .map_err(|error| error.to_string())?;

            Ok(authorised.workspace_id)
        }
        Err(reason) => {
            let _ = write_line(bytes, &Welcome::Refused { reason }).await;
            Err(reason.to_string())
        }
    }
}

/// Run the multiplexer until the connection ends.
///
/// Only this task touches it, which is why opening a stream goes through a
/// channel rather than a lock.
async fn drive(bytes: transport::Bytes, mut open: mpsc::Receiver<OpenStream>) {
    let mut connection =
        yamux::Connection::new(bytes, yamux::Config::default(), yamux::Mode::Server);

    loop {
        tokio::select! {
            // Someone wants a stream to this workspace.
            request = open.recv() => {
                let Some(reply) = request else { break };
                let opened = std::future::poll_fn(|cx| connection.poll_new_outbound(cx)).await;
                let _ = reply.send(opened.map_err(|error| error.to_string()));
            }
            // The agent opened one. Nothing sends to us yet, so these are
            // accepted and dropped rather than left to stall the connection.
            inbound = std::future::poll_fn(|cx| connection.poll_next_inbound(cx)) => {
                match inbound {
                    Some(Ok(_stream)) => {}
                    Some(Err(error)) => {
                        tracing::debug!(%error, "a workspace connection ended");
                        break;
                    }
                    None => break,
                }
            }
        }
    }
}

/// Write one JSON line.
async fn write_line<W, T>(writer: &mut W, message: &T) -> Result<(), Error>
where
    W: futures::AsyncWrite + Unpin,
    T: serde::Serialize,
{
    let bytes = encode(message).map_err(|error| Error::Unusable(error.to_string()))?;
    writer
        .write_all(&bytes)
        .await
        .map_err(|error| Error::Unusable(error.to_string()))?;
    writer
        .flush()
        .await
        .map_err(|error| Error::Unusable(error.to_string()))
}

/// Read one line, or nothing if the other side hung up.
async fn read_raw_line<R>(reader: &mut BufReader<R>) -> Result<Option<Vec<u8>>, Error>
where
    R: futures::AsyncRead + Unpin,
{
    let mut line = Vec::new();
    let read = reader
        .read_until(b'\n', &mut line)
        .await
        .map_err(|error| Error::Unusable(error.to_string()))?;

    if read == 0 {
        return Ok(None);
    }

    while line.last() == Some(&b'\n') || line.last() == Some(&b'\r') {
        line.pop();
    }

    Ok(Some(line))
}

/// Read one line and make sense of it.
async fn read_line<T>(reader: &mut BufReader<yamux::Stream>) -> Result<Option<T>, Error>
where
    T: for<'de> serde::Deserialize<'de>,
{
    match read_raw_line(reader).await? {
        Some(line) => decode(&line)
            .map(Some)
            .map_err(|error| Error::Unusable(error.to_string())),
        None => Ok(None),
    }
}
