//! Finds a Chromium-based browser (Chrome, Edge, or Brave), starts it with a
//! profile of its own, and reads pages through the DevTools protocol.
//!
//! The user logs in by hand in that browser. This crate never types, clicks, or
//! stores a credential: it only asks the browser which pages are open and runs
//! a fixed read-only script in one of them. The connection is on this computer
//! only, and no driver program (chromedriver) is needed.

mod cdp;
mod detect;
mod error;
mod launch;

pub use cdp::{Cdp, Page};
pub use detect::{Found, Kind, Os, candidate_paths, find_browsers, find_with};
pub use error::Error;
pub use launch::{Endpoint, parse_active_port, start};

#[cfg(test)]
mod tests;

#[cfg(test)]
fn cdp_script_for_tests() -> &'static str {
    cdp::READ_TABLES
}
