mod connection_alive;
mod connection_lost;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{Value, json};
use tungstenite::{Message, WebSocket};

use super::WsConn;
use super::daemon::handshake;
use super::jsonrpc::RpcClient;

#[cfg(windows)]
use uds_windows::{UnixListener, UnixStream};
#[cfg(not(windows))]
use std::os::unix::net::{UnixListener, UnixStream};

pub(super) const TEST_CALL_TIMEOUT: Duration = Duration::from_millis(300);

#[derive(Clone, Copy)]
pub(super) enum AfterInitialize {
    Answer,
    AskThenAnswer,
    NotifyTurnStarted,
    Silent,
    DropOnFirstRequest,
    GarbageOnFirstRequest,
}

pub(super) struct FakeDaemon {
    pub path: String,
    pub received: Arc<Mutex<Vec<Value>>>,
    _dir: tempfile::TempDir,
}

fn reply(ws: &mut WebSocket<UnixStream>, frame: Value) {
    let _ = ws.send(Message::text(frame.to_string()));
}

impl FakeDaemon {
    pub fn start(after: AfterInitialize) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("daemon.sock").to_string_lossy().into_owned();
        let listener = UnixListener::bind(&path).unwrap();
        let received = Arc::new(Mutex::new(Vec::new()));
        let log = received.clone();
        std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut ws = tungstenite::accept(stream).unwrap();
            let mut initialized = false;
            while let Ok(Message::Text(text)) = ws.read() {
                let frame: Value = serde_json::from_str(text.as_str()).unwrap();
                log.lock().unwrap().push(frame.clone());
                let (Some(id), Some(method)) = (frame.get("id").cloned(), frame.get("method").and_then(Value::as_str)) else { continue };
                if !initialized {
                    initialized = true;
                    reply(&mut ws, json!({"id": id, "result": {"userAgent": "fake"}}));
                    continue;
                }
                match after {
                    AfterInitialize::Answer if method == "fail" => {
                        reply(&mut ws, json!({"id": id, "error": {"code": -32600, "message": "no good"}}));
                    }
                    AfterInitialize::Answer => reply(&mut ws, json!({"id": id, "result": {"echo": method}})),
                    AfterInitialize::AskThenAnswer => {
                        reply(&mut ws, json!({"id": 99, "method": "item/commandExecution/requestApproval", "params": {}}));
                        reply(&mut ws, json!({"id": id, "result": {"echo": method}}));
                    }
                    AfterInitialize::NotifyTurnStarted => {
                        reply(&mut ws, json!({"method": "turn/started", "params": {"threadId": "t-8", "turn": {}}}));
                        reply(&mut ws, json!({"method": "turn/started", "params": {"threadId": "t-9", "turn": {}}}));
                        reply(&mut ws, json!({"id": id, "result": {"echo": method}}));
                    }
                    AfterInitialize::Silent => {}
                    AfterInitialize::DropOnFirstRequest => return,
                    AfterInitialize::GarbageOnFirstRequest => {
                        let _ = ws.send(Message::text("not json"));
                    }
                }
            }
        });
        Self { path, received, _dir: dir }
    }

    pub async fn connect(&self) -> Arc<RpcClient> {
        let conn = WsConn::dial(self.path.clone()).await.unwrap();
        handshake(conn, "test", TEST_CALL_TIMEOUT).await.unwrap()
    }
}
