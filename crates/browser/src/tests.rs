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
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut socket = tungstenite::accept(stream).unwrap();
        let mut first = true;
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
                    if request["params"]["expression"] == "throw" {
                        json!({"exceptionDetails": {"text": "secret page text"}})
                    } else {
                        json!({"result": {"type": "object", "value": script_value}})
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
    let endpoint = fake_browser(json!([["a", "1"], ["b", "2"]]));
    let mut cdp = Cdp::connect(&endpoint).unwrap();

    let pages = cdp.pages().unwrap();
    assert_eq!(pages.len(), 1, "workers are not pages");
    assert_eq!(pages[0].url, "https://example.test/holdings");

    let session = cdp.attach(&pages[0]).unwrap();
    let value = cdp.evaluate(&session, "read()").unwrap();
    assert_eq!(value, json!([["a", "1"], ["b", "2"]]));

    cdp.close().unwrap();
}

#[test]
fn a_script_error_does_not_repeat_the_page_text() {
    let endpoint = fake_browser(Value::Null);
    let mut cdp = Cdp::connect(&endpoint).unwrap();
    let pages = cdp.pages().unwrap();
    let session = cdp.attach(&pages[0]).unwrap();

    let error = cdp.evaluate(&session, "throw").unwrap_err();

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
