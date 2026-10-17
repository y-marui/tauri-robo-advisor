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
                "Runtime.evaluate" => {
                    assert_eq!(request["sessionId"], "s1");
                    if let Some(log) = &log {
                        let expression = request["params"]["expression"].as_str().unwrap_or("");
                        log.lock().unwrap().push(expression.to_owned());
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
    let endpoint = fake_browser(json!([[["a", "1"], ["b", "2"]]]));
    let mut cdp = Cdp::connect(&endpoint).unwrap();

    let pages = cdp.pages().unwrap();
    assert_eq!(pages.len(), 1, "workers are not pages");
    assert_eq!(pages[0].url, "https://example.test/holdings");

    let session = cdp.attach(&pages[0]).unwrap();
    let tables = cdp.read_tables(&session).unwrap();
    assert_eq!(tables, vec![vec![vec!["a", "1"], vec!["b", "2"]]]);

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
        json!([[["x".repeat(2000)]]]),
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

fn found(href: &str, same_site: bool, scripted: bool) -> Value {
    json!([{"index": 4, "href": href, "same_site": same_site, "scripted": scripted}])
}

#[test]
fn a_plain_link_is_found_clicked_and_waited_for() {
    let log = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let endpoint = fake_browser_logged(
        vec![
            found("/account/list", true, false),
            json!("navigated"),
            json!("loading"),
            json!("complete"),
        ],
        false,
        Some(log.clone()),
    );
    let (mut cdp, session) = attached(&endpoint);

    let followed = cdp.follow_link(&session, "口座(円建)", None).unwrap();
    assert_eq!(followed.reference, "/account/list");

    let scripts = log.lock().unwrap().clone();
    assert_eq!(scripts.len(), 4, "find, click, then two readiness checks");
    assert!(
        scripts[0].contains("\"口座(円建)\""),
        "the text is passed as JSON data"
    );
    assert!(
        scripts[1].contains(", 4, \"/account/list\")"),
        "the position and the accepted address come from the search"
    );
    assert_eq!(scripts[2], "document.readyState");
}

#[test]
fn a_link_text_is_passed_as_data_not_as_code() {
    let log = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let endpoint = fake_browser_logged(vec![json!([])], false, Some(log.clone()));
    let (mut cdp, session) = attached(&endpoint);

    let result = cdp.follow_link(&session, "x\"); alert(1); (\"", None);

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
        cdp.follow_link(&session, "買い注文", None),
        Err(Error::LinkRefused)
    );
    assert_eq!(
        cdp.follow_link(&session, "ログアウト", None),
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
            cdp.follow_link(&session, "保有銘柄", None),
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
        cdp.follow_link(&session, "保有銘柄", None),
        Err(Error::LinkNotFound)
    );

    let endpoint = fake_browser_queue(vec![found("/a", true, false), json!("gone")], false);
    let (mut cdp, session) = attached(&endpoint);
    assert_eq!(
        cdp.follow_link(&session, "保有銘柄", None),
        Err(Error::LinkNotFound)
    );
}

fn two_links(a: &str, b: &str) -> Value {
    json!([
        {"index": 1, "href": a, "same_site": true, "scripted": false},
        {"index": 7, "href": b, "same_site": true, "scripted": false},
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
            json!("navigated"),
            json!("complete"),
        ],
        false,
        Some(log.clone()),
    );
    let (mut cdp, session) = attached(&endpoint);

    let followed = cdp.follow_link(&session, "ポートフォリオ", None).unwrap();

    assert_eq!(followed.reference, "/ETGate/?_ControlID=A");
    assert!(
        log.lock().unwrap()[1].contains(", 1, \"/ETGate/?_ControlID=A&_SeqNo=1111111111\")"),
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

    let error = cdp.follow_link(&session, "一覧", None).unwrap_err();

    assert_eq!(
        error,
        Error::LinkAmbiguous(vec!["/a/list".to_owned(), "/b/list".to_owned()])
    );
    assert_eq!(log.lock().unwrap().len(), 1, "nothing was clicked");
}

#[test]
fn a_saved_reference_picks_the_right_link_and_a_changed_page_stops() {
    let log = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let endpoint = fake_browser_logged(
        vec![
            two_links("/a/list", "/b/list"),
            json!("navigated"),
            json!("complete"),
        ],
        false,
        Some(log.clone()),
    );
    let (mut cdp, session) = attached(&endpoint);
    cdp.follow_link(&session, "一覧", Some("/b/list")).unwrap();
    assert!(
        log.lock().unwrap()[1].contains(", 7, \"/b/list\")"),
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
        cdp.follow_link(&session, "一覧", Some("/c/list")),
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
        cdp.follow_link(&session, "保有銘柄", None),
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
        cdp.follow_link(&session, "保有銘柄", None),
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

    let error = cdp.follow_link(&session, "一覧", None).unwrap_err();

    assert!(matches!(error, Error::LinkAmbiguous(choices) if choices.len() == 2));
}
