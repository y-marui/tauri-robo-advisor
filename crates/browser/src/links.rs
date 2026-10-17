//! Following a link to show another page.
//!
//! This is the only kind of action the crate takes in a page, and it is kept
//! narrow on purpose: an `<a>` link with a real address, found by its exact
//! visible text, on the site the user logged in to. Nothing is typed, submitted, or entered. A link
//! whose text or address looks like an order or a money movement is refused
//! even when a recipe names it, unless it is clearly a view of past or current
//! state (an inquiry, a history, a list, a balance).

use serde::Deserialize;

use crate::Error;

const MAX_LINK_TEXT_CHARS: usize = 60;
const MAX_HREF_CHARS: usize = 500;

/// Words that suggest an action that changes something (orders, money, the account, the session).
const RISKY_TEXT: &[&str] = &[
    "注文",
    "発注",
    "売却",
    "売付",
    "買付",
    "買い",
    "売り",
    "購入",
    "入金",
    "出金",
    "送金",
    "振替",
    "振込",
    "解約",
    "申込",
    "申し込み",
    "取消",
    "訂正",
    "変更",
    "削除",
    "登録",
    "承認",
    "確定",
    "実行",
    "約定",
    "ログアウト",
    "order",
    "buy",
    "sell",
    "purchase",
    "deposit",
    "withdraw",
    "transfer",
    "trade",
    "payment",
    "cancel",
    "delete",
    "submit",
    "confirm",
    "logout",
    "sign out",
];

/// Words that show a link only displays something (so "注文照会" or "買付余力" may be followed).
const VIEW_TEXT: &[&str] = &[
    "照会", "履歴", "一覧", "余力", "残高", "明細", "状況", "inquiry", "history", "list",
    "balance", "status", "detail", "summary", "position", "holding",
];

/// The same idea for the address, which is English-ish and lower case.
const RISKY_HREF: &[&str] = &[
    "order", "buy", "sell", "purchase", "deposit", "withdraw", "transfer", "trade", "payment",
    "cancel", "delete", "submit", "confirm", "logout", "signout",
];
const VIEW_HREF: &[&str] = &[
    "inquiry", "history", "list", "balance", "status", "detail", "summary", "position", "holding",
];

fn mentions(haystack: &str, words: &[&str]) -> bool {
    let lower = haystack.to_lowercase();
    words.iter().any(|w| lower.contains(w))
}

/// Whether a link may be named by a recipe, judged by its visible text.
pub fn link_text_allowed(text: &str) -> Result<(), Error> {
    let trimmed = text.trim();
    if trimmed.is_empty()
        || trimmed.chars().count() > MAX_LINK_TEXT_CHARS
        || trimmed.chars().any(char::is_control)
    {
        return Err(Error::LinkRefused);
    }
    if mentions(trimmed, RISKY_TEXT) && !mentions(trimmed, VIEW_TEXT) {
        return Err(Error::LinkRefused);
    }
    Ok(())
}

/// A link found in the page.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct Candidate {
    pub index: usize,
    /// The address, cut to a safe length; empty for a link with no address.
    #[serde(default)]
    pub href: String,
    /// The address is on the page's own site.
    #[serde(default)]
    pub same_site: bool,
    /// A `javascript:` or `#` link, or one with no address: the page handles it itself.
    #[serde(default)]
    pub scripted: bool,
    /// The address (path and query) is longer than the safe length, so it was cut and cannot be checked.
    #[serde(default)]
    pub too_long: bool,
}

impl Candidate {
    /// A short, stable description of where the link goes, so that links with the
    /// same text can be told apart and a saved link can be recognized again. The
    /// parts that change from one visit to the next are left out.
    pub fn reference(&self) -> String {
        if self.scripted {
            String::new()
        } else {
            normalize_href(&self.href)
        }
    }

    pub(crate) fn allowed(&self) -> bool {
        // A link the page handles itself (`javascript:`, `#`, no address) runs the
        // page's own script when clicked, and nothing can show that it only
        // navigates. It is never followed.
        if self.scripted {
            return false;
        }
        if !self.same_site || self.too_long || self.href.chars().count() > MAX_HREF_CHARS {
            return false;
        }
        address_looks_read_only(&self.href)
    }
}

/// Decodes `%xx` escapes (more than once, in case of double encoding) so that a
/// word such as `/trans%66er` is seen as what the server will read. The address that
/// is actually followed stays as it was; this is only for judging it.
fn percent_decoded(text: &str) -> String {
    let mut current = text.to_owned();
    for _ in 0..3 {
        let bytes = current.as_bytes();
        let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
        let mut i = 0;
        let mut changed = false;
        while i < bytes.len() {
            let hex = |b: u8| (b as char).to_digit(16);
            if bytes[i] == b'%'
                && i + 2 < bytes.len()
                && let (Some(high), Some(low)) = (hex(bytes[i + 1]), hex(bytes[i + 2]))
            {
                out.push((high * 16 + low) as u8);
                i += 3;
                changed = true;
            } else {
                out.push(if bytes[i] == b'+' { b' ' } else { bytes[i] });
                i += 1;
            }
        }
        current = String::from_utf8_lossy(&out).into_owned();
        if !changed {
            break;
        }
    }
    current
}

/// Whether an address (path and query) looks like it only shows something. Every
/// part is judged on its own: a path segment or a query pair with an action word
/// is refused unless the same part also says it is a view (so `orderhistory` is
/// fine), and a view word in a neighbouring part never excuses it
/// (`/history/delete` and `/transfer/history` are both refused).
fn address_looks_read_only(href: &str) -> bool {
    let decoded = percent_decoded(href);
    // A page that routes by the part after `#` is judged like the rest of the address.
    let (before, fragment) = decoded.split_once('#').unwrap_or((decoded.as_str(), ""));
    let (route, query) = before.split_once('?').unwrap_or((before, ""));
    route
        .split('/')
        .chain(query.split('&'))
        .chain(fragment.split(['/', '&', '?']))
        .all(|part| !mentions(part, RISKY_HREF) || mentions(part, VIEW_HREF))
}

/// Words in a query name that mean the value belongs to one visit.
const VOLATILE_KEYS: &[&str] = &[
    "seq", "token", "session", "sid", "time", "stamp", "nonce", "rand", "csrf",
];

/// A value that is a timestamp (13 digits or more, like a time in milliseconds) or a long
/// token (24 or more hex characters) is made for one visit. Shorter numbers, such as an
/// account number, are part of where the link goes and are kept.
fn volatile_value(value: &str) -> bool {
    (value.len() >= 13 && value.chars().all(|c| c.is_ascii_digit()))
        || (value.len() >= 24 && value.chars().all(|c| c.is_ascii_hexdigit()))
}

/// The path of an address and its query without the parts that change per visit.
/// The remaining keys are sorted, so the same destination always gives the same text.
pub fn normalize_href(href: &str) -> String {
    // The part after `#` is kept as it is: on a page that routes by it, it is where the link goes.
    let (href, fragment) = href.split_once('#').unwrap_or((href, ""));
    let fragment = if fragment.is_empty() {
        String::new()
    } else {
        format!("#{fragment}")
    };
    let (path, query) = href.split_once('?').unwrap_or((href, ""));
    let mut kept: Vec<(String, String)> = query
        .split('&')
        .filter(|pair| !pair.is_empty())
        .filter_map(|pair| {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            let lower = key.to_lowercase();
            let volatile = VOLATILE_KEYS.iter().any(|k| lower.contains(k)) || volatile_value(value);
            (!volatile).then(|| (key.to_owned(), value.to_owned()))
        })
        .collect();
    kept.sort();
    let query: Vec<String> = kept.iter().map(|(k, v)| format!("{k}={v}")).collect();
    if query.is_empty() {
        format!("{path}{fragment}")
    } else {
        format!("{path}?{}{fragment}", query.join("&"))
    }
}

pub(crate) fn parse_candidates(value: &serde_json::Value) -> Result<Vec<Candidate>, Error> {
    let list = value.as_array().ok_or(Error::Script)?;
    if list.len() > 20 {
        return Err(Error::Script);
    }
    list.iter()
        .map(|item| serde_json::from_value(item.clone()).map_err(|_| Error::Script))
        .collect()
}

/// Finds the links with exactly this visible text and says where each one goes.
/// It only reads. The text is passed in as data, JSON-encoded.
pub(crate) const FIND_LINKS: &str = r#"((wanted) => {
  const clean = (t) => String(t ?? '').replace(/\s+/g, ' ').trim();
  const seen = new Map();
  [...document.querySelectorAll('a')].forEach((a, i) => {
    if (clean(a.innerText ?? a.textContent) !== wanted) return;
    const raw = a.getAttribute('href') ?? '';
    const scripted = raw === '' || raw.startsWith('#') || raw.toLowerCase().startsWith('javascript:');
    let sameSite = false;
    let href = '';
    let tooLong = false;
    if (!scripted) {
      try {
        const url = new URL(raw, location.href);
        sameSite = (url.protocol === 'https:' || url.protocol === 'http:') && url.host === location.host;
        const full = url.pathname + url.search + url.hash;
        tooLong = full.length > 500;
        href = full.slice(0, 500);
      } catch (e) { href = ''; }
    }
    // Every matching link is looked at, but each distinct destination is listed once.
    const key = scripted + '|' + sameSite + '|' + tooLong + '|' + href;
    if (!seen.has(key) && seen.size < 21) seen.set(key, { index: i, href, same_site: sameSite, scripted, too_long: tooLong });
  });
  return [...seen.values()];
})"#;

/// Goes to the address of the link at the given position, without clicking it:
/// a click would run whatever script the page attached to the link. Everything is
/// checked again in the page (the text, the site, and the exact path and query that
/// the Rust side accepted), and anything that differs is not followed.
pub(crate) const NAVIGATE: &str = r#"((wanted, index, expected) => {
  const clean = (t) => String(t ?? '').replace(/\s+/g, ' ').trim();
  const a = document.querySelectorAll('a')[index];
  if (!a || clean(a.innerText ?? a.textContent) !== wanted) return 'gone';
  let url;
  try { url = new URL(a.getAttribute('href') ?? '', location.href); } catch (e) { return 'gone'; }
  if ((url.protocol !== 'https:' && url.protocol !== 'http:') || url.host !== location.host) return 'gone';
  if (url.pathname + url.search + url.hash !== expected) return 'gone';
  location.assign(url.href);
  return 'navigated';
})"#;

pub(crate) const READY_STATE: &str = "document.readyState";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_view_links_are_allowed() {
        for ok in [
            "口座管理",
            "口座(円建)",
            "口座(外貨建)",
            "保有銘柄",
            "資産残高・推移（My資産）",
            "注文照会",
            "買付余力",
            "出金履歴",
            "Account balance",
        ] {
            assert!(link_text_allowed(ok).is_ok(), "{ok}");
        }
    }

    #[test]
    fn links_that_act_are_refused() {
        for bad in [
            "注文",
            "株式 買い注文",
            "売却",
            "入金",
            "出金",
            "振替",
            "解約",
            "ログアウト",
            "Buy",
            "Place order",
            "Withdraw funds",
            "Sign out",
            "Transfer",
            "",
        ] {
            assert_eq!(link_text_allowed(bad), Err(Error::LinkRefused), "{bad:?}");
        }
        assert_eq!(link_text_allowed(&"あ".repeat(61)), Err(Error::LinkRefused));
        assert_eq!(link_text_allowed("a\nb"), Err(Error::LinkRefused));
    }

    fn candidate(href: &str, same_site: bool, scripted: bool) -> Candidate {
        Candidate {
            index: 0,
            href: href.to_owned(),
            same_site,
            scripted,
            too_long: false,
        }
    }

    #[test]
    fn only_same_site_links_that_do_not_act_are_followed() {
        assert!(candidate("/ETGate/?_ControlID=WPLETacR001Control", true, false).allowed());
        assert!(candidate("/orderhistory/list", true, false).allowed());
        assert!(!candidate("/trade/order/confirm", true, false).allowed());
        assert!(!candidate("/transfer", true, false).allowed());
        assert!(
            !candidate("/transfer?account=123", true, false).allowed(),
            "a word in the query does not make a transfer a view"
        );
        assert!(!candidate("/order/confirm?return=/portfolio", true, false).allowed());
        assert!(
            !candidate("/account/transfer", true, false).allowed(),
            "account is not a sign of a view"
        );
        assert!(
            !candidate("/transfer/history", true, false).allowed(),
            "an action word in one part is not excused by a view word in another"
        );
        assert!(!candidate("/history/delete", true, false).allowed());
        assert!(!candidate("/transfer/history/cancel", true, false).allowed());
        assert!(
            !candidate("/trans%66er", true, false).allowed(),
            "escapes are decoded"
        );
        assert!(!candidate("/log%6fut", true, false).allowed());
        assert!(
            !candidate("/tr%2561nsfer", true, false).allowed(),
            "double encoding"
        );
        assert!(!candidate("/page?action=cancel", true, false).allowed());
        assert!(
            !candidate("/#/transfer", true, false).allowed(),
            "a route after #"
        );
        assert!(!candidate("/app#/history/delete", true, false).allowed());
        assert!(candidate("/app#/orderhistory", true, false).allowed());
        assert!(!candidate("/app#!/logout", true, false).allowed());
        assert!(candidate("/orderhistory/list", true, false).allowed());
        assert!(
            !candidate("/ETGate/", false, false).allowed(),
            "another site"
        );
        assert!(
            !candidate("", false, true).allowed(),
            "a link the page handles itself is never followed"
        );
        assert!(!candidate(&"a".repeat(600), true, false).allowed());
    }

    #[test]
    fn the_parts_of_an_address_that_change_per_visit_are_left_out() {
        assert_eq!(
            normalize_href(
                "/ETGate/?_ControlID=WPLETacR001Control&_SeqNo=1649728758632_default_task_354&getFlg=on"
            ),
            "/ETGate/?_ControlID=WPLETacR001Control&getFlg=on",
        );
        assert_eq!(
            normalize_href(
                "/a?b=2&a=1&sessionId=abc&n=0123456789abcdef0123456789abcdef&t=1649728758632"
            ),
            "/a?a=1&b=2",
            "sorted, and timestamps, long tokens, and session-like names are dropped",
        );
        assert_ne!(
            normalize_href("/holdings?account=1234567890"),
            normalize_href("/holdings?account=2234567890"),
            "an account number is part of where the link goes",
        );
        assert_eq!(normalize_href("/plain"), "/plain");
        assert_eq!(
            normalize_href("/app?_SeqNo=1&b=2#/holdings"),
            "/app?b=2#/holdings",
            "the part after # is kept"
        );
        assert_ne!(normalize_href("/app#/a"), normalize_href("/app#/b"));
        assert_eq!(normalize_href("/x?"), "/x");
    }

    #[test]
    fn what_the_page_returns_is_checked() {
        let good =
            serde_json::json!([{"index": 3, "href": "/a", "same_site": true, "scripted": false}]);
        assert_eq!(parse_candidates(&good).unwrap()[0].index, 3);
        assert_eq!(
            parse_candidates(&serde_json::json!("x")),
            Err(Error::Script)
        );
        assert_eq!(
            parse_candidates(&serde_json::json!([{"href": "/a"}])),
            Err(Error::Script)
        );
        assert_eq!(
            parse_candidates(&serde_json::json!(vec![serde_json::json!({}); 21])),
            Err(Error::Script)
        );
    }
}
