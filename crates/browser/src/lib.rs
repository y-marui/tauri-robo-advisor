//! Finds a Chromium-based browser (Chrome, Edge, or Brave), starts it with a
//! profile of its own, and reads pages through the DevTools protocol.
//!
//! The user logs in by hand in that browser. This crate never types or stores a
//! credential. It asks the browser which pages are open, runs fixed read-only
//! scripts in one of them, and may follow a plain link to show another page
//! (see `links`), or, on a vetted site that works by script menus, click one menu entry
//! and check the heading of the page it opens (`Cdp::click_menu_link`); it never types or
//! submits anything. The connection is on this computer
//! only, and no driver program (chromedriver) is needed.

mod cdp;
mod detect;
mod error;
mod launch;
mod links;

pub use cdp::{Cdp, Followed, MAX_TABLES, Page, PageTables};
pub use detect::{Found, Kind, Os, candidate_paths, find_browsers, find_with};
pub use error::{Error, LinkChoice};
pub use launch::{Endpoint, parse_active_port, start};
pub use links::{
    fingerprint, host_within, link_text_allowed, login_url_ok, normalize_href, origin_of,
    page_identity, sites_covering,
};

#[cfg(test)]
mod tests;

#[cfg(test)]
fn cdp_script_for_tests() -> &'static str {
    cdp::READ_TABLES
}
