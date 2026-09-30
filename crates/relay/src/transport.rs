//! Turning a WebSocket into something a stream multiplexer can use.
//!
//! A WebSocket carries messages; yamux wants a byte stream. Binary frames
//! become bytes in, and writes become binary frames out, so the multiplexer
//! sees an ordinary connection and the WebSocket does what it is good at:
//! crossing proxies on port 443 without special network setup.

use std::io;
use std::pin::Pin;
use std::task::{Context, Poll, ready};

use axum::extract::ws::{Message, WebSocket};
use futures::sink::Sink;
use futures::stream::{SplitSink, SplitStream, Stream, StreamExt as _};
use futures::{AsyncRead, AsyncWrite};

/// A WebSocket, seen as a stream of bytes.
pub struct Bytes {
    sink: SplitSink<WebSocket, Message>,
    stream: SplitStream<WebSocket>,
    /// What is left of the frame we are part way through handing out.
    pending: Vec<u8>,
    read: usize,
}

impl Bytes {
    /// Wrap a socket so a multiplexer can run over it.
    #[must_use]
    pub fn new(socket: WebSocket) -> Self {
        let (sink, stream) = socket.split();
        Self {
            sink,
            stream,
            pending: Vec::new(),
            read: 0,
        }
    }
}

impl AsyncRead for Bytes {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        let me = self.get_mut();

        loop {
            // Hand out whatever is left of the last frame first.
            if me.read < me.pending.len() {
                let take = (me.pending.len() - me.read).min(buf.len());
                buf[..take].copy_from_slice(&me.pending[me.read..me.read + take]);
                me.read += take;
                return Poll::Ready(Ok(take));
            }

            match ready!(Pin::new(&mut me.stream).poll_next(cx)) {
                Some(Ok(Message::Binary(bytes))) => {
                    me.pending = bytes.into();
                    me.read = 0;
                    // An empty frame carries nothing; wait for the next.
                    if me.pending.is_empty() {
                        continue;
                    }
                }
                // Pings, pongs and text are not ours to interpret. Ignoring
                // them keeps the byte stream clean.
                Some(Ok(_)) => continue,
                Some(Err(error)) => return Poll::Ready(Err(io::Error::other(error))),
                // The far side hung up: end of stream, not an error.
                None => return Poll::Ready(Ok(0)),
            }
        }
    }
}

impl AsyncWrite for Bytes {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let me = self.get_mut();

        ready!(Pin::new(&mut me.sink).poll_ready(cx)).map_err(io::Error::other)?;

        Pin::new(&mut me.sink)
            .start_send(Message::Binary(buf.to_vec().into()))
            .map_err(io::Error::other)?;

        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let me = self.get_mut();
        Pin::new(&mut me.sink)
            .poll_flush(cx)
            .map_err(io::Error::other)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let me = self.get_mut();
        Pin::new(&mut me.sink)
            .poll_close(cx)
            .map_err(io::Error::other)
    }
}
