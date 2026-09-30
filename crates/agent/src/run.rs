//! Running a command inside the workspace.
//!
//! Output is sent back as it happens rather than held until the end, so a
//! long command is visible while it runs. Two limits keep a workspace from
//! taking the platform with it: how much it may print, and how long it may
//! take. A workspace runs code we did not write.

use std::process::Stdio;
use std::time::Duration;

use croncave_protocol::control::MAX_OUTPUT_BYTES;
use croncave_protocol::{Chunk, Outcome, OutputStream, encode};
use futures::AsyncWrite;
use futures::io::AsyncWriteExt as _;
use tokio::io::{AsyncBufReadExt as _, BufReader};
use tokio::process::Command;

/// Run a command, sending its output down `out` as it arrives.
pub async fn run<W>(program: &str, args: &[String], timeout: Duration, out: &mut W) -> Outcome
where
    W: AsyncWrite + Unpin,
{
    let spawned = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        // Its own process group, so stopping it stops what it started.
        .kill_on_drop(true)
        .spawn();

    let mut child = match spawned {
        Ok(child) => child,
        Err(error) => {
            send(
                out,
                &Chunk {
                    stream: OutputStream::Stderr,
                    text: format!("could not run {program}: {error}\n"),
                },
            )
            .await;
            return Outcome::Exited {
                code: 127,
                truncated: false,
            };
        }
    };

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let mut sent = 0_usize;
    let mut truncated = false;

    let mut stdout = stdout.map(|handle| BufReader::new(handle).lines());
    let mut stderr = stderr.map(|handle| BufReader::new(handle).lines());

    let work = async {
        loop {
            let line = tokio::select! {
                line = next_line(&mut stdout) => line.map(|text| (OutputStream::Stdout, text)),
                line = next_line(&mut stderr) => line.map(|text| (OutputStream::Stderr, text)),
                status = child.wait() => {
                    // Drain whatever is left before reporting the exit, or
                    // the last lines of output would be lost.
                    drain(&mut stdout, OutputStream::Stdout, out, &mut sent, &mut truncated).await;
                    drain(&mut stderr, OutputStream::Stderr, out, &mut sent, &mut truncated).await;
                    return status;
                }
            };

            let Some((stream, text)) = line else { continue };

            if sent >= MAX_OUTPUT_BYTES {
                truncated = true;
                continue;
            }

            sent += text.len();
            send(
                out,
                &Chunk {
                    stream,
                    text: format!("{text}\n"),
                },
            )
            .await;
        }
    };

    match tokio::time::timeout(timeout, work).await {
        Ok(Ok(status)) => Outcome::Exited {
            // A process killed by a signal has no code; report it the way a
            // shell does rather than pretending it succeeded.
            code: status.code().unwrap_or(-1),
            truncated,
        },
        Ok(Err(error)) => {
            send(
                out,
                &Chunk {
                    stream: OutputStream::Stderr,
                    text: format!("could not wait for {program}: {error}\n"),
                },
            )
            .await;
            Outcome::Exited {
                code: -1,
                truncated,
            }
        }
        Err(_) => {
            // kill_on_drop stops it when the child is dropped, but being
            // explicit means the process is gone before we say so.
            let _ = child.start_kill();
            Outcome::TimedOut
        }
    }
}

/// The next line from one of the two pipes, or never if it has ended.
async fn next_line<R>(lines: &mut Option<tokio::io::Lines<BufReader<R>>>) -> Option<String>
where
    R: tokio::io::AsyncRead + Unpin,
{
    match lines {
        Some(lines) => lines.next_line().await.ok().flatten(),
        // Pending for ever, so `select!` simply stops choosing this branch.
        None => std::future::pending().await,
    }
}

/// Whatever is left in a pipe once the process has gone.
async fn drain<W, R>(
    lines: &mut Option<tokio::io::Lines<BufReader<R>>>,
    stream: OutputStream,
    out: &mut W,
    sent: &mut usize,
    truncated: &mut bool,
) where
    W: AsyncWrite + Unpin,
    R: tokio::io::AsyncRead + Unpin,
{
    let Some(lines) = lines else { return };

    while let Ok(Some(text)) = lines.next_line().await {
        if *sent >= MAX_OUTPUT_BYTES {
            *truncated = true;
            return;
        }
        *sent += text.len();
        send(
            out,
            &Chunk {
                stream,
                text: format!("{text}\n"),
            },
        )
        .await;
    }
}

/// Send one piece of output. A write that fails means the relay has gone,
/// which the connection loop will notice; there is nothing useful to do here.
async fn send<W: AsyncWrite + Unpin>(out: &mut W, chunk: &Chunk) {
    if let Ok(bytes) = encode(chunk) {
        let _ = out.write_all(&bytes).await;
        let _ = out.flush().await;
    }
}
