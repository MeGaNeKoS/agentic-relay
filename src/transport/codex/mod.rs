mod api;
mod client;
mod daemon;
pub mod errors;
pub mod jsonrpc;
mod sync_socket;

pub use api::{CodexApi, TurnStartedWaiter};
pub use client::CodexClient;
pub use jsonrpc::CallError;

use std::io::{Read, Write};

use anyhow::{anyhow, Context, Result};
use tokio::sync::mpsc;
use tungstenite::protocol::Role;
use tungstenite::{Message, WebSocket};

use sync_socket::SyncDuplex;

pub struct WsConn {
    pub read: WsReadHalf,
    pub write: WsWriteHalf,
}

pub struct WsWriteHalf {
    ws: WebSocket<ChannelWriter>,
}

pub struct WsReadHalf {
    rx: mpsc::UnboundedReceiver<std::io::Result<Vec<u8>>>,
}

struct ChannelWriter {
    tx: mpsc::UnboundedSender<Vec<u8>>,
}

impl Write for ChannelWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.tx
            .send(buf.to_vec())
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::BrokenPipe, "socket writer thread stopped"))?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Read for ChannelWriter {
    fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other("write-only stream"))
    }
}

struct ReaderStream {
    read: Box<dyn SyncDuplex>,
    write: ChannelWriter,
}

impl Read for ReaderStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.read.read(buf)
    }
}

impl Write for ReaderStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.write.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.write.flush()
    }
}

impl WsConn {
    pub async fn dial(socket_path: String) -> Result<Self> {
        let sock = tokio::task::spawn_blocking({
            let socket_path = socket_path.clone();
            move || sync_socket::connect(&socket_path).with_context(|| format!("dial {socket_path}"))
        })
        .await??;

        let (write_tx, write_rx) = mpsc::unbounded_channel::<Vec<u8>>();
        let (read_tx, read_rx) = mpsc::unbounded_channel::<std::io::Result<Vec<u8>>>();

        let mut writer_sock = sock.try_clone()?;
        std::thread::spawn(move || {
            let mut write_rx = write_rx;
            while let Some(frame) = write_rx.blocking_recv() {
                if writer_sock.write_all(&frame).is_err() {
                    break;
                }
            }
        });

        let handshake_tx = write_tx.clone();
        let reader_ws = tokio::task::spawn_blocking(move || -> Result<WebSocket<ReaderStream>> {
            let stream = ReaderStream { read: sock, write: ChannelWriter { tx: handshake_tx } };
            let (ws, _response) =
                tungstenite::client("ws://localhost/", stream).map_err(|e| anyhow!("websocket handshake: {e}"))?;
            Ok(ws)
        })
        .await??;

        std::thread::spawn(move || {
            let mut ws = reader_ws;
            loop {
                let outcome = match ws.read() {
                    Ok(Message::Text(text)) => Ok(text.as_bytes().to_vec()),
                    Ok(Message::Binary(data)) => Ok(data.to_vec()),
                    Ok(Message::Ping(_) | Message::Pong(_)) => continue,
                    Ok(Message::Close(_)) => {
                        Err(std::io::Error::new(std::io::ErrorKind::ConnectionReset, "connection closed"))
                    }
                    Ok(Message::Frame(_)) => unreachable!("read() never returns a raw Frame"),
                    Err(e) => Err(std::io::Error::other(e.to_string())),
                };
                let stop = outcome.is_err();
                if read_tx.send(outcome).is_err() || stop {
                    break;
                }
            }
        });

        let write_ws = WebSocket::from_raw_socket(ChannelWriter { tx: write_tx }, Role::Client, None);

        Ok(Self { read: WsReadHalf { rx: read_rx }, write: WsWriteHalf { ws: write_ws } })
    }
}

impl WsWriteHalf {
    pub fn write_frame(&mut self, payload: &[u8]) -> Result<()> {
        let text = String::from_utf8(payload.to_vec()).context("websocket frame is not valid UTF-8")?;
        self.ws.send(Message::text(text)).map_err(|e| anyhow!("websocket write: {e}"))
    }
}

impl WsReadHalf {
    pub async fn read_frame(&mut self) -> Result<Vec<u8>> {
        match self.rx.recv().await {
            Some(Ok(payload)) => Ok(payload),
            Some(Err(e)) => Err(e.into()),
            None => anyhow::bail!("connection closed"),
        }
    }
}

#[cfg(test)]
#[path = "../../tests/transport/codex/mod.rs"]
mod tests;
