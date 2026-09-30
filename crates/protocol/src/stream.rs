//! What a stream is for.
//!
//! Everything a workspace does travels over one connection, split into
//! streams. The first thing sent on a new stream says what it is for; the
//! other side either understands it or closes it.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// What a stream carries.
///
/// `docs/architecture.md` lists six. Three exist now; the rest are named so
/// that an agent meeting a relay that opens one can refuse it clearly rather
/// than mishandle it.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StreamKind {
    /// Relay to agent: start something, stop something, ask how it is.
    Control,
    /// Agent to relay: the output of something running.
    Output,
    /// Both ways, often enough that a dropped connection is noticed quickly.
    Heartbeat,
    /// Both ways: files in and out. Step 4 and later.
    Files,
    /// Both ways: web traffic to a port inside the workspace. Step 8.
    Preview,
    /// Both ways: a shell. R2, when the browser terminal arrives.
    Terminal,
}

impl StreamKind {
    /// Whether this relay knows how to handle the kind.
    ///
    /// A kind we recognise but cannot yet serve is refused with a clear
    /// reason, which is what lets the protocol grow without every version
    /// having to arrive everywhere at once.
    #[must_use]
    pub fn is_supported(self) -> bool {
        matches!(self, Self::Control | Self::Output | Self::Heartbeat)
    }
}

/// The first message on a new stream, saying what it is for.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StreamOpen {
    /// What it carries.
    pub kind: StreamKind,
    /// What it is about, when that is a particular piece of work. A control
    /// stream has none; an output stream names the command it belongs to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<Uuid>,
}

impl StreamOpen {
    /// A stream with no particular subject.
    #[must_use]
    pub fn new(kind: StreamKind) -> Self {
        Self {
            kind,
            subject: None,
        }
    }

    /// A stream about one piece of work.
    #[must_use]
    pub fn about(kind: StreamKind, subject: Uuid) -> Self {
        Self {
            kind,
            subject: Some(subject),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;
    use crate::{decode, encode};

    #[test]
    fn opening_a_stream_survives_the_round_trip() {
        let open = StreamOpen::about(StreamKind::Output, Uuid::now_v7());

        let bytes = encode(&open).unwrap();
        let back: StreamOpen = decode(bytes.trim_ascii_end()).unwrap();

        assert_eq!(back, open);
    }

    #[test]
    fn a_stream_with_no_subject_does_not_carry_an_empty_one() {
        let bytes = encode(&StreamOpen::new(StreamKind::Control)).unwrap();
        let text = String::from_utf8(bytes).unwrap();

        assert!(!text.contains("subject"), "{text}");
    }

    #[test]
    fn the_kinds_we_serve_are_the_ones_this_step_built() {
        assert!(StreamKind::Control.is_supported());
        assert!(StreamKind::Output.is_supported());
        assert!(StreamKind::Heartbeat.is_supported());

        // Named so they can be refused clearly, not handled badly.
        assert!(!StreamKind::Files.is_supported());
        assert!(!StreamKind::Preview.is_supported());
        assert!(!StreamKind::Terminal.is_supported());
    }

    #[test]
    fn a_stream_kind_we_do_not_know_is_an_error() {
        let result: Result<StreamKind, _> = decode(br#""quantum""#);

        assert!(result.is_err(), "an unknown stream kind must be refused");
    }
}
