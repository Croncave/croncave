//! What the agent and the relay say to each other.
//!
//! One workspace holds one connection, opened by the workspace, and every
//! kind of traffic travels over it as a separate stream. This crate defines
//! the words; `croncave-relay` and `croncave-agent` do the talking.
//!
//! # Changing this
//!
//! An agent is baked into a workspace image and may be months older than the
//! relay it meets. So:
//!
//! - **Adding a field is safe.** Both sides ignore fields they don't know.
//! - **Adding a variant is safe for the sender that is newer**, and the older
//!   side must say it didn't understand rather than guess.
//! - **Changing the meaning of anything is not safe.** Add a new field or a
//!   new variant instead, and remove the old one a release later — the same
//!   expand-then-contract rule the database migrations follow.
//! - **[`VERSION`] only goes up when a change is not backwards compatible**,
//!   and then old agents are refused with a reason rather than left to
//!   misbehave.

pub mod control;
pub mod stream;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub use control::{Chunk, Outcome, OutputStream, Request, Response};
pub use stream::{StreamKind, StreamOpen};

/// The version of this protocol.
///
/// Bumped only for a change an older agent could not survive. The relay
/// refuses anything it does not speak, with a reason.
pub const VERSION: u32 = 1;

/// The first thing an agent says, before anything else is allowed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Hello {
    /// The protocol the agent speaks.
    pub protocol_version: u32,
    /// The build of the agent, for the details layer and for knowing which
    /// workspaces need an update.
    pub agent_version: String,
    /// How the agent proves who it is.
    pub credential: Credential,
}

/// How an agent proves which workspace it is.
///
/// A workspace is handed a one-time bootstrap token when it starts. The agent
/// trades it for a credential it keeps for the life of the connection. Both
/// are secrets, so neither is ever logged, and only their hashes are stored.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Credential {
    /// First connection: the one-time token the orchestrator put in the
    /// workspace. Works once.
    Bootstrap {
        /// The token itself.
        token: String,
    },
    /// Later connections: what the bootstrap token was traded for.
    Workspace {
        /// The credential itself.
        token: String,
    },
}

impl std::fmt::Debug for Credential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The whole point is that these never reach a log.
        match self {
            Self::Bootstrap { .. } => f.write_str("Credential::Bootstrap(<secret>)"),
            Self::Workspace { .. } => f.write_str("Credential::Workspace(<secret>)"),
        }
    }
}

/// What the relay says back.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Welcome {
    /// Accepted. The connection is open for business.
    Accepted {
        /// Which workspace the relay believes this is.
        workspace_id: Uuid,
        /// Present only when a bootstrap token was traded in: the credential
        /// to use from now on. The agent keeps it for this connection and
        /// asks for a new one when it reconnects.
        credential: Option<String>,
        /// How often the agent should send a heartbeat.
        heartbeat_seconds: u64,
    },
    /// Refused, with a reason the agent can log and act on.
    Refused {
        /// Why.
        reason: Refusal,
    },
}

/// Why a connection was refused.
///
/// Deliberately coarse. An agent learning exactly why its credential failed
/// would be a way to probe the relay, so the three credential failures share
/// one answer.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Refusal {
    /// The agent speaks a protocol this relay does not.
    UnsupportedVersion,
    /// The credential is unknown, used, or expired.
    NotAuthorised,
    /// The relay is up but not taking connections.
    Unavailable,
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::UnsupportedVersion => "this relay does not speak the agent's protocol version",
            Self::NotAuthorised => "the credential was not accepted",
            Self::Unavailable => "the relay is not taking connections",
        })
    }
}

/// Anything that can go wrong reading or writing a message.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The bytes were not the message they claimed to be.
    #[error("could not read a protocol message")]
    Malformed(#[from] serde_json::Error),

    /// A message was larger than anything we are willing to read, which is
    /// how a connection is stopped from being used to exhaust memory.
    #[error("a protocol message was {size} bytes, over the {limit} limit")]
    TooLarge {
        /// How big it claimed to be.
        size: usize,
        /// The most we will read.
        limit: usize,
    },
}

/// The most a single control message may be.
///
/// Control messages are small by design; output travels on its own stream as
/// raw bytes. A limit here means a confused or hostile agent cannot ask the
/// relay to allocate without bound.
pub const MAX_MESSAGE_BYTES: usize = 64 * 1024;

/// Write a message as one line of JSON.
///
/// # Errors
///
/// Returns an error if the message cannot be encoded, or is over the limit.
pub fn encode<T: Serialize>(message: &T) -> Result<Vec<u8>, Error> {
    let mut bytes = serde_json::to_vec(message)?;

    if bytes.len() > MAX_MESSAGE_BYTES {
        return Err(Error::TooLarge {
            size: bytes.len(),
            limit: MAX_MESSAGE_BYTES,
        });
    }

    bytes.push(b'\n');
    Ok(bytes)
}

/// Read a message.
///
/// # Errors
///
/// Returns an error if the bytes are over the limit or are not that message.
pub fn decode<T: for<'de> Deserialize<'de>>(bytes: &[u8]) -> Result<T, Error> {
    if bytes.len() > MAX_MESSAGE_BYTES {
        return Err(Error::TooLarge {
            size: bytes.len(),
            limit: MAX_MESSAGE_BYTES,
        });
    }

    Ok(serde_json::from_slice(bytes)?)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    #[test]
    fn a_hello_survives_the_round_trip() {
        let hello = Hello {
            protocol_version: VERSION,
            agent_version: "0.1.0".to_owned(),
            credential: Credential::Bootstrap {
                token: "abc".to_owned(),
            },
        };

        let bytes = encode(&hello).unwrap();
        let back: Hello = decode(bytes.trim_ascii_end()).unwrap();

        assert_eq!(back, hello);
    }

    #[test]
    fn a_welcome_survives_the_round_trip() {
        let welcome = Welcome::Accepted {
            workspace_id: Uuid::now_v7(),
            credential: Some("secret".to_owned()),
            heartbeat_seconds: 15,
        };

        let bytes = encode(&welcome).unwrap();
        let back: Welcome = decode(bytes.trim_ascii_end()).unwrap();

        assert_eq!(back, welcome);
    }

    #[test]
    fn a_field_we_do_not_know_is_ignored() {
        // An older relay meeting a newer agent must not fall over.
        let from_the_future = br#"{"protocol_version":1,"agent_version":"9.0.0",
            "credential":{"kind":"bootstrap","token":"abc"},"something_new":true}"#;

        let hello: Hello = decode(from_the_future).expect("unknown fields are ignored");

        assert_eq!(hello.agent_version, "9.0.0");
    }

    #[test]
    fn a_variant_we_do_not_know_is_an_error_not_a_guess() {
        let from_the_future = br#"{"kind":"delegated","token":"abc"}"#;

        let result: Result<Credential, _> = decode(from_the_future);

        assert!(
            result.is_err(),
            "an unknown way of proving identity must be refused, never assumed"
        );
    }

    #[test]
    fn a_credential_never_appears_in_a_debug_line() {
        let credential = Credential::Workspace {
            token: "sup3rsecret".to_owned(),
        };

        let printed = format!("{credential:?}");

        assert!(!printed.contains("sup3rsecret"), "{printed}");
        assert!(printed.contains("<secret>"), "{printed}");
    }

    #[test]
    fn a_hello_never_carries_its_credential_into_a_debug_line() {
        let hello = Hello {
            protocol_version: VERSION,
            agent_version: "0.1.0".to_owned(),
            credential: Credential::Bootstrap {
                token: "sup3rsecret".to_owned(),
            },
        };

        assert!(!format!("{hello:?}").contains("sup3rsecret"));
    }

    #[test]
    fn a_message_over_the_limit_is_refused_rather_than_allocated() {
        let huge = "x".repeat(MAX_MESSAGE_BYTES + 1);
        let bytes = huge.into_bytes();

        let result: Result<Hello, _> = decode(&bytes);

        assert!(matches!(result, Err(Error::TooLarge { .. })));
    }
}
