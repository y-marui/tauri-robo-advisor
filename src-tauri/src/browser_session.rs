//! The browser the user opened to fetch tables from a page.
//!
//! The user logs in by hand in a browser the app starts with a profile of its
//! own; this module only asks which pages are open and reads their tables. It
//! never types, clicks, or keeps a credential. The browser to start is chosen
//! from the ones found on this computer, never from a path the frontend sends.

use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use browser::{Cdp, Endpoint, Kind};
use portfolio::FetchLink;
use serde::Serialize;
use tabular::Table;

/// How long to wait for a browser to open its debugging connection.
const START_WAIT: Duration = Duration::from_secs(20);
const MAX_TITLE_CHARS: usize = 80;
const MAX_LABEL_CHARS: usize = 40;

#[derive(Debug, Serialize)]
pub struct BrowserInfo {
    pub kind: Kind,
}

/// An open tab, described without its full address (which can carry a session
/// number): only the host and the title.
#[derive(Debug, Serialize)]
pub struct PageInfo {
    pub id: String,
    pub host: String,
    pub title: String,
}

/// One of several same-named links: what to save to find it again, and where it goes.
#[derive(Debug, Serialize)]
pub struct LinkChoiceView {
    pub reference: String,
    pub label: String,
}

/// What reading a recipe's page gave.
#[derive(Debug)]
pub enum RecipeRead {
    /// The table, and where each link went (to save when it was not saved yet).
    Table {
        table: Table,
        references: Vec<String>,
    },
    /// Several different links have this text; the user has to pick one.
    NeedsChoice {
        link: usize,
        choices: Vec<LinkChoiceView>,
    },
    /// A saved link no longer exists: the page has changed.
    LinkChanged { link: usize },
}

/// A table found on a page, described so the user can pick the right one.
#[derive(Debug, Serialize)]
pub struct TableSummary {
    pub index: usize,
    pub rows: usize,
    pub columns: usize,
    pub label: String,
}

#[derive(Default)]
pub struct BrowserSession {
    endpoint: Mutex<Option<Endpoint>>,
    fetched: Mutex<Vec<Table>>,
}

impl BrowserSession {
    pub fn installed() -> Vec<BrowserInfo> {
        browser::find_browsers()
            .into_iter()
            .map(|f| BrowserInfo { kind: f.kind })
            .collect()
    }

    /// Starts the browser (or reuses one already running with this profile).
    pub fn open(&self, kind: Kind, profile: &Path) -> Result<(), String> {
        check_profile(profile)?;
        let found = browser::find_browsers()
            .into_iter()
            .find(|f| f.kind == kind)
            .ok_or_else(|| browser::Error::NoBrowser.to_string())?;
        let endpoint =
            browser::start(&found.path, profile, START_WAIT).map_err(|e| e.to_string())?;
        *lock(&self.endpoint)? = Some(endpoint);
        Ok(())
    }

    fn connect(&self) -> Result<Cdp, String> {
        let endpoint = lock(&self.endpoint)?
            .clone()
            .ok_or_else(|| "Open the browser first.".to_owned())?;
        Cdp::connect(&endpoint).map_err(|e| e.to_string())
    }

    /// The web pages that are open.
    pub fn pages(&self) -> Result<Vec<PageInfo>, String> {
        let mut cdp = self.connect()?;
        let pages = cdp.pages().map_err(|e| e.to_string())?;
        Ok(pages
            .into_iter()
            .filter_map(|page| {
                Some(PageInfo {
                    id: page.target_id,
                    host: host_of(&page.url)?,
                    title: page.title.chars().take(MAX_TITLE_CHARS).collect(),
                })
            })
            .collect())
    }

    /// Reads the tables of one page and keeps them for `take_table`.
    pub fn fetch_tables(&self, page_id: &str) -> Result<Vec<TableSummary>, String> {
        let mut cdp = self.connect()?;
        let page = cdp
            .pages()
            .map_err(|e| e.to_string())?
            .into_iter()
            .find(|p| p.target_id == page_id)
            .ok_or_else(|| browser::Error::NoPage.to_string())?;
        let session = cdp.attach(&page).map_err(|e| e.to_string())?;
        let raw = cdp.read_tables(&session).map_err(|e| e.to_string())?;

        let tables: Vec<Table> = raw
            .into_iter()
            .filter_map(|rows| tabular::from_rows(rows).ok())
            .collect();
        let summaries = tables
            .iter()
            .enumerate()
            .map(|(index, table)| TableSummary {
                index,
                rows: table.rows.len(),
                columns: table.columns(),
                label: table
                    .rows
                    .first()
                    .and_then(|row| row.iter().find(|c| !c.trim().is_empty()))
                    .map(|c| c.chars().take(MAX_LABEL_CHARS).collect())
                    .unwrap_or_default(),
            })
            .collect();
        *lock(&self.fetched)? = tables;
        Ok(summaries)
    }

    /// Follows the recipe's links from the given page (each is a plain link that only
    /// shows another page), then reads the one table whose first cell starts with
    /// `title`. Nothing is typed or submitted.
    pub fn run_recipe(
        &self,
        page_id: &str,
        links: &[FetchLink],
        title: &str,
    ) -> Result<RecipeRead, String> {
        let mut cdp = self.connect()?;
        let page = cdp
            .pages()
            .map_err(|e| e.to_string())?
            .into_iter()
            .find(|p| p.target_id == page_id)
            .ok_or_else(|| browser::Error::NoPage.to_string())?;
        let session = cdp.attach(&page).map_err(|e| e.to_string())?;

        let mut references = Vec::with_capacity(links.len());
        for (index, link) in links.iter().enumerate() {
            match cdp.follow_link(&session, &link.text, link.reference.as_deref()) {
                Ok(followed) => references.push(followed.reference),
                Err(browser::Error::LinkAmbiguous(choices)) => {
                    let choices = choices
                        .into_iter()
                        .map(|c| LinkChoiceView {
                            reference: c.reference,
                            label: c.label,
                        })
                        .collect();
                    return Ok(RecipeRead::NeedsChoice {
                        link: index,
                        choices,
                    });
                }
                Err(browser::Error::LinkChanged) => {
                    return Ok(RecipeRead::LinkChanged { link: index });
                }
                Err(error) => return Err(format!("Link {}: {error}", index + 1)),
            }
        }

        let raw = cdp.read_tables(&session).map_err(|e| e.to_string())?;
        let table = pick_table(raw, title)?;
        Ok(RecipeRead::Table { table, references })
    }

    /// The table the user chose from the last fetch.
    pub fn take_table(&self, index: usize) -> Result<Table, String> {
        lock(&self.fetched)?
            .get(index)
            .cloned()
            .ok_or_else(|| "That table is no longer available.".to_owned())
    }

    /// Asks the browser to quit and forgets what was fetched. A browser that is
    /// already gone is not an error.
    pub fn close(&self) -> Result<(), String> {
        lock(&self.fetched)?.clear();
        let Some(endpoint) = lock(&self.endpoint)?.take() else {
            return Ok(());
        };
        if let Ok(cdp) = Cdp::connect(&endpoint) {
            let _ = cdp.close();
        }
        Ok(())
    }
}

fn lock<T>(mutex: &Mutex<T>) -> Result<std::sync::MutexGuard<'_, T>, String> {
    mutex
        .lock()
        .map_err(|_| "The data is temporarily unavailable.".to_owned())
}

/// The one table whose first cell starts with `title`. More than one (or none)
/// is an error: the recipe should name the table clearly.
fn pick_table(raw: Vec<Vec<Vec<String>>>, title: &str) -> Result<Table, String> {
    let title = title.trim();
    let mut matching: Vec<Table> = raw
        .into_iter()
        .filter_map(|rows| tabular::from_rows(rows).ok())
        .filter(|table| {
            table
                .rows
                .first()
                .and_then(|row| row.iter().find(|c| !c.trim().is_empty()))
                .is_some_and(|cell| cell.trim().starts_with(title))
        })
        .collect();
    match matching.len() {
        0 => Err("No table on the page starts with the recipe's table title.".to_owned()),
        1 => Ok(matching.remove(0)),
        n => Err(format!(
            "{n} tables on the page start with the recipe's table title; make the title more specific."
        )),
    }
}

/// The profile folder is chosen by the user on their own computer, but it must
/// be an absolute path without `..`, so it cannot mean something else.
fn check_profile(profile: &Path) -> Result<(), String> {
    if !profile.is_absolute() || profile.components().any(|c| c == Component::ParentDir) {
        return Err("The profile folder must be a full path.".to_owned());
    }
    Ok(())
}

/// The host of a web address, or `None` for anything that is not http(s).
fn host_of(url: &str) -> Option<String> {
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;
    let authority = rest.split(['/', '?', '#']).next()?;
    let host = authority.rsplit('@').next()?;
    (!host.is_empty()).then(|| host.chars().take(100).collect())
}

/// Where the dedicated profile goes when the user did not choose a folder.
pub fn default_profile(app_data: &Path) -> PathBuf {
    app_data.join("browser-profile")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_host_of_a_web_page_is_shown() {
        assert_eq!(
            host_of("https://site.example/a/b?seq=123#x").as_deref(),
            Some("site.example")
        );
        assert_eq!(
            host_of("http://user@site.example:8080/").as_deref(),
            Some("site.example:8080")
        );
        assert_eq!(host_of("chrome://settings/"), None);
        assert_eq!(host_of("https:///x"), None);
        assert_eq!(host_of("about:blank"), None);
    }

    fn rows(first: &str) -> Vec<Vec<String>> {
        vec![
            vec![first.to_owned(), String::new()],
            vec!["a".to_owned(), "1".to_owned()],
        ]
    }

    #[test]
    fn the_table_is_picked_by_the_start_of_its_first_cell() {
        let raw = vec![rows("Menu"), rows("Stocks (cash/taxable)"), rows("Funds")];

        let table = pick_table(raw, " Stocks (").unwrap();

        assert_eq!(table.rows[0][0], "Stocks (cash/taxable)");
    }

    #[test]
    fn no_match_and_several_matches_are_errors() {
        assert!(pick_table(vec![rows("Menu")], "Stocks").is_err());
        let error = pick_table(vec![rows("Stocks A"), rows("Stocks B")], "Stocks").unwrap_err();
        assert!(error.starts_with("2 tables"));
    }

    #[test]
    fn the_profile_must_be_a_full_path_without_parent_steps() {
        let full = std::env::temp_dir().join("profile");
        assert!(check_profile(&full).is_ok());
        assert!(check_profile(Path::new("relative/profile")).is_err());
        assert!(check_profile(&full.join("..").join("elsewhere")).is_err());
    }

    #[test]
    fn closing_without_a_browser_is_fine_and_forgets_the_tables() {
        let session = BrowserSession::default();
        *session.fetched.lock().unwrap() =
            vec![tabular::from_rows(vec![vec!["a".to_owned()]]).unwrap()];

        assert!(session.close().is_ok());
        assert!(session.take_table(0).is_err());
        assert!(session.pages().is_err(), "needs an open browser");
    }
}
