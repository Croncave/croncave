//! What the relay asks a workspace to do, and what it hears back.
//!
//! One request, one response, over the control stream. Output does not come
//! back this way: a command's output arrives on its own stream tagged with
//! the command's id, so a long-running command streams rather than holding
//! everything until it finishes. Step 4's sessions and step 6's runs are
//! built on exactly this.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The most output we will accept from one command.
///
/// A workspace runs code we did not write, and some of it prints for ever.
/// Past this the output is truncated and the command is told about it,
/// rather than the relay being asked to hold an unbounded amount.
pub const MAX_OUTPUT_BYTES: usize = 1024 * 1024;

/// Something the relay asks the agent to do.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Request {
    /// Run a command, and stream its output on a stream tagged with `id`.
    Run {
        /// Names the command, and the output stream that belongs to it.
        id: Uuid,
        /// The program to run.
        program: String,
        /// Its arguments.
        #[serde(default)]
        args: Vec<String>,
        /// Give up after this long. A workspace runs code we did not write,
        /// so nothing may run for ever by accident.
        timeout_seconds: u64,
    },
    /// Stop a command that is still running.
    Cancel {
        /// Which one.
        id: Uuid,
    },
    /// Are you there?
    Ping,
}

/// What the agent says back.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Response {
    /// The command finished, one way or another.
    Finished {
        /// Which command.
        id: Uuid,
        /// How it ended.
        outcome: Outcome,
    },
    /// Still here.
    Pong,
    /// The request could not be carried out.
    Failed {
        /// Which command, when the failure was about one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<Uuid>,
        /// What went wrong, in words a person can act on.
        message: String,
    },
}

/// How a command ended.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Outcome {
    /// It exited on its own.
    Exited {
        /// Its exit code. Zero is success, as everywhere else.
        code: i32,
        /// Whether output was cut short at [`MAX_OUTPUT_BYTES`].
        #[serde(default)]
        truncated: bool,
    },
    /// It was still going when its time ran out, and was stopped.
    TimedOut,
    /// Something asked for it to stop.
    Cancelled,
}

impl Outcome {
    /// Whether this counts as having worked.
    #[must_use]
    pub fn succeeded(self) -> bool {
        matches!(self, Self::Exited { code: 0, .. })
    }

    /// The word the product uses for it.
    #[must_use]
    pub fn word(self) -> &'static str {
        match self {
            Self::Exited { code: 0, .. } => "done",
            Self::Exited { .. } => "failed",
            Self::TimedOut => "timed out",
            Self::Cancelled => "stopped",
        }
    }
}

/// One piece of a command's output, as it happens.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Chunk {
    /// Which of the two it came from.
    pub stream: OutputStream,
    /// The bytes, as text. Output that is not valid UTF-8 is replaced rather
    /// than dropped, because a garbled line is more useful than none.
    pub text: String,
}

/// Where a piece of output came from.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OutputStream {
    /// What the command printed.
    Stdout,
    /// What it complained about.
    Stderr,
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;
    use crate::{decode, encode};

    #[test]
    fn a_request_survives_the_round_trip() {
        let request = Request::Run {
            id: Uuid::now_v7(),
            program: "echo".to_owned(),
            args: vec!["hello".to_owned()],
            timeout_seconds: 30,
        };

        let bytes = encode(&request).unwrap();
        assert_eq!(decode::<Request>(bytes.trim_ascii_end()).unwrap(), request);
    }

    #[test]
    fn a_response_survives_the_round_trip() {
        let response = Response::Finished {
            id: Uuid::now_v7(),
            outcome: Outcome::Exited {
                code: 0,
                truncated: false,
            },
        };

        let bytes = encode(&response).unwrap();
        assert_eq!(
            decode::<Response>(bytes.trim_ascii_end()).unwrap(),
            response
        );
    }

    #[test]
    fn arguments_may_be_left_out() {
        let request: Request =
            decode(br#"{"kind":"run","id":"01920000-0000-7000-8000-000000000000","program":"ls","timeout_seconds":5}"#)
                .expect("args default to none");

        match request {
            Request::Run { args, .. } => assert!(args.is_empty()),
            other => panic!("expected a run, got {other:?}"),
        }
    }

    #[test]
    fn only_a_zero_exit_counts_as_having_worked() {
        assert!(
            Outcome::Exited {
                code: 0,
                truncated: false
            }
            .succeeded()
        );
        assert!(
            !Outcome::Exited {
                code: 1,
                truncated: false
            }
            .succeeded()
        );
        assert!(!Outcome::TimedOut.succeeded());
        assert!(!Outcome::Cancelled.succeeded());
    }

    #[test]
    fn every_outcome_has_a_word() {
        for outcome in [
            Outcome::Exited {
                code: 0,
                truncated: false,
            },
            Outcome::Exited {
                code: 2,
                truncated: true,
            },
            Outcome::TimedOut,
            Outcome::Cancelled,
        ] {
            assert!(!outcome.word().is_empty());
        }
    }
}
