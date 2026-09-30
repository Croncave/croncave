//! A WebSocket seen as a stream of bytes, so a multiplexer can run over it.
//!
//! The relay's half of this is the same idea against axum's socket type.

use std::io;
use std::pin::Pin;
use std::task::{Context, Poll, ready};

use futures::sink::Sink;
use futures::stream::{SplitSink, SplitStream, Stream, StreamExt as _};
use futures::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// A WebSocket, as bytes.
pub struct Bytes {
    sink: SplitSink<Socket, Message>,
    stream: SplitStream<Socket>,
    pending: Vec<u8>,
    read: usize,
}

impl Bytes {
    pub fn new(socket: Socket) -> Self {
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
                    if me.pending.is_empty() {
                        continue;
                    }
                }
                Some(Ok(_)) => continue,
                Some(Err(error)) => return Poll::Ready(Err(io::Error::other(error))),
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
        Pin::new(&mut self.get_mut().sink)
            .poll_flush(cx)
            .map_err(io::Error::other)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().sink)
            .poll_close(cx)
            .map_err(io::Error::other)
    }
}
