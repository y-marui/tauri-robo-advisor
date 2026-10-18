use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::time::Duration;

use serde_json::{Value, json};
use tungstenite::protocol::WebSocketConfig;
use tungstenite::{Message, WebSocket};

use crate::links::{self, FIND_LINKS, NAVIGATE};
use crate::{Endpoint, Error};

/// How long one answer may take, and the largest message accepted.
const REPLY_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_MESSAGE_BYTES: usize = 16 * 1024 * 1024;
const MAX_ERROR_CHARS: usize = 200;

/// The limits on what a page may hand back; the script cuts at the same numbers.
const MAX_TABLES: usize = 200;
const MAX_ROWS: usize = 5_000;
const MAX_COLUMNS: usize = 200;
const MAX_CELL_CHARS: usize = 1_000;

/// The one script that is ever run in a page. Tables that contain other tables
/// are layout, not data, and are left out.
pub(crate) const READ_TABLES: &str = r#"(() => {
  const clean = (t) => String(t ?? '').replace(/\s+/g, ' ').trim().slice(0, 1000);
  return [...document.querySelectorAll('table')]
    .filter((table) => !table.querySelector('table'))
    .slice(0, 200)
    .map((table) =>
      [...table.rows].slice(0, 5000).map((row) => {
        const cells = [];
        for (const cell of [...row.cells].slice(0, 200)) {
          cells.push(clean(cell.innerText ?? cell.textContent));
          for (let i = 1; i < Math.min(cell.colSpan || 1, 200); i++) cells.push('');
        }
        return cells.slice(0, 200);
      }));
})()"#;

/// Checks the shape of what the page returned instead of trusting it.
fn parse_tables(value: &Value) -> Result<Vec<Vec<Vec<String>>>, Error> {
    let tables = value.as_array().ok_or(Error::Script)?;
    if tables.len() > MAX_TABLES {
        return Err(Error::Script);
    }
    tables
        .iter()
        .map(|table| {
            let rows = table.as_array().ok_or(Error::Script)?;
            if rows.len() > MAX_ROWS {
                return Err(Error::Script);
            }
            rows.iter()
                .map(|row| {
                    let cells = row.as_array().ok_or(Error::Script)?;
                    if cells.len() > MAX_COLUMNS {
                        return Err(Error::Script);
                    }
                    cells
                        .iter()
                        .map(|cell| {
                            let text = cell.as_str().ok_or(Error::Script)?;
                            if text.chars().count() > MAX_CELL_CHARS {
                                return Err(Error::Script);
                            }
                            Ok(text.to_owned())
                        })
                        .collect()
                })
                .collect()
        })
        .collect()
}

/// The link that was followed, described by where it went.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Followed {
    /// Save this next to the link text so the same link is found next time.
    pub reference: String,
}

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

    /// Reads every table of the page as rows of text (with spans filled in so
    /// that columns line up). The script is fixed and only reads the page; no
    /// caller-provided code is ever run in it, and nothing is clicked or typed.
    pub fn read_tables(&mut self, session: &str) -> Result<Vec<Vec<Vec<String>>>, Error> {
        let value = self.evaluate(session, READ_TABLES)?;
        parse_tables(&value)
    }

    /// Follows the link with exactly this visible text, to show another page,
    /// and waits until the page has loaded. Only a plain `<a>` link with a real
    /// address on the page's own site is followed, and not one that looks like an
    /// order or a money movement. The browser is sent to the checked address
    /// directly; the link is not clicked, so no script of the page runs.
    /// Nothing is typed or submitted.
    ///
    /// Links with the same text that go to the same place count as one. When they
    /// go to different places, nothing is clicked unless a `reference` (from an
    /// earlier run) says which one; a reference that matches nothing means the
    /// page has changed, and nothing is clicked either.
    pub fn follow_link(
        &mut self,
        session: &str,
        text: &str,
        reference: Option<&str>,
    ) -> Result<Followed, Error> {
        links::link_text_allowed(text)?;
        let wanted = serde_json::to_string(text.trim()).map_err(|_| Error::Script)?;
        let found = self.evaluate(session, &format!("({FIND_LINKS})({wanted})"))?;
        let candidates = links::parse_candidates(&found)?;
        if candidates.is_empty() {
            return Err(Error::LinkNotFound);
        }
        let allowed: Vec<_> = candidates.iter().filter(|c| c.allowed()).collect();
        if allowed.is_empty() {
            return Err(Error::LinkRefused);
        }
        let chosen = match reference {
            Some(reference) => allowed
                .iter()
                .find(|c| c.reference() == reference)
                .ok_or(Error::LinkChanged)?,
            None => {
                // Different destinations are ambiguous; scripted links cannot be
                // told apart by where they go, so several of them are ambiguous too.
                let mut places: Vec<String> = Vec::new();
                for candidate in &allowed {
                    let place = candidate.reference();
                    if place.is_empty() || !places.contains(&place) {
                        places.push(place);
                    }
                }
                if places.len() > 1 {
                    return Err(Error::LinkAmbiguous(places));
                }
                &allowed[0]
            }
        };
        // A mark on the current document: it is gone once the new document has replaced it.
        let mark = navigation_mark();
        let before = self
            .evaluate(
                session,
                &format!("(() => {{ window.__roboAdvisorNav = {mark}; return location.href; }})()"),
            )?
            .as_str()
            .unwrap_or_default()
            .to_owned();
        let expected = serde_json::to_string(&chosen.href).map_err(|_| Error::Script)?;
        let navigated = self.evaluate(
            session,
            &format!("({NAVIGATE})({wanted}, {}, {expected})", chosen.index),
        )?;
        if navigated.as_str() != Some("navigated") {
            return Err(Error::LinkNotFound);
        }
        let followed = Followed {
            reference: chosen.reference(),
        };
        self.wait_navigated(session, mark, &before)?;
        Ok(followed)
    }

    /// Waits (bounded) until the page that was asked for has really replaced the old
    /// one and has loaded. The old document still reports "complete" for a while after
    /// the navigation starts, so readiness alone proves nothing: the mark left on the old
    /// document must be gone (a new document), or, for a page that changes its address
    /// without loading a new document, the address must have changed.
    fn wait_navigated(&mut self, session: &str, mark: u64, before: &str) -> Result<(), Error> {
        let probe = format!(
            "({{ marker: window.__roboAdvisorNav === {mark}, ready: document.readyState, href: location.href }})"
        );
        for _ in 0..60 {
            std::thread::sleep(Duration::from_millis(300));
            let state = self.evaluate(session, &probe)?;
            let marker = state.get("marker").and_then(Value::as_bool).unwrap_or(true);
            let ready = state.get("ready").and_then(Value::as_str) == Some("complete");
            let href = state
                .get("href")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if ready && (!marker || href != before) {
                // A page that builds itself after loading gets a moment to settle.
                if marker {
                    std::thread::sleep(Duration::from_millis(700));
                }
                return Ok(());
            }
        }
        Err(Error::Timeout)
    }

    fn evaluate(&mut self, session: &str, expression: &str) -> Result<Value, Error> {
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

/// A number that differs from visit to visit, to tell one document from the next.
fn navigation_mark() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(1, |d| d.as_nanos() as u64 % 1_000_000_000_000)
}

fn connection(error: tungstenite::Error) -> Error {
    use std::io::ErrorKind::{TimedOut, WouldBlock};
    match &error {
        tungstenite::Error::Io(io) if matches!(io.kind(), WouldBlock | TimedOut) => Error::Timeout,
        _ => Error::Connection(error.to_string()),
    }
}
