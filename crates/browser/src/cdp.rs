use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::time::Duration;

use serde_json::{Value, json};
use tungstenite::protocol::WebSocketConfig;
use tungstenite::{Message, WebSocket};

use crate::{Endpoint, Error};

/// How long one answer may take, and the largest message accepted.
const REPLY_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_MESSAGE_BYTES: usize = 16 * 1024 * 1024;
const MAX_ERROR_CHARS: usize = 200;

/// A page the browser has open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    pub target_id: String,
    pub url: String,
    pub title: String,
}

/// A connection to the browser's DevTools protocol on this computer. Every
/// request goes over one WebSocket; pages are reached by attaching to them, so
/// no HTTP request and no driver program is needed.
pub struct Cdp {
    socket: WebSocket<TcpStream>,
    next_id: u64,
}

impl Cdp {
    pub fn connect(endpoint: &Endpoint) -> Result<Self, Error> {
        let address = SocketAddr::from((Ipv4Addr::LOCALHOST, endpoint.port));
        let stream = TcpStream::connect_timeout(&address, Duration::from_secs(5))
            .map_err(|e| Error::Connection(e.kind().to_string()))?;
        stream
            .set_read_timeout(Some(REPLY_TIMEOUT))
            .and_then(|()| stream.set_write_timeout(Some(REPLY_TIMEOUT)))
            .map_err(|e| Error::Connection(e.kind().to_string()))?;
        let config = WebSocketConfig::default()
            .max_message_size(Some(MAX_MESSAGE_BYTES))
            .max_frame_size(Some(MAX_MESSAGE_BYTES));
        let (socket, _) =
            tungstenite::client::client_with_config(endpoint.url(), stream, Some(config))
                .map_err(|e| Error::Connection(e.to_string()))?;
        Ok(Self { socket, next_id: 1 })
    }

    /// Sends one request and waits for its answer. Events the browser sends in
    /// between are ignored.
    pub fn call(
        &mut self,
        session: Option<&str>,
        method: &str,
        params: Value,
    ) -> Result<Value, Error> {
        let id = self.next_id;
        self.next_id += 1;
        let mut request = json!({ "id": id, "method": method, "params": params });
        if let Some(session) = session {
            request["sessionId"] = json!(session);
        }
        self.socket
            .send(Message::text(request.to_string()))
            .map_err(connection)?;
        loop {
            let message = self.socket.read().map_err(connection)?;
            let Message::Text(text) = message else {
                continue;
            };
            let Ok(reply) = serde_json::from_str::<Value>(text.as_str()) else {
                continue;
            };
            if reply.get("id").and_then(Value::as_u64) != Some(id) {
                continue;
            }
            if let Some(error) = reply.get("error") {
                let message = error
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .chars()
                    .filter(|c| !c.is_control())
                    .take(MAX_ERROR_CHARS)
                    .collect();
                return Err(Error::Protocol(message));
            }
            return Ok(reply.get("result").cloned().unwrap_or(Value::Null));
        }
    }

    /// The tabs that are open (not workers, extensions' background pages, or the like).
    pub fn pages(&mut self) -> Result<Vec<Page>, Error> {
        let result = self.call(None, "Target.getTargets", json!({}))?;
        let infos = result
            .get("targetInfos")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        Ok(infos
            .iter()
            .filter(|info| info.get("type").and_then(Value::as_str) == Some("page"))
            .filter_map(|info| {
                Some(Page {
                    target_id: info.get("targetId")?.as_str()?.to_owned(),
                    url: info.get("url")?.as_str()?.to_owned(),
                    title: info
                        .get("title")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_owned(),
                })
            })
            .collect())
    }

    /// Attaches to a page and returns the session to address it with.
    pub fn attach(&mut self, page: &Page) -> Result<String, Error> {
        let result = self.call(
            None,
            "Target.attachToTarget",
            json!({ "targetId": page.target_id, "flatten": true }),
        )?;
        result
            .get("sessionId")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or(Error::NoPage)
    }

    /// Runs a fixed script in the page and returns its value. The script only
    /// reads; nothing the page contains is ever run as code.
    pub fn evaluate(&mut self, session: &str, expression: &str) -> Result<Value, Error> {
        let result = self.call(
            Some(session),
            "Runtime.evaluate",
            json!({ "expression": expression, "returnByValue": true, "awaitPromise": true }),
        )?;
        if result.get("exceptionDetails").is_some() {
            return Err(Error::Script);
        }
        Ok(result
            .get("result")
            .and_then(|r| r.get("value"))
            .cloned()
            .unwrap_or(Value::Null))
    }

    /// Asks the browser to quit, so no extra instance is left open.
    pub fn close(mut self) -> Result<(), Error> {
        let sent = self.call(None, "Browser.close", json!({}));
        let _ = self.socket.close(None);
        sent.map(|_| ())
    }
}

fn connection(error: tungstenite::Error) -> Error {
    use std::io::ErrorKind::{TimedOut, WouldBlock};
    match &error {
        tungstenite::Error::Io(io) if matches!(io.kind(), WouldBlock | TimedOut) => Error::Timeout,
        _ => Error::Connection(error.to_string()),
    }
}
