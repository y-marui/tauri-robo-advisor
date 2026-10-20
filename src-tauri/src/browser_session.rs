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

/// What opening the login pages did.
#[derive(Debug, Default, Serialize)]
pub struct LoginPages {
    /// Sites whose login page was opened in a new tab.
    pub opened: Vec<String>,
    /// Sites that already had a tab open, so nothing was opened for them.
    pub already_open: Vec<String>,
    /// Sites of recipes for which no login page is known.
    pub unknown: Vec<String>,
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

/// What a recipe reads from the page it reached.
#[derive(Debug, Clone, Copy)]
pub enum ReadSource<'a> {
    /// The table whose first cell starts with `title` (see `pick_table`).
    Table {
        title: &'a str,
        merge: Option<usize>,
        follow: bool,
    },
    /// The cards under the one heading that starts with `heading`.
    Cards { heading: &'a str, item_depth: u8 },
}

/// Where a candidate that is a card list was found, to save it as a recipe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CardCandidate {
    pub heading: String,
    pub item_depth: u8,
}

/// A table (or a list of cards, shown the same way: one row per card) found on a page, described
/// so the user can pick the right one.
#[derive(Debug, Serialize)]
pub struct TableSummary {
    pub index: usize,
    pub rows: usize,
    pub columns: usize,
    pub label: String,
    /// Set when the candidate is a card list; `None` for a table.
    pub card: Option<CardCandidate>,
}

/// The candidates found on a page.
#[derive(Debug, Serialize)]
pub struct FetchedTables {
    pub tables: Vec<TableSummary>,
    /// The page has more headings than the app reads, so card lists below them are not offered.
    pub headings_cut: bool,
}

/// Starts an error message that means the browser tab is no longer where the recipes expect it:
/// the rest of the batch must not run.
pub const STOP_BATCH: &str = "[stop] ";

#[derive(Default)]
pub struct BrowserSession {
    endpoint: Mutex<Option<Endpoint>>,
    fetched: Mutex<Vec<Table>>,
    /// The address each tab had when a batch of recipes first read it. Every later recipe of the
    /// batch that reads the same tab must start on exactly this address; a tab that was navigated
    /// meanwhile stops the batch.
    pinned: Mutex<std::collections::HashMap<String, String>>,
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
    pub fn fetch_tables(&self, page_id: &str) -> Result<FetchedTables, String> {
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
            .into_all()
            .into_iter()
            .filter_map(|rows| tabular::from_rows(rows).ok())
            .collect();
        let summaries: Vec<TableSummary> = tables
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
                    // Plain words: the label is put in a text input (which would drop a line
                    // break) and is compared with the recipe's title as plain words.
                    .map(|c| {
                        c.split_whitespace()
                            .collect::<Vec<_>>()
                            .join(" ")
                            .chars()
                            .take(MAX_LABEL_CHARS)
                            .collect()
                    })
                    .unwrap_or_default(),
                card: None,
            })
            .collect();
        let mut tables = tables;
        let mut summaries = summaries;
        // The card lists of the page come after its tables. A failure to read the headings
        // (a page without any) only means there are none.
        let mut headings_cut = false;
        if let Ok(headings) = cdp.read_headings(&session) {
            headings_cut = headings.cut;
            for heading in headings.list {
                // The first depth at which the heading gives cards; the others would give the
                // same blocks again, split further.
                let found = (1..=3u8).find_map(|depth| {
                    let rows = cdp.read_cards(&session, &heading, depth).ok()?;
                    Some((depth, tabular::from_rows(rows).ok()?))
                });
                if let Some((item_depth, table)) = found {
                    summaries.push(TableSummary {
                        index: tables.len(),
                        rows: table.rows.len(),
                        columns: table.columns(),
                        label: heading.clone(),
                        card: Some(CardCandidate {
                            heading,
                            item_depth,
                        }),
                    });
                    tables.push(table);
                }
            }
        }
        *lock(&self.fetched)? = tables;
        Ok(FetchedTables {
            tables: summaries,
            headings_cut,
        })
    }

    /// Follows the recipe's links from the given page (each is a plain link that only
    /// shows another page), then reads the one table whose first cell starts with
    /// `title`. Nothing is typed or submitted.
    pub fn run_recipe(
        &self,
        page_id: &str,
        links: &[FetchLink],
        source: ReadSource,
        allowed_sites: &[String],
    ) -> Result<RecipeRead, String> {
        let mut cdp = self.connect()?;
        let page = cdp
            .pages()
            .map_err(|e| e.to_string())?
            .into_iter()
            .find(|p| p.target_id == page_id)
            .ok_or_else(|| browser::Error::NoPage.to_string())?;
        let session = cdp.attach(&page).map_err(|e| e.to_string())?;

        let start = cdp.current_href(&session).map_err(|e| e.to_string())?;
        {
            let mut pinned = lock(&self.pinned)?;
            match pinned.get(page_id) {
                // The same page, though a session identifier or the query may have changed.
                Some(href) if browser::page_identity(href) != browser::page_identity(&start) => {
                    // The batch stops here, so the old start is forgotten: fetching again starts
                    // from the page the tab is on now instead of failing the same way every time.
                    pinned.remove(page_id);
                    return Err(format!(
                        "{STOP_BATCH}The page changed while the recipes were running. Fetch again."
                    ));
                }
                Some(_) => {}
                None => {
                    pinned.insert(page_id.to_owned(), start.clone());
                }
            }
        }
        // Only the allowed sites that contain the host this recipe starts on count; a recipe can
        // reach the other hosts of its own site, never another company's.
        let sites = browser::sites_covering(&browser::origin_of(&start), allowed_sites);
        // A tab that already shows the recipe's table (it was left on the page the links lead to)
        // is read as it is: the links are not looked for, since that page may not have them.
        let links: &[FetchLink] = if self.shows_table(&mut cdp, &session, &source) {
            &[]
        } else {
            links
        };
        let mut references = Vec::with_capacity(links.len());
        let mut clicked = false;
        for (index, link) in links.iter().enumerate() {
            // A menu entry of a vetted site is clicked, not followed. Nothing is gone back from
            // afterwards (a page that was posted cannot be restored): the tab stays on the page
            // that was reached, and the batch takes that page as its own.
            if let (Some(group), Some(heading)) = (&link.menu, &link.heading) {
                match cdp.click_menu_link(&session, group, &link.text, heading, &sites) {
                    Ok(()) => {
                        clicked = true;
                        references.push(String::new());
                        continue;
                    }
                    // Nothing was clicked, so the page is where it was.
                    Err(error @ (browser::Error::LinkNotFound | browser::Error::LinkRefused)) => {
                        return Err(if clicked {
                            format!("{STOP_BATCH}Link {}: {error}", index + 1)
                        } else {
                            format!("Link {}: {error}", index + 1)
                        });
                    }
                    Err(error) => {
                        return Err(format!(
                            "{STOP_BATCH}Link {}: {error}; the page may have moved, so the remaining recipes were not run.",
                            index + 1
                        ));
                    }
                }
            }
            match cdp.follow_link(&session, &link.text, link.reference.as_deref(), &sites) {
                Ok(followed) => references.push(followed.reference),
                Err(browser::Error::LinkAmbiguous(choices)) => {
                    // Back to the page the recipe started from, so it can be run again.
                    cdp.go_back(&session, index, &start).map_err(|e| {
                        format!("{STOP_BATCH}Could not return to the starting page: {e}")
                    })?;
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
                    cdp.go_back(&session, index, &start).map_err(|e| {
                        format!("{STOP_BATCH}Could not return to the starting page: {e}")
                    })?;
                    return Ok(RecipeRead::LinkChanged { link: index });
                }
                Err(error) => {
                    // A timeout leaves it unknown whether the page moved, so the way back cannot
                    // be told: the whole batch stops. Otherwise go back to the start; if that
                    // fails the batch stops too, since the next recipe would read the wrong page.
                    if error == browser::Error::Timeout || error == browser::Error::WrongPage {
                        return Err(format!(
                            "{STOP_BATCH}Link {}: {error}; the page may have moved, so the remaining recipes were not run.",
                            index + 1
                        ));
                    }
                    return match cdp.go_back(&session, index, &start) {
                        Ok(()) => Err(format!("Link {}: {error}", index + 1)),
                        Err(back) => Err(format!(
                            "{STOP_BATCH}Link {}: {error}; could not return to the starting page: {back}",
                            index + 1
                        )),
                    };
                }
            }
        }

        if clicked {
            let now = cdp.current_href(&session).map_err(|e| e.to_string())?;
            lock(&self.pinned)?.insert(page_id.to_owned(), now);
        }
        let read = match source {
            // The table with the recipe's title may appear after the page has been quiet for a
            // while (a single-page app), so it is waited for, up to a limit; then the usual checks
            // decide.
            ReadSource::Table {
                title,
                merge,
                follow,
            } => {
                let title_words = squash(title);
                let has_title = |page: &browser::PageTables| {
                    page.leaf
                        .iter()
                        .chain(page.outer.iter())
                        .any(|t| title_matches(t, &title_words))
                };
                cdp.read_tables_until(&session, &has_title, TITLE_WAIT)
                    .map_err(|e| e.to_string())
                    .and_then(|page| pick_table_in_page(page, title, merge, follow))
            }
            ReadSource::Cards {
                heading,
                item_depth,
            } => read_cards_settled(&mut cdp, &session, heading, item_depth),
        };
        // Every run ends on the page it started from, so the next recipe starts there too. After a
        // click there is no way back, and the next recipe starts from the page that was reached.
        let restored = if clicked {
            Ok(())
        } else {
            cdp.go_back(&session, links.len(), &start)
        };
        // A failed restoration takes precedence: the batch must stop before the next recipe
        // reads the wrong page.
        restored.map_err(|e| format!("{STOP_BATCH}Could not return to the starting page: {e}"))?;
        let table = read?;
        Ok(RecipeRead::Table { table, references })
    }

    /// Whether the page the tab is on already has the table a table recipe reads. Only reads.
    fn shows_table(&self, cdp: &mut Cdp, session: &str, source: &ReadSource) -> bool {
        let ReadSource::Table { title, .. } = source else {
            return false;
        };
        let title_words = squash(title);
        cdp.read_tables(session).is_ok_and(|page| {
            page.leaf
                .iter()
                .chain(page.outer.iter())
                .any(|t| title_matches(t, &title_words))
        })
    }

    /// Forgets the page a batch started from; the next run pins its own.
    pub fn unpin(&self) {
        if let Ok(mut pinned) = self.pinned.lock() {
            pinned.clear();
        }
    }

    /// Opens the login page of each given site in a new tab, unless a tab of that site is already
    /// open. The browser only goes to the address; the person logs in by hand. Returns what was
    /// opened and what was already there.
    pub fn open_login_pages(&self, pages: &[(String, String)]) -> Result<LoginPages, String> {
        let mut cdp = self.connect()?;
        let open = cdp.pages().map_err(|e| e.to_string())?;
        let mut result = LoginPages::default();
        for (site, url) in pages {
            let there = open
                .iter()
                .any(|p| host_of(&p.url).is_some_and(|host| browser::host_within(&host, site)));
            if there {
                result.already_open.push(site.clone());
                continue;
            }
            if !browser::login_url_ok(url) {
                return Err("A login page address is not acceptable.".to_owned());
            }
            cdp.open_tab(url).map_err(|e| e.to_string())?;
            result.opened.push(site.clone());
        }
        Ok(result)
    }

    /// Sends the browser to the bitFlyer price page in a new tab, unless it is already open.
    /// Nothing is typed or clicked; this is the same kind of step as opening a login page.
    pub fn open_price_page(&self) -> Result<(), String> {
        debug_assert!(browser::login_url_ok(bitflyer::PRICE_PAGE_URL));
        let mut cdp = self.connect()?;
        let open = cdp.pages().map_err(|e| e.to_string())?;
        if !open.iter().any(|p| bitflyer::is_price_page(&p.url)) {
            cdp.open_tab(bitflyer::PRICE_PAGE_URL)
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// Reads the prices from the open tab of the price page (read-only).
    pub fn read_price_page(&self) -> Result<bitflyer::PagePrices, String> {
        let mut cdp = self.connect()?;
        let page = cdp
            .pages()
            .map_err(|e| e.to_string())?
            .into_iter()
            .find(|p| bitflyer::is_price_page(&p.url))
            .ok_or_else(|| {
                "The bitFlyer price page is not open. Press \"Open the price page\" first."
                    .to_owned()
            })?;
        let session = cdp.attach(&page).map_err(|e| e.to_string())?;
        let raw = cdp.read_tables(&session).map_err(|e| e.to_string())?;
        bitflyer::PagePrices::from_tables(&raw.into_all()).map_err(|e| {
            format!("Could not read the prices from the page ({e}). Wait until the page has loaded, then try again.")
        })
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

/// The longest a recipe waits for its table title to show up on the page it reached.
const TITLE_WAIT: std::time::Duration = std::time::Duration::from_secs(10);

/// Reads the cards under a heading, waiting (bounded) for a page that fills in late: the heading
/// or its cards may not be there yet, and the result is accepted only when a second read half a
/// second later gives exactly the same cards. Only reads.
fn read_cards_settled(
    cdp: &mut Cdp,
    session: &str,
    heading: &str,
    item_depth: u8,
) -> Result<Table, String> {
    let start = std::time::Instant::now();
    let mut last: Option<Vec<Vec<String>>> = None;
    loop {
        match cdp.read_cards(session, heading, item_depth) {
            Ok(rows) => {
                if last.as_ref() == Some(&rows) {
                    return tabular::from_rows(rows).map_err(|e| e.to_string());
                }
                last = Some(rows);
            }
            Err(browser::Error::CardsHeadingNotFound | browser::Error::CardsEmpty)
                if start.elapsed() < TITLE_WAIT =>
            {
                last = None;
            }
            Err(error) => return Err(error.to_string()),
        }
        if start.elapsed() >= TITLE_WAIT {
            return Err(browser::Error::Timeout.to_string());
        }
        std::thread::sleep(Duration::from_millis(500));
    }
}

/// Picks the table from the tables of a page. The tables that hold no other table are asked
/// first; only when none of them starts with the title, the outer tables (those that hold small
/// tables in their cells) are. So a page that was readable before is read exactly as before. The
/// two groups together are cut at the limit, and a page that reaches it may be missing tables.
fn pick_table_in_page(
    page: browser::PageTables,
    title: &str,
    merge: Option<usize>,
    follow: bool,
) -> Result<Table, String> {
    if follow && page.len() >= browser::MAX_TABLES {
        return Err("The page has too many tables to be sure the whole list was read.".to_owned());
    }
    let title_words = squash(title);
    let in_leaf = page.leaf.iter().any(|t| title_matches(t, &title_words));
    let raw = if in_leaf { page.leaf } else { page.outer };
    pick_table(raw, title, merge, follow)
}

/// Cells keep their line breaks and no-break spaces, and a heading may be broken into lines where
/// the page was written with a line break (or not, depending on how the text is read), so text is
/// compared without any whitespace at all.
fn squash(text: &str) -> String {
    text.split_whitespace().collect::<String>()
}

/// Whether the start of the first row, read as one line (its cells one after the other), is
/// `title` (already squashed). So a page whose tables all begin with the same cell can be told
/// apart by the cells that follow. A title that fits in the first cell matches as it always did.
fn title_matches(rows: &[Vec<String>], title: &str) -> bool {
    rows.first().is_some_and(|row| {
        let line: String = row.iter().map(|c| squash(c)).collect();
        line.starts_with(title)
    })
}

/// The table whose first cell starts with `title`. By default exactly one must match (more
/// than one, or none, is an error: the recipe should name the table clearly). With
/// `merge: Some(header_rows)` every matching table is read and their rows are put together:
/// a page often has one table per kind of account, all laid out the same way. The first table
/// is taken whole; of the others the first `header_rows` rows (the title and the headings) are
/// left out, so they do not show up as rows that were skipped.
///
/// With `follow`, exactly one table must match (the heading), and the rows of every table after
/// it that has the same number of columns are added: a page laid out as a heading table and then
/// one small table per holding.
fn pick_table(
    raw: Vec<Vec<Vec<String>>>,
    title: &str,
    merge: Option<usize>,
    follow: bool,
) -> Result<Table, String> {
    let title = squash(title);
    let matches_title = |rows: &Vec<Vec<String>>| title_matches(rows, &title);
    if follow {
        // The rows come from the tables after the heading: if the page was cut off at the limit,
        // holdings may be missing, and a result that is applied would delete them.
        if raw.len() >= browser::MAX_TABLES {
            return Err(
                "The page has too many tables to be sure the whole list was read.".to_owned(),
            );
        }
        let heads: Vec<usize> = (0..raw.len()).filter(|&i| matches_title(&raw[i])).collect();
        let head = match heads.as_slice() {
            [] => {
                return Err("No table on the page starts with the recipe's table title.".to_owned());
            }
            [one] => *one,
            many => {
                return Err(format!(
                    "{} tables on the page start with the recipe's table title; make the title more specific.",
                    many.len()
                ));
            }
        };
        let columns = |rows: &Vec<Vec<String>>| rows.iter().map(Vec::len).max().unwrap_or(0);
        let width = columns(&raw[head]);
        let mut rows = raw[head].clone();
        for table in &raw[head + 1..] {
            if columns(table) == width {
                rows.extend(table.iter().cloned());
            }
        }
        return tabular::from_rows(rows).map_err(|e| e.to_string());
    }
    let mut matching: Vec<Vec<Vec<String>>> =
        raw.into_iter().filter(|rows| matches_title(rows)).collect();
    let rows = match (matching.len(), merge) {
        (0, _) => {
            return Err("No table on the page starts with the recipe's table title.".to_owned());
        }
        (1, _) => matching.remove(0),
        (n, None) => {
            return Err(format!(
                "{n} tables on the page start with the recipe's table title; make the title more specific."
            ));
        }
        (_, Some(header_rows)) => {
            let mut all = matching.remove(0);
            for table in matching {
                all.extend(table.into_iter().skip(header_rows));
            }
            all
        }
    };
    tabular::from_rows(rows).map_err(|e| e.to_string())
}

/// For the tests of the presets: the table a recipe would read from these tables.
#[cfg(test)]
pub fn pick_table_for_test(raw: Vec<Vec<Vec<String>>>, title: &str, follow: bool) -> Table {
    pick_table(raw, title, None, follow).unwrap()
}

/// For the tests of the presets: the table a recipe would read from the two groups of tables of a
/// page.
#[cfg(test)]
pub fn pick_page_table_for_test(page: browser::PageTables, title: &str, follow: bool) -> Table {
    pick_table_in_page(page, title, None, follow).unwrap()
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

        let table = pick_table(raw, " Stocks (", None, false).unwrap();

        assert_eq!(table.rows[0][0], "Stocks (cash/taxable)");
    }

    #[test]
    fn a_title_can_run_over_the_next_cells_to_tell_tables_with_the_same_first_cell_apart() {
        let head = |cells: &[&str]| vec![cells.iter().map(|c| (*c).to_owned()).collect::<Vec<_>>()];
        let raw = vec![
            head(&["Name", "Account", "Total"]),
            head(&["Name", "Account", "Units", "Price"]),
            head(&["Name", "", "Units"]),
        ];

        // The first cell alone matches all three; the cells after it pick one.
        assert!(pick_table(raw.clone(), "Name", None, false).is_err());
        let table = pick_table(raw.clone(), "Name Account Units", None, false).unwrap();
        assert_eq!(table.rows[0].len(), 4);
        // Empty cells are left out when the row is read as one line.
        let table = pick_table(raw, "Name Units", None, false).unwrap();
        assert_eq!(table.rows[0].len(), 3);
    }

    #[test]
    fn an_outer_table_is_read_only_when_no_table_without_tables_in_it_matches() {
        let page = |leaf: Vec<Vec<Vec<String>>>, outer: Vec<Vec<Vec<String>>>| {
            browser::PageTables { leaf, outer }
        };
        let leaf = vec![rows("Name"), rows("Amount")];
        // The small table inside the outer table's cell is a leaf table of its own and is
        // not read as the holdings.
        let outer = vec![vec![
            vec!["Name".to_owned(), "Units".to_owned()],
            vec!["A".to_owned(), "1".to_owned()],
        ]];

        // A table that holds no table and matches wins, as it always did.
        let table =
            pick_table_in_page(page(leaf.clone(), outer.clone()), "Name", None, false).unwrap();
        assert_eq!(table.rows.last().unwrap()[0], "a");
        // Only when none matches is an outer table asked.
        let table = pick_table_in_page(
            page(vec![rows("Amount")], outer.clone()),
            "Name Units",
            None,
            false,
        )
        .unwrap();
        assert_eq!(table.rows.last().unwrap()[0], "A");
        // No match in either group is the same error as before.
        assert!(
            pick_table_in_page(page(vec![rows("Amount")], outer), "Nope", None, false).is_err()
        );
    }

    #[test]
    fn the_two_groups_together_are_held_to_the_table_limit_for_following_tables() {
        let one_row =
            |cells: &[&str]| vec![cells.iter().map(|c| (*c).to_owned()).collect::<Vec<_>>()];
        let mut leaf = vec![one_row(&["Code", "Account"])];
        while leaf.len() < browser::MAX_TABLES - 1 {
            leaf.push(one_row(&["A", "Taxable"]));
        }
        let page = |outer: Vec<Vec<Vec<String>>>| browser::PageTables {
            leaf: leaf.clone(),
            outer,
        };

        assert!(pick_table_in_page(page(vec![]), "Code", None, true).is_ok());
        assert!(pick_table_in_page(page(vec![one_row(&["X"])]), "Code", None, true).is_err());
    }

    #[test]
    fn whitespace_inside_a_heading_does_not_matter_to_the_title() {
        let raw = vec![vec![vec![
            "Name".to_owned(),
            "Account\nType".to_owned(),
            "Price (yen)\u{a0}Change".to_owned(),
        ]]];

        for title in [
            "Name Account Type Price (yen) Change",
            "NameAccountTypePrice(yen)Change",
        ] {
            assert!(
                pick_table(raw.clone(), title, None, false).is_ok(),
                "{title}"
            );
        }
    }

    #[test]
    fn line_breaks_and_no_break_spaces_in_the_first_cell_do_not_hide_the_title() {
        let raw = vec![rows("Menu"), rows("Stocks\n(cash/\u{a0}taxable)")];

        let table = pick_table(raw, "Stocks (cash/ taxable)", None, false).unwrap();

        assert_eq!(table.rows[0][0], "Stocks\n(cash/\u{a0}taxable)");
    }

    #[test]
    fn no_match_and_several_matches_are_errors() {
        assert!(pick_table(vec![rows("Menu")], "Stocks", None, false).is_err());
        let error = pick_table(
            vec![rows("Stocks A"), rows("Stocks B")],
            "Stocks",
            None,
            false,
        )
        .unwrap_err();
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

    #[test]
    fn every_matching_table_can_be_read_and_put_together() {
        let table = |title: &str, name: &str| {
            vec![
                vec![title.to_owned(), String::new()],
                vec!["Quantity".to_owned(), "Price".to_owned()],
                vec![name.to_owned(), String::new()],
                vec!["1".to_owned(), "2".to_owned()],
            ]
        };
        let raw = vec![
            table("Stocks (taxable)", "A"),
            table("Menu", "X"),
            table("Stocks (tax-free)", "B"),
        ];

        // Several match: an error unless the recipe asks for all of them.
        assert!(pick_table(raw.clone(), "Stocks (", None, false).is_err());
        let merged = pick_table(raw, "Stocks (", Some(2), false).unwrap();

        // The first table whole, the second without its title and heading rows.
        let first_cells: Vec<&str> = merged.rows.iter().map(|r| r[0].as_str()).collect();
        assert_eq!(
            first_cells,
            vec!["Stocks (taxable)", "Quantity", "A", "1", "B", "1"]
        );
    }

    #[test]
    fn the_tables_after_a_heading_table_are_its_rows() {
        let one_row =
            |cells: &[&str]| vec![cells.iter().map(|c| (*c).to_owned()).collect::<Vec<_>>()];
        let raw = vec![
            one_row(&["Total", "1"]),
            one_row(&["Code / Name", "Account", "Quantity"]),
            one_row(&["Search"]),
            one_row(&["A", "Taxable", "10"]),
            one_row(&["B", "Taxable", "20"]),
            one_row(&["Footer", "x"]),
        ];

        let table = pick_table(raw.clone(), "Code", None, true).unwrap();

        // The heading, then every later table with three columns (the one-column and
        // two-column tables between and after are not rows).
        let first: Vec<&str> = table.rows.iter().map(|r| r[0].as_str()).collect();
        assert_eq!(first, vec!["Code / Name", "A", "B"]);
        // Without the title, or with two heading tables, it is an error as before.
        assert!(pick_table(raw.clone(), "Nope", None, true).is_err());
        let mut twice = raw;
        twice.push(one_row(&["Code / Name 2", "Account", "Quantity"]));
        assert!(pick_table(twice, "Code", None, true).is_err());
    }

    #[test]
    fn a_page_cut_off_at_the_table_limit_is_not_accepted_for_following_tables() {
        let one_row =
            |cells: &[&str]| vec![cells.iter().map(|c| (*c).to_owned()).collect::<Vec<_>>()];
        let mut raw = vec![one_row(&["Code", "Account"])];
        while raw.len() < browser::MAX_TABLES {
            raw.push(one_row(&["A", "Taxable"]));
        }

        assert!(pick_table(raw.clone(), "Code", None, true).is_err());
        raw.pop();
        assert!(pick_table(raw, "Code", None, true).is_ok());
    }
}
