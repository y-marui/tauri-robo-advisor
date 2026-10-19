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

/// Words that name an action that changes something. A view word never excuses them:
/// "Cancel order history" is still a cancel.
const ACTION_TEXT: &[&str] = &[
    "ログアウト",
    "取消",
    "削除",
    "変更",
    "登録",
    "承認",
    "確定",
    "実行",
    "解約",
    "解除",
    "送信",
    "保存",
    "追加",
    "訂正",
    "申込",
    "買う",
    "売る",
    "換金",
    "償還",
    "閉鎖",
    "廃止",
    "退会",
    "申し込み",
    "cancel",
    "delete",
    "submit",
    "confirm",
    "logout",
    "log out",
    "log-out",
    "logoff",
    "log off",
    "sign out",
    "sign-out",
    "signoff",
    "ログオフ",
    "サインアウト",
    "change",
    "update",
    "save",
    "edit",
    "modify",
    "register",
    "apply",
    "execute",
    "approve",
    "remove",
    "create",
    "send",
    "buy",
    "sell",
    "redeem",
    "redemption",
    "liquidate",
    "liquidation",
];

/// Order and money words. They can also name a view ("注文照会", "出金履歴", "Payment history"),
/// so a view word in the same text excuses them.
const MONEY_TEXT: &[&str] = &[
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
    "約定",
    "subscribe",
    "subscription",
    "order",
    "purchase",
    "deposit",
    "withdraw",
    "transfer",
    "trade",
    "payment",
];

/// Words that show a link only displays something (so "注文照会" or "買付余力" may be followed).
const VIEW_TEXT: &[&str] = &[
    "照会", "履歴", "一覧", "余力", "残高", "明細", "状況", "inquiry", "history", "list",
    "balance", "status", "detail", "summary", "position", "holding",
];

/// The same for the address, whose parts (path segments and query pairs) are judged one by one.
const ACTION_HREF: &[&str] = &[
    "cancel",
    "delete",
    "submit",
    "confirm",
    "logout",
    "logoff",
    "signout",
    "signoff",
    "change",
    "update",
    "save",
    "modify",
    "regist",
    "execute",
    "approve",
    "remove",
    "create",
    "settle",
    "remit",
    "buy",
    "sell",
    "redeem",
    "redemption",
    "liquidate",
    "liquidation",
    "取消",
    "取り消し",
    "削除",
    "変更",
    "更新",
    "登録",
    "解約",
    "送信",
    "保存",
    "承認",
    "確定",
    "実行",
    "訂正",
    "申込",
    "申し込み",
    "買う",
    "売る",
    "換金",
    "償還",
    "閉鎖",
    "廃止",
    "退会",
    "ログアウト",
];
/// Words that must start a word of the address ("edit" is not "credit").
const ACTION_HREF_AT_START: &[&str] = &["edit", "apply", "send"];
const MONEY_HREF: &[&str] = &[
    "order",
    "purchase",
    "deposit",
    "withdraw",
    "transfer",
    "trade",
    "payment",
    "subscribe",
    "subscription",
    "注文",
    "発注",
    "売却",
    "売付",
    "買付",
    "購入",
    "入金",
    "出金",
    "送金",
    "振替",
    "振込",
    "約定",
];
const VIEW_HREF: &[&str] = &[
    "照会", "履歴", "一覧", "余力", "残高", "明細", "状況", "inquiry", "history", "list",
    "balance", "status", "detail", "summary", "position", "holding",
];

/// Words that count only as a whole word ("add" is an action, "address" is not).
const ACTION_WHOLE_WORD: &[&str] = &["add", "adds", "close", "closing", "closure"];

fn has_whole_word(text: &str, words: &[&str]) -> bool {
    tokens(text).iter().any(|t| words.contains(&t.as_str()))
}

/// The words of an identifier or a phrase, lower-cased: split at anything that is not a
/// letter or digit and at a camelCase boundary, so `addAccount`, `add_account` and
/// `_SeqNo` give `add`, `account` and `seq`, `no`, while `address` stays one word.
fn tokens(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut previous_lower = false;
    for c in text.chars() {
        if !c.is_alphanumeric() {
            if !current.is_empty() {
                out.push(std::mem::take(&mut current));
            }
            previous_lower = false;
            continue;
        }
        if c.is_uppercase() && previous_lower && !current.is_empty() {
            out.push(std::mem::take(&mut current));
        }
        previous_lower = c.is_lowercase();
        current.extend(c.to_lowercase());
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

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
    if mentions(trimmed, ACTION_TEXT)
        || has_whole_word(trimmed, ACTION_WHOLE_WORD)
        || (mentions(trimmed, MONEY_TEXT) && !mentions(trimmed, VIEW_TEXT))
    {
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
    /// The site the page was on when the link was found; the move is made only if the page
    /// is still on it (a redirect in between must not carry the link over to another site).
    #[serde(default)]
    pub origin: String,
    /// The site the link goes to (`https://host`), when it is an http(s) address; the same as
    /// `origin` for a link on the page's own site.
    #[serde(default)]
    pub target_origin: String,
}

/// The host of an origin like `https://www.example.co.jp` (no port, no path), lower-cased; `None`
/// for anything else (an explicit port, a user name, a path, another scheme).
pub(crate) fn https_host(origin: &str) -> Option<String> {
    let host = origin.strip_prefix("https://")?.to_lowercase();
    let valid = !host.is_empty()
        && host.len() <= 253
        && host.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && label
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        });
    valid.then_some(host)
}

/// The `scheme://authority` part of an address (`https://host/path?q` gives `https://host`).
pub fn origin_of(address: &str) -> String {
    match address.split_once("://") {
        Some((scheme, rest)) => {
            let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
            format!("{scheme}://{}", &rest[..end])
        }
        None => String::new(),
    }
}

/// Whether an address may be opened as a login page: plain `https://host/...` (no user name,
/// port, white space, control character, or back slash), not too long.
pub fn login_url_ok(url: &str) -> bool {
    url.len() <= 300
        && url.starts_with("https://")
        && !url
            .chars()
            .any(|c| c.is_control() || c.is_whitespace() || c == '\\')
        && https_host(&origin_of(url)).is_some()
}

/// Whether `host` is `site` or a host below it (`www.example.co.jp` is within `example.co.jp`;
/// `badexample.co.jp` is not).
pub fn host_within(host: &str, site: &str) -> bool {
    host == site
        || host
            .strip_suffix(site)
            .is_some_and(|rest| rest.ends_with('.'))
}

/// The allowed sites of a recipe that cover the host the recipe started on. A site that does
/// not cover it is dropped: a recipe can only reach the other hosts of the site it starts on,
/// never another company's. An origin that is not `https://host` covers nothing.
pub fn sites_covering(start_origin: &str, sites: &[String]) -> Vec<String> {
    let Some(host) = https_host(start_origin) else {
        return Vec::new();
    };
    sites
        .iter()
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty() && host_within(&host, s))
        .collect()
}

impl Candidate {
    /// A short, stable description of where the link goes, so that links with the
    /// same text can be told apart and a saved link can be recognized again. The
    /// parts that change from one visit to the next are left out.
    pub fn reference(&self) -> String {
        if self.scripted {
            String::new()
        } else if self.is_cross_site() {
            // The same path on another host is another destination.
            fingerprint(&format!(
                "{}{}",
                self.target_origin,
                normalize_href(&self.href)
            ))
        } else {
            fingerprint(&normalize_href(&self.href))
        }
    }

    /// The readable form of the destination, to show while the user chooses. It is
    /// never saved: only the fingerprint is, so nothing a link carries ends up in the data.
    pub fn label(&self) -> String {
        if self.is_cross_site() {
            format!("{}{}", self.target_origin, normalize_href(&self.href))
        } else {
            normalize_href(&self.href)
        }
    }

    /// The link goes to a site other than the one the page is on.
    fn is_cross_site(&self) -> bool {
        !self.same_site && !self.target_origin.is_empty() && self.target_origin != self.origin
    }

    /// Whether the link may be followed. `sites` are the allowed sites that cover the host the
    /// recipe started on (see `sites_covering`): a link to another host is followed only when
    /// it is `https` with no explicit port and the host is within one of them.
    pub(crate) fn allowed(&self, sites: &[String]) -> bool {
        // A link the page handles itself (`javascript:`, `#`, no address) runs the
        // page's own script when clicked, and nothing can show that it only
        // navigates. It is never followed.
        if self.scripted {
            return false;
        }
        if self.too_long || self.href.chars().count() > MAX_HREF_CHARS {
            return false;
        }
        if !self.same_site {
            let within = https_host(&self.target_origin)
                .is_some_and(|host| sites.iter().any(|site| host_within(&host, site)));
            if !within {
                return false;
            }
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

/// One path segment, query pair, or fragment part. An action word refuses it whatever
/// else it says; an order or money word is excused only by a view word in the same part.
fn part_looks_read_only(part: &str) -> bool {
    let lower = part.to_lowercase();
    // Separators are dropped so that `log-out`, `log_out` and `sign.out` read as one word.
    let compact: String = lower
        .chars()
        .filter(|c| !matches!(c, '-' | '_' | '.' | ' '))
        .collect();
    let action = mentions(&lower, ACTION_HREF)
        || mentions(&compact, ACTION_HREF)
        || has_whole_word(part, ACTION_WHOLE_WORD)
        || lower
            .split(|c: char| !c.is_alphanumeric())
            .any(|word| ACTION_HREF_AT_START.iter().any(|w| word.starts_with(w)));
    !action && (!mentions(&lower, MONEY_HREF) || mentions(&lower, VIEW_HREF))
}

/// Whether the address the tab ended up on (after any redirect) may be stayed on: it is
/// judged by the rules for a link's destination, and an address too long to check is not.
pub(crate) fn address_allowed(address: &str) -> bool {
    address.chars().count() <= MAX_HREF_CHARS && address_looks_read_only(address)
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
        .all(part_looks_read_only)
}

/// A short, stable fingerprint (FNV-1a) of a destination. Saved instead of the address
/// itself, so a token or an identifier in a link never reaches the data file; a link that
/// changes from visit to visit in a way the normalization misses simply stops matching,
/// and the user is asked again.
pub fn fingerprint(text: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in text.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    format!("h:{hash:016x}")
}

/// Words in a query name that mean the value belongs to one visit.
const VOLATILE_KEYS: &[&str] = &[
    "seq",
    "token",
    "session",
    "sessionid",
    "jsessionid",
    "phpsessid",
    "sessid",
    "sid",
    "time",
    "timestamp",
    "stamp",
    "nonce",
    "rand",
    "csrf",
    "ticket",
    "auth",
    "otp",
    "signature",
    "secret",
    "password",
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
    let kept: Vec<(String, String)> = query
        .split('&')
        .filter(|pair| !pair.is_empty())
        .filter_map(|pair| {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            // A name or value written with escapes (`%73ession`) is judged as the server reads it.
            // Whole words only: `timeframe` is not a time stamp and `author` is not `auth`.
            let words = tokens(&percent_decoded(key));
            let volatile = words.iter().any(|w| VOLATILE_KEYS.contains(&w.as_str()))
                || volatile_value(&percent_decoded(value));
            (!volatile).then(|| (key.to_owned(), value.to_owned()))
        })
        .collect();
    // The order is kept: with a repeated name a server may take the first or the last value.
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
  const rendered = (a) => { if (a.getClientRects().length === 0) return false; for (let e = a; e; e = e.parentElement) { const s = getComputedStyle(e); if (s.visibility === 'hidden' || s.display === 'none' || Number(s.opacity) === 0) return false; } return true; };
  const clean = (t) => String(t ?? '').replace(/\s+/g, ' ').trim();
  // A link to another site is trusted only when it is written out in full and in the plain form
  // `https://host/...`: the URL API quietly drops a user name, a default port, extra slashes,
  // back slashes, tabs and line breaks, so what is checked is the text as written.
  const plainHttps = (raw, url) => {
    // As written: no white space or control character anywhere, not even around it.
    const text = String(raw);
    if (text !== text.trim() || /[\u0000-\u001f\\]/.test(text) || url.username !== '' || url.password !== '' || url.port !== '') return false;
    const start = 'https://' + url.hostname;
    if (text.slice(0, start.length).toLowerCase() !== start) return false;
    const next = text.charAt(start.length);
    return next === '' || next === '/' || next === '?' || next === '#';
  };
  const seen = new Map();
  const shown = rendered;
  [...document.querySelectorAll('a')].forEach((a, i) => {
    if (!shown(a) || clean(a.innerText ?? a.textContent) !== wanted) return;
    const raw = a.getAttribute('href') ?? '';
    const scripted = raw === '' || raw.startsWith('#') || raw.toLowerCase().startsWith('javascript:');
    let sameSite = false;
    let href = '';
    let tooLong = false;
    let target = '';
    if (!scripted) {
      try {
        const url = new URL(raw, location.href);
        const web = url.protocol === 'https:' || url.protocol === 'http:';
        const own = web && url.origin === location.origin && url.username === '' && url.password === '';
        sameSite = own;
        target = own ? url.origin : (url.protocol === 'https:' && plainHttps(raw, url) ? url.origin : '');
        const full = url.pathname + url.search + url.hash;
        tooLong = full.length > 500;
        href = full.slice(0, 500);
      } catch (e) { href = ''; }
    }
    // Every matching link is looked at, but each distinct destination is listed once.
    const key = scripted + '|' + sameSite + '|' + tooLong + '|' + target + '|' + href;
    if (!seen.has(key) && seen.size < 21) seen.set(key, { index: i, href, same_site: sameSite, scripted, too_long: tooLong, origin: location.origin, target_origin: target });
  });
  return [...seen.values()];
})"#;

/// Goes to the address of the link at the given position, without clicking it:
/// a click would run whatever script the page attached to the link. Everything is
/// checked again in the page (the text, the site it goes to, and the exact path and query that
/// the Rust side accepted), and anything that differs is not followed.
pub(crate) const NAVIGATE: &str = r#"((wanted, index, expected, origin, target) => {
  const rendered = (a) => { if (a.getClientRects().length === 0) return false; for (let e = a; e; e = e.parentElement) { const s = getComputedStyle(e); if (s.visibility === 'hidden' || s.display === 'none' || Number(s.opacity) === 0) return false; } return true; };
  const clean = (t) => String(t ?? '').replace(/\s+/g, ' ').trim();
  // A link to another site is trusted only when it is written out in full and in the plain form
  // `https://host/...`: the URL API quietly drops a user name, a default port, extra slashes,
  // back slashes, tabs and line breaks, so what is checked is the text as written.
  const plainHttps = (raw, url) => {
    // As written: no white space or control character anywhere, not even around it.
    const text = String(raw);
    if (text !== text.trim() || /[\u0000-\u001f\\]/.test(text) || url.username !== '' || url.password !== '' || url.port !== '') return false;
    const start = 'https://' + url.hostname;
    if (text.slice(0, start.length).toLowerCase() !== start) return false;
    const next = text.charAt(start.length);
    return next === '' || next === '/' || next === '?' || next === '#';
  };
  if (!origin || origin !== location.origin) return 'gone';
  const a = document.querySelectorAll('a')[index];
  if (!a || !rendered(a)) return 'gone';
  if (clean(a.innerText ?? a.textContent) !== wanted) return 'gone';
  let url;
  const raw = a.getAttribute('href') ?? '';
  try { url = new URL(raw, location.href); } catch (e) { return 'gone'; }
  if (url.username !== '' || url.password !== '') return 'gone';
  if (url.origin !== location.origin && !(url.protocol === 'https:' && plainHttps(raw, url))) return 'gone';
  // The site the link goes to must be exactly the one the Rust side accepted.
  if ((url.protocol !== 'https:' && url.protocol !== 'http:') || !target || url.origin !== target) return 'gone';
  if (url.pathname + url.search + url.hash !== expected) return 'gone';
  location.assign(url.href);
  return 'navigated';
})"#;

/// Clicks one entry of a menu whose links are handled by the page's own script (`javascript:`,
/// `#`, or no address), for the sites the app has vetted. The entry is found by its exact text and
/// by the exact text of the heading link of the menu group it is in (the nearest enclosing list
/// item that has a link of its own), and it must be the only one: a menu entry that is not a
/// scripted link, opens another tab, or downloads is refused, so is anything on another site than
/// the one the Rust side accepted. It is the only place where a link is clicked, and it does
/// nothing else in the page. The texts are passed in as data, JSON-encoded.
pub(crate) const CLICK_MENU: &str = r#"((group, text, origin) => {
  const clean = (t) => String(t ?? '').replace(/\s+/g, ' ').trim();
  if (!origin || origin !== location.origin) return 'gone';
  const hits = [];
  for (const a of document.querySelectorAll('a')) {
    if (clean(a.textContent) !== text) continue;
    let item = a.closest('li');
    let owner = null;
    while (item) {
      const head = [...item.children].find((c) => c.tagName === 'A' && c !== a);
      if (head && clean(head.textContent) !== '') { owner = clean(head.textContent); break; }
      item = item.parentElement ? item.parentElement.closest('li') : null;
    }
    if (owner === group) hits.push(a);
  }
  if (hits.length === 0) return 'none';
  if (hits.length > 1) return 'ambiguous';
  const a = hits[0];
  const raw = a.getAttribute('href') ?? '';
  const scripted = raw === '' || raw.startsWith('#') || raw.toLowerCase().startsWith('javascript:');
  if (!scripted || (a.target && a.target !== '_self') || a.hasAttribute('download')) return 'refused';
  a.click();
  return 'clicked';
})"#;

/// The text of the first level-one heading of the page, as read after a click.
pub(crate) const FIRST_HEADING: &str = r#"(() => {
  const h = document.querySelector('h1');
  return h ? String(h.innerText ?? h.textContent ?? '').replace(/\s+/g, ' ').trim().slice(0, 100) : '';
})()"#;

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
        for mixed in [
            "Cancel order history",
            "Delete history",
            "注文取消履歴",
            "変更履歴",
            "Save balance",
            "Buy list",
        ] {
            assert_eq!(
                link_text_allowed(mixed),
                Err(Error::LinkRefused),
                "an action word is not excused by a view word: {mixed}"
            );
        }
        for out in [
            "Log out",
            "Log-out",
            "Sign-out",
            "Logoff",
            "ログオフ",
            "サインアウト",
        ] {
            assert_eq!(link_text_allowed(out), Err(Error::LinkRefused), "{out}");
        }
        for ja in ["買う", "売る", "今すぐ買う"] {
            assert_eq!(link_text_allowed(ja), Err(Error::LinkRefused), "{ja}");
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
            origin: "https://h.example".to_owned(),
            target_origin: "https://h.example".to_owned(),
        }
    }

    #[test]
    fn only_same_site_links_that_do_not_act_are_followed() {
        assert!(candidate("/ETGate/?_ControlID=WPLETacR001Control", true, false).allowed(&[]));
        assert!(candidate("/orderhistory/list", true, false).allowed(&[]));
        assert!(
            !candidate("/deletehistory", true, false).allowed(&[]),
            "delete in the same part"
        );
        assert!(!candidate("/page?savelist=1", true, false).allowed(&[]));
        assert!(
            candidate("/credit/list", true, false).allowed(&[]),
            "edit is not credit"
        );
        assert!(!candidate("/profile/editor", true, false).allowed(&[]));
        assert!(!candidate("/trade/order/confirm", true, false).allowed(&[]));
        assert!(!candidate("/transfer", true, false).allowed(&[]));
        assert!(
            !candidate("/transfer?account=123", true, false).allowed(&[]),
            "a word in the query does not make a transfer a view"
        );
        assert!(!candidate("/order/confirm?return=/portfolio", true, false).allowed(&[]));
        assert!(
            !candidate("/account/transfer", true, false).allowed(&[]),
            "account is not a sign of a view"
        );
        assert!(
            !candidate("/transfer/history", true, false).allowed(&[]),
            "an action word in one part is not excused by a view word in another"
        );
        assert!(!candidate("/history/delete", true, false).allowed(&[]));
        assert!(!candidate("/transfer/history/cancel", true, false).allowed(&[]));
        assert!(
            !candidate("/trans%66er", true, false).allowed(&[]),
            "escapes are decoded"
        );
        assert!(!candidate("/log%6fut", true, false).allowed(&[]));
        assert!(
            !candidate("/tr%2561nsfer", true, false).allowed(&[]),
            "double encoding"
        );
        assert!(!candidate("/page?action=cancel", true, false).allowed(&[]));
        for route in [
            "/account/change",
            "/profile/update",
            "/settings/save",
            "/user/edit",
            "/x/register",
            "/x/remove",
        ] {
            assert!(!candidate(route, true, false).allowed(&[]), "{route}");
        }
        assert!(
            !candidate("/#/transfer", true, false).allowed(&[]),
            "a route after #"
        );
        assert!(!candidate("/app#/history/delete", true, false).allowed(&[]));
        assert!(candidate("/app#/orderhistory", true, false).allowed(&[]));
        assert!(!candidate("/app#!/logout", true, false).allowed(&[]));
        assert!(candidate("/orderhistory/list", true, false).allowed(&[]));
        assert!(
            !candidate("/ETGate/", false, false).allowed(&[]),
            "another site"
        );
        assert!(
            !candidate("", false, true).allowed(&[]),
            "a link the page handles itself is never followed"
        );
        assert!(!candidate(&"a".repeat(600), true, false).allowed(&[]));
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
            "/a?b=2&a=1",
            "kept in order, and timestamps, long tokens, and session-like names are dropped",
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
        assert_eq!(
            normalize_href(
                "/a?%73ession=secret&b=2&%74oken=abc&c=%31%32%33%34%35%36%37%38%39%30%31%32%33"
            ),
            "/a?b=2",
            "escaped names and values are judged as the server reads them"
        );
        assert_eq!(normalize_href("/a?ticket=xyz&b=2&auth=q"), "/a?b=2");
        assert_eq!(
            normalize_href(
                "/h?timeframe=daily&author=p&sessionId=1&_SeqNo=2&JSESSIONID=3&access_token=4"
            ),
            "/h?timeframe=daily&author=p",
            "whole words only"
        );
        assert!(fingerprint("/a?b=2").starts_with("h:") && fingerprint("/a?b=2").len() == 18);
        assert_ne!(fingerprint("/a"), fingerprint("/b"));
        assert_eq!(fingerprint("/a"), fingerprint("/a"));
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

    fn cross(href: &str, target: &str) -> Candidate {
        Candidate {
            same_site: false,
            target_origin: target.to_owned(),
            ..candidate(href, true, false)
        }
    }

    #[test]
    fn another_host_is_followed_only_within_an_allowed_site_that_covers_the_start() {
        let sites = sites_covering("https://site3.example.co.jp", &["example.co.jp".to_owned()]);
        assert_eq!(sites, vec!["example.co.jp".to_owned()]);

        // The site's own other host: yes. Its path is judged like any other.
        assert!(cross("/list", "https://www.example.co.jp").allowed(&sites));
        assert!(cross("/list", "https://example.co.jp").allowed(&sites));
        assert!(!cross("/cancel", "https://www.example.co.jp").allowed(&sites));
        // Not within the site: a look-alike, another company, http, a port, another scheme.
        for target in [
            "https://badexample.co.jp",
            "https://example.co.jp.evil.test",
            "https://other.test",
            "http://www.example.co.jp",
            "https://www.example.co.jp:8443",
            "https://user@www.example.co.jp",
            "",
        ] {
            assert!(!cross("/list", target).allowed(&sites), "{target}");
        }
        // No allowed site: another host is refused whatever it is.
        assert!(!cross("/list", "https://www.example.co.jp").allowed(&[]));
    }

    #[test]
    fn an_allowed_site_that_does_not_cover_the_start_host_is_dropped() {
        let sites = ["example.co.jp".to_owned(), "other.test".to_owned()];
        assert_eq!(
            sites_covering("https://site3.example.co.jp", &sites),
            vec!["example.co.jp".to_owned()]
        );
        assert!(sites_covering("https://evil.test", &sites).is_empty());
        assert!(sites_covering("https://badexample.co.jp", &sites).is_empty());
        assert!(
            sites_covering("http://site3.example.co.jp", &sites).is_empty(),
            "not https"
        );
        assert!(sites_covering("https://site3.example.co.jp:444", &sites).is_empty());
    }

    #[test]
    fn the_same_path_on_another_host_is_another_destination() {
        let here = candidate("/list", true, false);
        let there = cross("/list", "https://www.example.co.jp");
        assert_ne!(here.reference(), there.reference());
        assert!(there.label().starts_with("https://www.example.co.jp/"));
    }

    #[test]
    fn only_a_plain_https_address_is_a_login_page() {
        assert!(login_url_ok("https://www.example.co.jp/login.html"));
        assert!(login_url_ok("https://www.example.co.jp"));
        for bad in [
            "http://www.example.co.jp/login",
            "https://user@www.example.co.jp/login",
            "https://www.example.co.jp:8443/login",
            "https://www.example.co.jp/lo gin",
            "https://www.example.co.jp/\\login",
            "https://www.example.co.jp/\nlogin",
            "javascript:alert(1)",
            "//www.example.co.jp/login",
            "",
        ] {
            assert!(!login_url_ok(bad), "{bad}");
        }
        assert!(host_within("www.example.co.jp", "example.co.jp"));
        assert!(!host_within("badexample.co.jp", "example.co.jp"));
    }
}
