// Every address, page, and value in these tests is made up. The browser is a
// small fake DevTools server on this computer.

use std::collections::HashMap;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::thread;

use serde_json::{Value, json};
use tungstenite::Message;

use crate::*;

// ---- finding browsers -------------------------------------------------------

fn env_of(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
    let map: HashMap<String, String> = pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect();
    move |name| map.get(name).cloned()
}

#[test]
fn mac_browsers_are_found_in_the_applications_folders() {
    let env = env_of(&[("HOME", "/data/user-a")]);
    let installed = [
        "/Applications/Brave Browser.app/Contents/MacOS/Brave Browser",
        "/data/user-a/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
    ];

    let found = find_with(Os::Mac, &env, &|p| {
        installed.iter().any(|i| Path::new(i) == p)
    });

    let kinds: Vec<_> = found.iter().map(|f| f.kind).collect();
    assert_eq!(kinds, vec![Kind::Chrome, Kind::Brave]);
}

#[test]
fn windows_browsers_are_found_under_the_program_folders() {
    let env = env_of(&[
        ("PROGRAMFILES", r"C:\Program Files"),
        ("LOCALAPPDATA", r"D:\LocalApps"),
    ]);
    let edge = PathBuf::from(r"C:\Program Files").join(r"Microsoft\Edge\Application\msedge.exe");

    let found = find_with(Os::Windows, &env, &|p| p == edge);

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].kind, Kind::Edge);
}

#[test]
fn linux_browsers_are_found_on_the_path_and_each_kind_once() {
    let env = env_of(&[("PATH", "/usr/local/bin:/usr/bin")]);
    let installed = [
        "/usr/bin/brave-browser",
        "/usr/local/bin/brave-browser",
        "/usr/bin/google-chrome",
    ];

    let found = find_with(Os::Linux, &env, &|p| {
        installed.iter().any(|i| Path::new(i) == p)
    });

    assert_eq!(found.len(), 2);
    assert!(
        found
            .iter()
            .any(|f| f.kind == Kind::Brave && f.path == Path::new("/usr/local/bin/brave-browser"))
    );
}

#[test]
fn nothing_is_found_when_nothing_is_installed() {
    assert!(find_with(Os::Mac, &env_of(&[]), &|_| false).is_empty());
}

// ---- the debugging port file -------------------------------------------------

#[test]
fn the_active_port_file_is_read() {
    let endpoint = parse_active_port("9222\n/devtools/browser/abc-123\n").unwrap();

    assert_eq!(endpoint.port, 9222);
    assert_eq!(
        endpoint.url(),
        "ws://127.0.0.1:9222/devtools/browser/abc-123"
    );
}

#[test]
fn a_damaged_or_foreign_port_file_is_refused() {
    for bad in [
        "",
        "9222",
        "0\n/devtools/browser/abc",
        "99999\n/devtools/browser/abc",
        "9222\n/json/list",
        "9222\n/devtools/browser/",
        "9222\n/devtools/browser/../../etc",
        "9222\n/devtools/browser/a b",
        "abc\n/devtools/browser/x",
    ] {
        assert_eq!(parse_active_port(bad), None, "{bad:?}");
    }
}

// ---- the protocol, against a fake browser -------------------------------------

/// Serves one connection like a browser would: it answers the few requests the
/// client makes, and sends an unrelated event before the first answer.
fn fake_browser(script_value: Value) -> Endpoint {
    fake_browser_with(script_value, false)
}

fn fake_browser_with(script_value: Value, fail_script: bool) -> Endpoint {
    fake_browser_queue(vec![script_value], fail_script)
}

/// Like `fake_browser`, but the page answers successive scripts with these values
/// in order (the last one is repeated), and the scripts it was asked to run are
/// kept for the test to look at.
fn fake_browser_queue(values: Vec<Value>, fail_script: bool) -> Endpoint {
    fake_browser_logged(values, fail_script, None)
}

fn fake_browser_logged(
    values: Vec<Value>,
    fail_script: bool,
    log: Option<std::sync::Arc<std::sync::Mutex<Vec<String>>>>,
) -> Endpoint {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut socket = tungstenite::accept(stream).unwrap();
        let mut first = true;
        let mut next = 0usize;
        let mut world = 0i64;
        while let Ok(Message::Text(text)) = socket.read() {
            let request: Value = serde_json::from_str(text.as_str()).unwrap();
            let id = request["id"].clone();
            if first {
                first = false;
                let event = json!({"method": "Target.targetCreated", "params": {}});
                socket.send(Message::text(event.to_string())).unwrap();
            }
            let result = match request["method"].as_str().unwrap() {
                "Target.getTargets" => json!({"targetInfos": [
                    {"targetId": "w1", "type": "service_worker", "url": "https://example.test/sw.js", "title": ""},
                    {"targetId": "p1", "type": "page", "url": "https://example.test/holdings", "title": "Holdings"},
                ]}),
                "Target.attachToTarget" => {
                    assert_eq!(request["params"]["flatten"], true);
                    json!({"sessionId": "s1"})
                }
                "Page.getFrameTree" => {
                    assert_eq!(request["sessionId"], "s1");
                    json!({"frameTree": {"frame": {"id": "frame1"}}})
                }
                "Page.createIsolatedWorld" => {
                    assert_eq!(request["sessionId"], "s1");
                    assert_eq!(request["params"]["frameId"], "frame1");
                    world += 1;
                    // Only the test that looks at the worlds records them.
                    if let Some(log) = log
                        .as_ref()
                        .filter(|_| values.contains(&json!("__context_lost__")))
                    {
                        log.lock().unwrap().push(format!("world {world}"));
                    }
                    json!({"executionContextId": world})
                }
                "Runtime.evaluate" => {
                    assert_eq!(request["sessionId"], "s1");
                    // Every script runs in the isolated world, never in the page's own.
                    assert_eq!(request["params"]["contextId"], world);
                    if let Some(log) = &log {
                        let expression = request["params"]["expression"].as_str().unwrap_or("");
                        log.lock().unwrap().push(expression.to_owned());
                    }
                    if values
                        .get(next)
                        .is_some_and(|v| v == &json!("__context_lost__"))
                    {
                        next += 1;
                        let error = json!({"id": id, "error": {"code": -32000, "message": "Cannot find context with specified id"}});
                        socket.send(Message::text(error.to_string())).unwrap();
                        continue;
                    }
                    if fail_script {
                        json!({"exceptionDetails": {"text": "secret page text"}})
                    } else {
                        let value = values[next.min(values.len() - 1)].clone();
                        next += 1;
                        json!({"result": {"type": "object", "value": value}})
                    }
                }
                "Browser.close" => json!({}),
                "Target.createTarget" => {
                    if let Some(log) = &log {
                        let url = request["params"]["url"].as_str().unwrap_or("");
                        log.lock().unwrap().push(format!("createTarget {url}"));
                    }
                    json!({"targetId": "new1"})
                }
                _ => {
                    let error =
                        json!({"id": id, "error": {"code": -32601, "message": "no\nsuch method"}});
                    socket.send(Message::text(error.to_string())).unwrap();
                    continue;
                }
            };
            let reply = json!({"id": id, "result": result});
            socket.send(Message::text(reply.to_string())).unwrap();
        }
    });
    Endpoint {
        port,
        path: "/devtools/browser/test".to_owned(),
    }
}

#[test]
fn pages_are_listed_attached_read_and_closed() {
    let endpoint = fake_browser(json!({
        "leaf": [[["a", "1"], ["b", "2"]]],
        "outer": [[["c", "3"]]],
    }));
    let mut cdp = Cdp::connect(&endpoint).unwrap();

    let pages = cdp.pages().unwrap();
    assert_eq!(pages.len(), 1, "workers are not pages");
    assert_eq!(pages[0].url, "https://example.test/holdings");

    let session = cdp.attach(&pages[0]).unwrap();
    let tables = cdp.read_tables(&session).unwrap();
    assert_eq!(tables.leaf, vec![vec![vec!["a", "1"], vec!["b", "2"]]]);
    assert_eq!(tables.outer, vec![vec![vec!["c", "3"]]]);
    assert_eq!(tables.len(), 2);

    cdp.close().unwrap();
}

#[test]
fn a_script_error_does_not_repeat_the_page_text() {
    let endpoint = fake_browser_with(Value::Null, true);
    let mut cdp = Cdp::connect(&endpoint).unwrap();
    let pages = cdp.pages().unwrap();
    let session = cdp.attach(&pages[0]).unwrap();

    let error = cdp.read_tables(&session).unwrap_err();

    assert_eq!(error, Error::Script);
    assert!(!error.to_string().contains("secret"));
}

#[test]
fn a_browser_error_is_reported_without_control_characters() {
    let endpoint = fake_browser(Value::Null);
    let mut cdp = Cdp::connect(&endpoint).unwrap();

    let error = cdp.call(None, "Nope.nothing", json!({})).unwrap_err();

    assert_eq!(error, Error::Protocol("nosuch method".to_owned()));
}

#[test]
fn connecting_to_a_closed_port_fails_cleanly() {
    let port = {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap().port()
    };
    let endpoint = Endpoint {
        port,
        path: "/devtools/browser/x".to_owned(),
    };

    assert!(matches!(Cdp::connect(&endpoint), Err(Error::Connection(_))));
}

#[test]
fn a_page_answer_of_the_wrong_shape_is_refused() {
    for wrong in [
        json!("text"),
        json!([["a"]]),
        json!([[[1, 2]]]),
        json!({"leaf": [[[1, 2]]], "outer": []}),
        json!({"leaf": [[["x".repeat(2000)]]], "outer": []}),
        json!({"leaf": []}),
        // Both groups together are held to the limit.
        json!({
            "leaf": vec![json!([["a"]]); MAX_TABLES],
            "outer": [[["a"]]],
        }),
        Value::Null,
    ] {
        let endpoint = fake_browser(wrong.clone());
        let mut cdp = Cdp::connect(&endpoint).unwrap();
        let pages = cdp.pages().unwrap();
        let session = cdp.attach(&pages[0]).unwrap();

        assert_eq!(cdp.read_tables(&session), Err(Error::Script), "{wrong}");
    }
}

#[test]
fn the_fixed_script_only_reads_the_page() {
    // A guard against someone adding an action to the one script that runs.
    let script = crate::cdp_script_for_tests();

    for action in [
        "click(",
        "submit(",
        ".value =",
        "fetch(",
        "XMLHttpRequest",
        "eval(",
        "location",
    ] {
        assert!(!script.contains(action), "{action}");
    }
}

// ---- following a link --------------------------------------------------------

fn attached(endpoint: &Endpoint) -> (Cdp, String) {
    let mut cdp = Cdp::connect(endpoint).unwrap();
    let pages = cdp.pages().unwrap();
    let session = cdp.attach(&pages[0]).unwrap();
    (cdp, session)
}

/// What the page answers to the readiness probe: whether the old document's mark is still there,
/// and its state.
fn probe(marker: bool, ready: &str) -> Value {
    json!({"marker": marker, "ready": ready, "href": "https://h.example/old"})
}

fn found(href: &str, same_site: bool, scripted: bool) -> Value {
    json!([{"index": 4, "href": href, "same_site": same_site, "scripted": scripted, "origin": "https://h.example", "target_origin": "https://h.example"}])
}

#[test]
fn a_plain_link_is_found_clicked_and_waited_for() {
    let log = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let endpoint = fake_browser_logged(
        vec![
            found("/account/list", true, false),
            json!("https://h.example/old"),
            json!("navigated"),
            probe(true, "loading"),
            probe(false, "complete"),
            json!(true),
            json!("https://h.example"),
        ],
        false,
        Some(log.clone()),
    );
    let (mut cdp, session) = attached(&endpoint);

    let followed = cdp.follow_link(&session, "口座(円建)", None, &[]).unwrap();
    assert_eq!(followed.reference, crate::fingerprint("/account/list"));

    let scripts = log.lock().unwrap().clone();
    assert_eq!(
        scripts.len(),
        8,
        "find, mark, go, two checks, the settle wait, the site and address checks"
    );
    assert!(scripts[5].contains("MutationObserver"));
    assert!(
        scripts[0].contains("\"口座(円建)\""),
        "the text is passed as JSON data"
    );
    assert!(
        scripts[1].contains("__roboAdvisorNav"),
        "the old document is marked first"
    );
    assert!(
        scripts[2]
            .contains(", 4, \"/account/list\", \"https://h.example\", \"https://h.example\")"),
        "the position, the accepted address and the site come from the search"
    );
}

#[test]
fn a_link_text_is_passed_as_data_not_as_code() {
    let log = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let endpoint = fake_browser_logged(vec![json!([])], false, Some(log.clone()));
    let (mut cdp, session) = attached(&endpoint);

    let result = cdp.follow_link(&session, "x\"); alert(1); (\"", None, &[]);

    assert_eq!(result, Err(Error::LinkNotFound));
    let scripts = log.lock().unwrap().clone();
    assert!(
        scripts[0].contains("x\\\"); alert(1); (\\\""),
        "quotes are escaped: {}",
        scripts[0]
    );
}

#[test]
fn risky_links_are_refused_before_the_page_is_asked() {
    let log = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let endpoint = fake_browser_logged(vec![json!([])], false, Some(log.clone()));
    let (mut cdp, session) = attached(&endpoint);

    assert_eq!(
        cdp.follow_link(&session, "買い注文", None, &[]),
        Err(Error::LinkRefused)
    );
    assert_eq!(
        cdp.follow_link(&session, "ログアウト", None, &[]),
        Err(Error::LinkRefused)
    );
    assert!(log.lock().unwrap().is_empty());
}

#[test]
fn a_link_to_another_site_or_to_an_action_is_refused_and_nothing_is_clicked() {
    for page in [
        found("/x", false, false),
        found("/trade/order/confirm", true, false),
    ] {
        let log = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let endpoint = fake_browser_logged(vec![page], false, Some(log.clone()));
        let (mut cdp, session) = attached(&endpoint);

        assert_eq!(
            cdp.follow_link(&session, "保有銘柄", None, &[]),
            Err(Error::LinkRefused)
        );
        assert_eq!(log.lock().unwrap().len(), 1, "only the search ran");
    }
}

#[test]
fn a_missing_link_and_a_link_that_is_gone_are_reported() {
    let endpoint = fake_browser(json!([]));
    let (mut cdp, session) = attached(&endpoint);
    assert_eq!(
        cdp.follow_link(&session, "保有銘柄", None, &[]),
        Err(Error::LinkNotFound)
    );

    let endpoint = fake_browser_queue(
        vec![
            found("/a", true, false),
            json!("https://h.example/old"),
            json!("gone"),
        ],
        false,
    );
    let (mut cdp, session) = attached(&endpoint);
    assert_eq!(
        cdp.follow_link(&session, "保有銘柄", None, &[]),
        Err(Error::LinkNotFound)
    );
}

fn two_links(a: &str, b: &str) -> Value {
    json!([
        {"index": 1, "href": a, "same_site": true, "scripted": false, "origin": "https://h.example", "target_origin": "https://h.example"},
        {"index": 7, "href": b, "same_site": true, "scripted": false, "origin": "https://h.example", "target_origin": "https://h.example"},
    ])
}

#[test]
fn links_with_the_same_text_and_the_same_destination_count_as_one() {
    let log = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let endpoint = fake_browser_logged(
        vec![
            two_links(
                "/ETGate/?_ControlID=A&_SeqNo=1111111111",
                "/ETGate/?_ControlID=A&_SeqNo=2222222222",
            ),
            json!("https://h.example/old"),
            json!("navigated"),
            probe(false, "complete"),
            json!(true),
            json!("https://h.example"),
        ],
        false,
        Some(log.clone()),
    );
    let (mut cdp, session) = attached(&endpoint);

    let followed = cdp
        .follow_link(&session, "ポートフォリオ", None, &[])
        .unwrap();

    assert_eq!(
        followed.reference,
        crate::fingerprint("/ETGate/?_ControlID=A")
    );
    assert!(
        log.lock().unwrap()[2]
            .contains(", 1, \"/ETGate/?_ControlID=A&_SeqNo=1111111111\", \"https://h.example\", \"https://h.example\")"),
        "the first one is followed"
    );
}

#[test]
fn links_with_the_same_text_and_different_destinations_are_not_guessed() {
    let log = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let endpoint = fake_browser_logged(
        vec![two_links("/a/list", "/b/list")],
        false,
        Some(log.clone()),
    );
    let (mut cdp, session) = attached(&endpoint);

    let error = cdp.follow_link(&session, "一覧", None, &[]).unwrap_err();

    assert_eq!(
        error,
        Error::LinkAmbiguous(vec![
            crate::LinkChoice {
                reference: crate::fingerprint("/a/list"),
                label: "/a/list".to_owned(),
            },
            crate::LinkChoice {
                reference: crate::fingerprint("/b/list"),
                label: "/b/list".to_owned(),
            },
        ])
    );
    assert_eq!(log.lock().unwrap().len(), 1, "nothing was clicked");
}

#[test]
fn a_saved_reference_picks_the_right_link_and_a_changed_page_stops() {
    let log = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let endpoint = fake_browser_logged(
        vec![
            two_links("/a/list", "/b/list"),
            json!("https://h.example/old"),
            json!("navigated"),
            probe(false, "complete"),
            json!(true),
            json!("https://h.example"),
        ],
        false,
        Some(log.clone()),
    );
    let (mut cdp, session) = attached(&endpoint);
    cdp.follow_link(&session, "一覧", Some(&crate::fingerprint("/b/list")), &[])
        .unwrap();
    assert!(
        log.lock().unwrap()[2]
            .contains(", 7, \"/b/list\", \"https://h.example\", \"https://h.example\")"),
        "the second link was followed"
    );

    let log = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let endpoint = fake_browser_logged(
        vec![two_links("/a/list", "/b/list")],
        false,
        Some(log.clone()),
    );
    let (mut cdp, session) = attached(&endpoint);
    assert_eq!(
        cdp.follow_link(&session, "一覧", Some(&crate::fingerprint("/c/list")), &[]),
        Err(Error::LinkChanged)
    );
    assert_eq!(log.lock().unwrap().len(), 1, "nothing was clicked");
}

#[test]
fn a_link_the_page_handles_itself_is_never_followed() {
    let log = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let endpoint = fake_browser_logged(vec![found("", false, true)], false, Some(log.clone()));
    let (mut cdp, session) = attached(&endpoint);

    assert_eq!(
        cdp.follow_link(&session, "保有銘柄", None, &[]),
        Err(Error::LinkRefused)
    );
    assert_eq!(log.lock().unwrap().len(), 1, "nothing was clicked");
}

#[test]
fn an_address_that_was_too_long_to_check_is_refused() {
    let mut page = found("/ok/list", true, false);
    page[0]["too_long"] = json!(true);
    let endpoint = fake_browser(page);
    let (mut cdp, session) = attached(&endpoint);

    assert_eq!(
        cdp.follow_link(&session, "保有銘柄", None, &[]),
        Err(Error::LinkRefused)
    );
}

#[test]
fn links_that_differ_only_in_an_account_number_are_different_destinations() {
    let endpoint = fake_browser(two_links(
        "/holdings?account=1234567890",
        "/holdings?account=2234567890",
    ));
    let (mut cdp, session) = attached(&endpoint);

    let error = cdp.follow_link(&session, "一覧", None, &[]).unwrap_err();

    assert!(matches!(error, Error::LinkAmbiguous(choices) if choices.len() == 2));
}

#[test]
fn the_old_document_saying_complete_is_not_taken_for_the_new_page() {
    let endpoint = fake_browser_queue(
        vec![
            found("/next", true, false),
            json!("https://h.example/old"),
            json!("navigated"),
            // The old document is still there (its mark is on it) and still says complete.
            probe(true, "complete"),
            probe(true, "complete"),
            // Now the new document has replaced it.
            probe(false, "complete"),
            json!(true),
            json!("https://h.example"),
        ],
        false,
    );
    let (mut cdp, session) = attached(&endpoint);
    let started = std::time::Instant::now();

    cdp.follow_link(&session, "保有銘柄", None, &[]).unwrap();

    assert!(
        started.elapsed() >= std::time::Duration::from_millis(900),
        "it kept waiting through the two probes of the old document"
    );
}

#[test]
fn a_page_that_only_changes_its_address_is_also_recognized() {
    let endpoint = fake_browser_queue(
        vec![
            found("/app#/holdings", true, false),
            json!("https://h.example/old"),
            json!("navigated"),
            json!({"marker": true, "ready": "complete", "href": "https://h.example/app#/holdings"}),
            json!({"marker": true, "ready": "complete", "href": "https://h.example/app#/holdings"}),
            json!("https://h.example"),
        ],
        false,
    );
    let (mut cdp, session) = attached(&endpoint);

    assert!(cdp.follow_link(&session, "保有銘柄", None, &[]).is_ok());
}

#[test]
fn a_page_on_another_site_after_the_move_is_not_accepted() {
    let endpoint = fake_browser_queue(
        vec![
            found("/next", true, false),
            json!("https://h.example/old"),
            json!("navigated"),
            probe(false, "complete"),
            json!(true),
            // A redirect took the tab to another site.
            json!("https://elsewhere.example"),
        ],
        false,
    );
    let (mut cdp, session) = attached(&endpoint);

    assert_eq!(
        cdp.follow_link(&session, "保有銘柄", None, &[]),
        Err(Error::WrongPage)
    );
}

#[test]
fn going_back_is_accepted_only_on_the_start_page() {
    let endpoint = fake_browser_queue(
        vec![
            json!("https://h.example/old"),
            json!(true),
            probe(false, "complete"),
            json!(true),
            json!("https://h.example/other"),
        ],
        false,
    );
    let (mut cdp, session) = attached(&endpoint);

    assert_eq!(
        cdp.go_back(&session, 1, "https://h.example/start"),
        Err(Error::WrongPage)
    );
}

#[test]
fn a_redirect_to_a_refused_address_on_the_same_site_is_not_accepted() {
    let endpoint = fake_browser_queue(
        vec![
            found("/next", true, false),
            json!("https://h.example/old"),
            json!("navigated"),
            probe(false, "complete"),
            json!(true),
            json!("https://h.example"),
            json!("/logout"),
        ],
        false,
    );
    let (mut cdp, session) = attached(&endpoint);

    assert_eq!(
        cdp.follow_link(&session, "保有銘柄", None, &[]),
        Err(Error::WrongPage)
    );
}

#[test]
fn a_link_to_another_host_of_an_allowed_site_is_followed_and_checked_on_arrival() {
    let to_www = |site: &str| {
        let mut page = found("/ETGate/", true, false);
        page[0]["same_site"] = json!(false);
        page[0]["target_origin"] = json!("https://www.example.co.jp");
        fake_browser_logged(
            vec![
                page,
                json!("https://site3.example.co.jp/start"),
                json!("navigated"),
                probe(false, "complete"),
                json!(true),
                json!(site),
                json!("/ETGate/"),
            ],
            false,
            None,
        )
    };
    let sites = vec!["example.co.jp".to_owned()];

    // Arrived where the link went: accepted.
    let endpoint = to_www("https://www.example.co.jp");
    let (mut cdp, session) = attached(&endpoint);
    assert!(
        cdp.follow_link(&session, "口座(外貨建)", None, &sites)
            .is_ok()
    );

    // Not allowed without the site: refused before anything is done.
    let endpoint = to_www("https://www.example.co.jp");
    let (mut cdp, session) = attached(&endpoint);
    assert_eq!(
        cdp.follow_link(&session, "口座(外貨建)", None, &[]),
        Err(Error::LinkRefused)
    );

    // A redirect took the tab to another company: not accepted.
    let endpoint = to_www("https://other.test");
    let (mut cdp, session) = attached(&endpoint);
    assert_eq!(
        cdp.follow_link(&session, "口座(外貨建)", None, &sites),
        Err(Error::WrongPage)
    );
}

#[test]
fn a_login_page_is_opened_by_going_to_its_address_only() {
    let log = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let endpoint = fake_browser_logged(vec![json!(null)], false, Some(log.clone()));
    let mut cdp = Cdp::connect(&endpoint).unwrap();

    cdp.open_tab("https://www.example.co.jp/login.html")
        .unwrap();

    assert_eq!(
        log.lock().unwrap().clone(),
        vec!["createTarget https://www.example.co.jp/login.html".to_owned()],
        "one call: nothing is attached, evaluated, clicked, or typed"
    );
}

// ---- clicking a menu entry ---------------------------------------------------

fn menu_page(
    origin: &str,
    answer: &str,
    after_origin: &str,
    heading: &str,
) -> (Endpoint, std::sync::Arc<std::sync::Mutex<Vec<String>>>) {
    let log = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let endpoint = fake_browser_logged(
        vec![
            json!(origin),
            json!("https://h.example/old"),
            json!(answer),
            probe(false, "complete"),
            json!(true),
            json!(after_origin),
            json!("/web/App"),
            json!(heading),
        ],
        false,
        Some(log.clone()),
    );
    (endpoint, log)
}

fn vetted() -> Vec<String> {
    vec!["example.co.jp".to_owned()]
}

#[test]
fn a_menu_entry_is_clicked_waited_for_and_its_heading_checked() {
    let (endpoint, log) = menu_page(
        "https://trade.example.co.jp",
        "clicked",
        "https://trade.example.co.jp",
        "Holdings list",
    );
    let (mut cdp, session) = attached(&endpoint);
    cdp.click_menu_link(&session, "Stocks", "Holdings", "Holdings list", &vetted())
        .unwrap();

    let scripts = log.lock().unwrap().clone();
    assert_eq!(
        scripts.len(),
        8,
        "site, mark, click, readiness, settle, site, address, heading"
    );
    assert!(scripts[1].contains("__roboAdvisorNav"));
    assert!(
        scripts[2].ends_with("(\"Stocks\", \"Holdings\", \"https://trade.example.co.jp\")"),
        "the texts and the site are passed as JSON data: {}",
        scripts[2]
    );
    assert!(scripts[2].contains("a.click()"));
    assert_eq!(
        scripts.iter().filter(|s| s.contains("a.click()")).count(),
        1,
        "exactly one click"
    );
}

#[test]
fn a_menu_click_is_refused_outside_the_vetted_sites_and_for_risky_texts() {
    let (endpoint, log) = menu_page(
        "https://trade.other.test",
        "clicked",
        "https://trade.other.test",
        "Holdings list",
    );
    let (mut cdp, session) = attached(&endpoint);
    assert_eq!(
        cdp.click_menu_link(&session, "Stocks", "Holdings", "Holdings list", &vetted()),
        Err(Error::LinkRefused)
    );
    assert!(
        !log.lock().unwrap().iter().any(|s| s.contains("a.click()")),
        "nothing is clicked on a site that is not vetted"
    );
    assert_eq!(
        cdp.click_menu_link(&session, "Stocks", "Holdings", "Holdings list", &[]),
        Err(Error::LinkRefused)
    );

    for (group, text) in [
        ("Stocks", "出金指示"),
        ("Stocks", "買付注文"),
        ("Stocks", "ログアウト"),
        ("入出金", "入金口座表示"),
        ("ログアウト", "Holdings"),
        ("Stocks", ""),
    ] {
        let (endpoint, log) = menu_page(
            "https://trade.example.co.jp",
            "clicked",
            "https://trade.example.co.jp",
            "Holdings list",
        );
        let (mut cdp, session) = attached(&endpoint);
        assert_eq!(
            cdp.click_menu_link(&session, group, text, "Holdings list", &vetted()),
            Err(Error::LinkRefused),
            "{group} / {text}"
        );
        assert!(
            log.lock().unwrap().is_empty(),
            "the page is not asked for a risky text: {group} / {text}"
        );
    }
    // A heading that is empty or has control characters is not a usable check.
    let (endpoint, _) = menu_page(
        "https://trade.example.co.jp",
        "clicked",
        "https://trade.example.co.jp",
        "",
    );
    let (mut cdp, session) = attached(&endpoint);
    assert_eq!(
        cdp.click_menu_link(&session, "Stocks", "Holdings", " ", &vetted()),
        Err(Error::LinkRefused)
    );
}

#[test]
fn a_menu_entry_that_is_missing_ambiguous_or_not_scripted_is_not_clicked() {
    for (answer, expected) in [
        ("none", Error::LinkNotFound),
        ("gone", Error::LinkNotFound),
        ("ambiguous", Error::LinkRefused),
        ("refused", Error::LinkRefused),
        ("something else", Error::LinkRefused),
    ] {
        let (endpoint, _) = menu_page(
            "https://trade.example.co.jp",
            answer,
            "https://trade.example.co.jp",
            "Holdings list",
        );
        let (mut cdp, session) = attached(&endpoint);
        assert_eq!(
            cdp.click_menu_link(&session, "Stocks", "Holdings", "Holdings list", &vetted()),
            Err(expected),
            "{answer}"
        );
    }
}

#[test]
fn the_page_after_a_menu_click_must_be_on_the_same_site_with_the_expected_heading() {
    // The tab went to another site.
    let (endpoint, _) = menu_page(
        "https://trade.example.co.jp",
        "clicked",
        "https://elsewhere.test",
        "Holdings list",
    );
    let (mut cdp, session) = attached(&endpoint);
    assert_eq!(
        cdp.click_menu_link(&session, "Stocks", "Holdings", "Holdings list", &vetted()),
        Err(Error::WrongPage)
    );
    // The heading is not the expected one: the batch must stop.
    let (endpoint, _) = menu_page(
        "https://trade.example.co.jp",
        "clicked",
        "https://trade.example.co.jp",
        "Something else",
    );
    let (mut cdp, session) = attached(&endpoint);
    assert_eq!(
        cdp.click_menu_link(&session, "Stocks", "Holdings", "Holdings list", &vetted()),
        Err(Error::WrongPage)
    );
}

#[test]
fn the_click_script_only_clicks_one_scripted_entry_of_the_named_group() {
    let script = crate::links::CLICK_MENU;
    // One entry, found by its group; nothing else is touched.
    assert!(script.contains("hits.length > 1"));
    assert!(script.contains("!scripted"));
    assert!(script.contains("a.target !== '_self'"));
    assert!(script.contains("hasAttribute('download')"));
    assert_eq!(script.matches(".click()").count(), 1);
    for word in [
        "submit(",
        ".value",
        "dispatchEvent",
        "fetch(",
        "XMLHttpRequest",
        "form",
    ] {
        assert!(!script.contains(word), "{word}");
    }
}

#[test]
fn scripts_run_in_one_isolated_world_that_is_made_again_when_its_context_is_gone() {
    let log = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let endpoint = fake_browser_logged(
        vec![
            json!({"leaf": [], "outer": []}),
            json!("__context_lost__"),
            json!({"leaf": [], "outer": []}),
        ],
        false,
        Some(log.clone()),
    );
    let mut cdp = Cdp::connect(&endpoint).unwrap();
    let pages = cdp.pages().unwrap();
    let session = cdp.attach(&pages[0]).unwrap();

    cdp.read_tables(&session).unwrap();
    cdp.read_tables(&session).unwrap();

    let log = log.lock().unwrap();
    let worlds: Vec<_> = log.iter().filter(|l| l.starts_with("world ")).collect();
    assert_eq!(
        worlds,
        ["world 1", "world 2"],
        "made once, and again only after the browser said the context was gone"
    );
}

#[test]
fn the_tables_are_read_again_until_the_title_shows_and_the_page_holds_still() {
    let empty = json!({"leaf": [], "outer": []});
    let with = json!({"leaf": [[["Holdings", "x"]]], "outer": []});
    let log = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let endpoint = fake_browser_logged(
        vec![empty.clone(), empty, with.clone(), with],
        false,
        Some(log.clone()),
    );
    let mut cdp = Cdp::connect(&endpoint).unwrap();
    let pages = cdp.pages().unwrap();
    let session = cdp.attach(&pages[0]).unwrap();

    let ready = |page: &PageTables| page.leaf.iter().any(|t| t[0][0] == "Holdings");
    let page = cdp
        .read_tables_until(&session, &ready, std::time::Duration::from_secs(10))
        .unwrap();

    assert_eq!(page.leaf.len(), 1);
    let reads = log
        .lock()
        .unwrap()
        .iter()
        .filter(|l| l.contains("querySelectorAll"))
        .count();
    assert_eq!(reads, 4, "two empty reads, then the table twice in a row");
}

#[test]
fn a_title_that_never_shows_ends_the_wait_with_what_was_read() {
    let endpoint = fake_browser(json!({"leaf": [], "outer": []}));
    let mut cdp = Cdp::connect(&endpoint).unwrap();
    let pages = cdp.pages().unwrap();
    let session = cdp.attach(&pages[0]).unwrap();

    let page = cdp
        .read_tables_until(&session, &|_| false, std::time::Duration::from_millis(800))
        .unwrap();

    assert!(page.is_empty());
}

#[test]
fn a_title_seen_only_in_the_last_read_is_not_taken_as_complete() {
    let with = json!({"leaf": [[["Holdings", "x"]]], "outer": []});
    let endpoint = fake_browser(with);
    let mut cdp = Cdp::connect(&endpoint).unwrap();
    let pages = cdp.pages().unwrap();
    let session = cdp.attach(&pages[0]).unwrap();

    // The table is there from the first read; with no time to confirm it, it is not accepted.
    let result = cdp.read_tables_until(&session, &|p| !p.is_empty(), std::time::Duration::ZERO);

    assert_eq!(result, Err(Error::Timeout));
}
