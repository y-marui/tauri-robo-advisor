use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::time::Duration;

use serde_json::{Value, json};
use tungstenite::protocol::WebSocketConfig;
use tungstenite::{Message, WebSocket};

use crate::links::{self, CLICK_MENU, FIND_LINKS, FIRST_HEADING, NAVIGATE};
use crate::{Endpoint, Error};

/// How long one answer may take, and the largest message accepted.
const REPLY_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_MESSAGE_BYTES: usize = 16 * 1024 * 1024;
const MAX_ERROR_CHARS: usize = 200;

/// The limits on what a page may hand back; the script cuts at the same numbers.
/// The most tables read from one page. A page that has this many may have more, which were
/// left out.
pub const MAX_TABLES: usize = 200;
const MAX_ROWS: usize = 5_000;
const MAX_COLUMNS: usize = 200;
const MAX_CELL_CHARS: usize = 1_000;

/// The one script that is ever run in a page. It returns two groups of tables. The first are
/// the tables that contain no other table. The second are the outer tables: a table that holds
/// tables in its cells, none of which holds a table of its own (a layout that puts a small table
/// in a cell for an amount). A table that holds deeper nesting is page layout and is left out.
/// The tables inside an outer table are still in the first group; which group is used is decided
/// by the table title (see the caller). Both groups together are cut at `MAX_TABLES`; up to 20 of
/// those are kept for the outer tables, so many leaf tables cannot crowd them out.
pub(crate) const READ_TABLES: &str = r#"(() => {
  // Line breaks and no-break spaces inside a cell are kept: they separate its values.
  const clean = (t) => String(t ?? '').split('\n').map((l) => l.replace(/[ \t\r\f\v\u2000-\u200a\u3000]+/g, ' ').trim()).filter((l) => l !== '').join('\n').slice(0, 1000);
  const shown = (e) => {
    if (e.getClientRects().length === 0) return false;
    for (let n = e; n; n = n.parentElement) {
      const s = getComputedStyle(n);
      if (s.visibility === 'hidden' || s.display === 'none' || Number(s.opacity) === 0) return false;
    }
    return true;
  };
  const read = (table) =>
    [...table.rows].filter(shown).slice(0, 5000).map((row) => {
      const cells = [];
      for (const cell of [...row.cells].filter(shown).slice(0, 200)) {
        cells.push(clean(cell.innerText ?? cell.textContent));
        for (let i = 1; i < Math.min(cell.colSpan || 1, 200); i++) cells.push('');
      }
      return cells.slice(0, 200);
    });
  const all = [...document.querySelectorAll('table')].filter(shown);
  const leaf = all.filter((table) => !table.querySelector('table'));
  const outer = all.filter((table) => {
    const inner = [...table.querySelectorAll('table')];
    return inner.length > 0 && inner.every((t) => !t.querySelector('table'));
  });
  // The outer tables are few, so they are taken first and the leaf tables get what is left of the
  // limit: a page with very many leaf tables (one small table per amount) must not leave no room
  // for the outer table that holds them.
  const outerRead = outer.slice(0, 20);
  const first = leaf.slice(0, 200 - outerRead.length);
  return { leaf: first.map(read), outer: outerRead.map(read) };
})()"#;

/// The tables of a page, in two groups (see `READ_TABLES`). Together they hold at most
/// `MAX_TABLES`; a page that reaches the limit may have more, which were left out.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PageTables {
    /// Tables that contain no table.
    pub leaf: Vec<Vec<Vec<String>>>,
    /// Tables that contain tables (one level deep) in their cells.
    pub outer: Vec<Vec<Vec<String>>>,
}

impl PageTables {
    pub fn len(&self) -> usize {
        self.leaf.len() + self.outer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Every table, the leaf ones first.
    pub fn into_all(self) -> Vec<Vec<Vec<String>>> {
        let mut all = self.leaf;
        all.extend(self.outer);
        all
    }
}

/// Checks the shape of what the page returned instead of trusting it.
fn parse_tables(value: &Value) -> Result<PageTables, Error> {
    let group = |name: &str| parse_group(value.get(name).ok_or(Error::Script)?);
    let page = PageTables {
        leaf: group("leaf")?,
        outer: group("outer")?,
    };
    if page.len() > MAX_TABLES {
        return Err(Error::Script);
    }
    Ok(page)
}

fn parse_group(value: &Value) -> Result<Vec<Vec<Vec<String>>>, Error> {
    let tables = value.as_array().ok_or(Error::Script)?;
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
    /// The isolated world the fixed scripts run in: the session it belongs to and its context id.
    /// It is dropped when the browser says the context is gone (a new document replaced it).
    world: Option<(String, i64)>,
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
        Ok(Self {
            socket,
            next_id: 1,
            world: None,
        })
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

    /// Opens `url` in a new tab: the browser is sent to the address. Nothing is clicked, typed,
    /// or submitted, so a login page can be shown and the person logs in by hand.
    pub fn open_tab(&mut self, url: &str) -> Result<(), Error> {
        self.call(None, "Target.createTarget", json!({ "url": url }))?;
        Ok(())
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

    /// Reads the tables of the page (both groups, see `PageTables`) as rows of text (with spans filled in so
    /// that columns line up). The script is fixed and only reads the page; no
    /// caller-provided code is ever run in it, and nothing is clicked or typed.
    pub fn read_tables(&mut self, session: &str) -> Result<PageTables, Error> {
        let value = self.evaluate(session, READ_TABLES)?;
        parse_tables(&value)
    }

    /// Like `read_tables`, but waits (bounded by `max`) for the page to be complete: the tables
    /// are read again until `ready` accepts them and one more read, half a second later, gives
    /// exactly the same tables. This is for a page that fills in after a long silence, which the
    /// settle wait cannot see. When the time is up and the title never showed, the last read is returned as it is
    /// and the caller reports what is missing; a title seen only in the last read, without a
    /// confirming equal read, is a `Timeout`. Only reads; nothing is clicked or typed.
    pub fn read_tables_until(
        &mut self,
        session: &str,
        ready: &dyn Fn(&PageTables) -> bool,
        max: Duration,
    ) -> Result<PageTables, Error> {
        let start = std::time::Instant::now();
        let mut last = self.read_tables(session)?;
        while start.elapsed() < max {
            let was_ready = ready(&last);
            std::thread::sleep(Duration::from_millis(500));
            let now = self.read_tables(session)?;
            if was_ready && now == last {
                return Ok(now);
            }
            last = now;
        }
        // A title that showed up only in the last read was never confirmed by a second, equal
        // read: the table may still be filling in, so it is not handed on as complete.
        if ready(&last) {
            return Err(Error::Timeout);
        }
        Ok(last)
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
        sites: &[String],
    ) -> Result<Followed, Error> {
        links::link_text_allowed(text)?;
        let wanted = serde_json::to_string(text.trim()).map_err(|_| Error::Script)?;
        let found = self.evaluate(session, &format!("({FIND_LINKS})({wanted})"))?;
        let candidates = links::parse_candidates(&found)?;
        if candidates.is_empty() {
            return Err(Error::LinkNotFound);
        }
        let allowed: Vec<_> = candidates.iter().filter(|c| c.allowed(sites)).collect();
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
                let mut places: Vec<crate::LinkChoice> = Vec::new();
                for candidate in &allowed {
                    let reference = candidate.reference();
                    if reference.is_empty() || !places.iter().any(|p| p.reference == reference) {
                        places.push(crate::LinkChoice {
                            reference,
                            label: candidate.label(),
                        });
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
        let origin = serde_json::to_string(&chosen.origin).map_err(|_| Error::Script)?;
        let target = serde_json::to_string(&chosen.target_origin).map_err(|_| Error::Script)?;
        let navigated = self.evaluate(
            session,
            &format!(
                "({NAVIGATE})({wanted}, {}, {expected}, {origin}, {target})",
                chosen.index
            ),
        )?;
        if navigated.as_str() != Some("navigated") {
            return Err(Error::LinkNotFound);
        }
        let followed = Followed {
            reference: chosen.reference(),
        };
        self.wait_navigated(session, mark, &before)?;
        // The page that was reached must be on the site the link went to: a redirect (or a
        // script) may have taken the tab elsewhere.
        let site = self.evaluate(session, "location.origin")?;
        if site.as_str() != Some(chosen.target_origin.as_str()) {
            return Err(Error::WrongPage);
        }
        // The address that was really reached (a redirect may have changed it) is judged by
        // the same rules as a link.
        let reached = self.evaluate(
            session,
            "location.pathname + location.search + location.hash",
        )?;
        match reached.as_str() {
            Some(address) if links::address_allowed(address) => {}
            _ => return Err(Error::WrongPage),
        }
        Ok(followed)
    }

    /// Clicks one entry of the page's menu (`group` is the text of the heading link of its
    /// group, `text` the entry), waits until the page it opens has loaded, and checks that the
    /// first heading of that page is `heading`. This is for the vetted sites only: the page must
    /// be on one of `sites` (the allowed sites that cover the host the recipe started on), the
    /// entry must be a link that the page handles itself, and neither text may look like an order
    /// or a money movement. A click runs the page's own script, which cannot be shown to only
    /// navigate, so the heading is checked afterwards and the batch stops when it differs. The tab
    /// is left where it is (a page that was posted cannot be restored by going back).
    pub fn click_menu_link(
        &mut self,
        session: &str,
        group: &str,
        text: &str,
        heading: &str,
        sites: &[String],
    ) -> Result<(), Error> {
        links::link_text_allowed(group)?;
        links::link_text_allowed(text)?;
        let heading = heading.trim();
        if heading.is_empty()
            || heading.chars().count() > 60
            || heading.chars().any(char::is_control)
        {
            return Err(Error::LinkRefused);
        }
        let origin = self
            .evaluate(session, "location.origin")?
            .as_str()
            .unwrap_or_default()
            .to_owned();
        let within = links::https_host(&origin)
            .is_some_and(|host| sites.iter().any(|site| links::host_within(&host, site)));
        if !within {
            return Err(Error::LinkRefused);
        }
        let mark = navigation_mark();
        let before = self
            .evaluate(
                session,
                &format!("(() => {{ window.__roboAdvisorNav = {mark}; return location.href; }})()"),
            )?
            .as_str()
            .unwrap_or_default()
            .to_owned();
        let group_json = serde_json::to_string(group.trim()).map_err(|_| Error::Script)?;
        let text_json = serde_json::to_string(text.trim()).map_err(|_| Error::Script)?;
        let origin_json = serde_json::to_string(&origin).map_err(|_| Error::Script)?;
        let answer = self.evaluate(
            session,
            &format!("({CLICK_MENU})({group_json}, {text_json}, {origin_json})"),
        )?;
        match answer.as_str() {
            Some("clicked") => {}
            Some("none" | "gone") => return Err(Error::LinkNotFound),
            _ => return Err(Error::LinkRefused),
        }
        self.wait_navigated(session, mark, &before)?;
        let now = self.evaluate(session, "location.origin")?;
        if now.as_str() != Some(origin.as_str()) {
            return Err(Error::WrongPage);
        }
        let reached = self.evaluate(
            session,
            "location.pathname + location.search + location.hash",
        )?;
        match reached.as_str() {
            Some(address) if links::address_allowed(address) => {}
            _ => return Err(Error::WrongPage),
        }
        let first = self.evaluate(session, FIRST_HEADING)?;
        if first.as_str() != Some(heading) {
            return Err(Error::WrongPage);
        }
        Ok(())
    }

    /// Goes back `steps` pages in the tab's history, to the page a recipe started from,
    /// so that a recipe stopped at a link can be run again from its beginning.
    pub fn go_back(&mut self, session: &str, steps: usize, start: &str) -> Result<(), Error> {
        if steps > 10 {
            return Err(Error::WrongPage);
        }
        if steps == 0 {
            return self.expect_href(session, start);
        }
        let mark = navigation_mark();
        let before = self
            .evaluate(
                session,
                &format!("(() => {{ window.__roboAdvisorNav = {mark}; return location.href; }})()"),
            )?
            .as_str()
            .unwrap_or_default()
            .to_owned();
        self.evaluate(
            session,
            &format!("(() => {{ history.go(-{steps}); return true; }})()"),
        )?;
        self.wait_navigated(session, mark, &before)?;
        self.expect_href(session, start)
    }

    /// The address of the page the tab is on.
    pub fn current_href(&mut self, session: &str) -> Result<String, Error> {
        Ok(self
            .evaluate(session, "location.href")?
            .as_str()
            .unwrap_or_default()
            .to_owned())
    }

    /// Fails unless the tab is on exactly this address (a restored start page).
    fn expect_href(&mut self, session: &str, expected: &str) -> Result<(), Error> {
        if self.current_href(session)? == expected {
            Ok(())
        } else {
            Err(Error::WrongPage)
        }
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
                // A page that builds itself after loading (a single-page app) is waited
                // for until its content stops changing, not for a fixed time.
                self.evaluate(session, SETTLE)?;
                return Ok(());
            }
        }
        Err(Error::Timeout)
    }

    /// The context of an isolated world of the page's main frame, made when needed. The fixed
    /// scripts run there: it sees the same document, but not the page's own scripts or what they
    /// changed, so a page cannot replace `URL`, `querySelectorAll`, `getComputedStyle`, or the like
    /// before the app reads it. A new document gets a new world.
    fn world_for(&mut self, session: &str) -> Result<i64, Error> {
        if let Some((owner, id)) = &self.world
            && owner == session
        {
            return Ok(*id);
        }
        let tree = self.call(Some(session), "Page.getFrameTree", json!({}))?;
        let frame = tree
            .get("frameTree")
            .and_then(|t| t.get("frame"))
            .and_then(|f| f.get("id"))
            .and_then(Value::as_str)
            .ok_or(Error::Script)?
            .to_owned();
        let created = self.call(
            Some(session),
            "Page.createIsolatedWorld",
            json!({ "frameId": frame, "worldName": "robo-advisor" }),
        )?;
        let id = created
            .get("executionContextId")
            .and_then(Value::as_i64)
            .ok_or(Error::Script)?;
        self.world = Some((session.to_owned(), id));
        Ok(id)
    }

    fn evaluate(&mut self, session: &str, expression: &str) -> Result<Value, Error> {
        // A context that was destroyed by a navigation is made again, once. A mark left in the
        // old world is gone with it, which is what a new document means.
        let result = match self.evaluate_in_world(session, expression) {
            Err(Error::Protocol(message)) if message.contains("context") => {
                self.world = None;
                self.evaluate_in_world(session, expression)?
            }
            other => other?,
        };
        if result.get("exceptionDetails").is_some() {
            return Err(Error::Script);
        }
        Ok(result
            .get("result")
            .and_then(|r| r.get("value"))
            .cloned()
            .unwrap_or(Value::Null))
    }

    fn evaluate_in_world(&mut self, session: &str, expression: &str) -> Result<Value, Error> {
        let context = self.world_for(session)?;
        self.call(
            Some(session),
            "Runtime.evaluate",
            json!({
                "expression": expression,
                "contextId": context,
                "returnByValue": true,
                "awaitPromise": true
            }),
        )
    }

    /// Asks the browser to quit, so no extra instance is left open.
    pub fn close(mut self) -> Result<(), Error> {
        let sent = self.call(None, "Browser.close", json!({}));
        let _ = self.socket.close(None);
        sent.map(|_| ())
    }
}

/// Resolves once the page has stopped changing for a moment (bounded to 8 seconds), so
/// that the content of a page that renders after loading is there before it is read.
const SETTLE: &str = "new Promise(resolve => { \
    const done = () => { observer.disconnect(); resolve(true); }; \
    let timer = setTimeout(done, 1500); \
    const observer = new MutationObserver(() => { clearTimeout(timer); timer = setTimeout(done, 1500); }); \
    observer.observe(document, { subtree: true, childList: true, characterData: true, attributes: true }); \
    setTimeout(done, 8000); \
})";

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
