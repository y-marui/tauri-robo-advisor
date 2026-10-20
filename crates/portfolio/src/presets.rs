//! Fetch recipes that ship with the app, as data.
//!
//! A preset is an ordinary recipe (the same fields the user can write) for a page layout that
//! is known, so a user does not have to author it. There is no brokerage-specific code: when a
//! page changes, the fix is a change to the data here (or to the user's copy of the recipe).
//! What a preset names was checked against the structure of the real page (headings and shape
//! only); the names below are the labels the pages themselves show.

use serde::Serialize;

use crate::{AccountId, Error, FetchLink, ImportMapping, KindRead, KindRule, Recipe};

/// How a preset reads its table: the columns and rows of a record. Counted from 0.
struct Layout {
    source: &'static str,
    name_column: usize,
    /// The evaluation column, or the quantity column when `price_column` is set.
    value_column: usize,
    price_column: Option<usize>,
    price_unit: u64,
    /// Which value of the price cell to read when the cell holds several (the price, then its
    /// change); the whole cell when `None`.
    price_part: Option<usize>,
    header_rows: usize,
    rows_per_record: usize,
    name_row: usize,
    value_row: usize,
    loose_numbers: bool,
    quantity_column: Option<usize>,
    max_records: Option<usize>,
    /// For a table that lists several kinds of holding together: the cell of the row that says
    /// the kind, and how each kind is read (a kind that is not listed is left out and reported).
    kind_column: Option<usize>,
    kinds: &'static [(&'static str, KindRead)],
}

/// The click on a menu entry that a preset makes (see `FetchLink::menu`).
struct Menu {
    group: &'static str,
    heading: &'static str,
}

/// A recipe for a known page, waiting for an account.
pub struct Preset {
    pub id: &'static str,
    pub label: &'static str,
    /// Where to start, and what it reads, in a sentence for the screen.
    pub description: &'static str,
    links: &'static [&'static str],
    /// For a site that works by script menus: the menu group of the one link (which is then clicked,
    /// not followed) and the heading the page must show after the click.
    menu: Option<Menu>,
    table_starts_with: &'static str,
    /// Sites whose other hosts the recipe may follow links to (the pages of one company are often
    /// spread over several hosts).
    allowed_sites: &'static [&'static str],
    /// The site of the page it reads, and whether each holding is a table of its own after the
    /// heading table (otherwise every table that starts with the title is read).
    start_site: &'static str,
    following_tables: bool,
    /// The page where the person logs in (the app only opens it; nothing is typed). One per
    /// company: it belongs to the preset of the company's first entry.
    login_url: &'static str,
    /// The currency of the values the page shows, when it is known (a page that shows every
    /// valuation in yen); empty means the currency of the product a name is linked to. A name
    /// linked to a product in another currency is then reported, not imported.
    currency: &'static str,
    layout: Layout,
}

/// What the screen needs to list a preset.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PresetInfo {
    pub id: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    /// What identifies the recipe of this preset (the same fields `AddPresetRecipe` uses to
    /// find an existing one), so the screen can tell which recipe came from which preset.
    pub table_starts_with: &'static str,
    pub source: &'static str,
    pub links: &'static [&'static str],
}

/// SBI Securities: every table is a title row, a heading row, then two rows per holding (a row
/// with the name, then a row with the quantity, the cost, the price, and the profit; only the
/// quantity and the price are read).
const fn sbi_layout(price_unit: u64) -> Layout {
    Layout {
        source: "SBI",
        name_column: 0,
        value_column: 0,
        price_column: Some(2),
        price_unit,
        price_part: None,
        header_rows: 2,
        rows_per_record: 2,
        name_row: 0,
        value_row: 1,
        loose_numbers: false,
        quantity_column: None,
        max_records: None,
        kind_column: None,
        kinds: &[],
    }
}

const PRESETS: &[Preset] = &[
    Preset {
        id: "sbi-domestic-stocks",
        label: "SBI Securities: domestic stocks",
        description: "Starts from the top page of SBI Securities (after logging in): follows \"保有資産評価\" and reads the domestic stock tables of every account type (quantity x current price).",
        links: &["保有資産評価"],
        menu: None,
        table_starts_with: "株式（現物/",
        allowed_sites: &[],
        start_site: "sbisec.co.jp",
        following_tables: false,
        login_url: "https://www.sbisec.co.jp/ETGate",
        currency: "",
        layout: sbi_layout(1),
    },
    Preset {
        id: "sbi-funds",
        label: "SBI Securities: investment trusts",
        description: "Starts from the top page of SBI Securities (after logging in): follows \"保有資産評価\" and reads the investment trust tables of every account type (units x price per 10,000 units).",
        links: &["保有資産評価"],
        menu: None,
        table_starts_with: "投資信託（金額/",
        allowed_sites: &[],
        start_site: "sbisec.co.jp",
        following_tables: false,
        login_url: "https://www.sbisec.co.jp/ETGate",
        currency: "",
        layout: sbi_layout(10_000),
    },
    // Monex Securities, the page 保有残高・口座管理. The page the person lands on after logging in
    // links to it with a plain same-host link, 残高照会 (the other links to the page there are
    // hidden menu entries or have the same text more than once). Every table of the page begins
    // with the same heading cell (name), so the title runs over the first cells of the heading row
    // to pick the investment trust table. It is a heading row and then one row per holding: name,
    // account type, price, one more column, units, cost, evaluation, profit, and a last column.
    // Only the units and the price are read, and the value is the units times the price per
    // 10,000 units (as for SBI): the units from their column, the price from the price column (the
    // first value of its cell, which also shows the day's change).
    Preset {
        id: "monex-funds",
        label: "Monex Securities: investment trusts",
        description: "Starts from the top page of Monex Securities (after logging in): follows \"残高照会\" and reads the investment trust table (units x price per 10,000 units).",
        links: &["残高照会"],
        menu: None,
        table_starts_with: "銘柄 口座区分預り区分 基準価額（円）前日比 分配金の取扱い 保有数（口）",
        allowed_sites: &["monex.co.jp"],
        start_site: "monex.co.jp",
        following_tables: false,
        login_url: "https://www.monex.co.jp/",
        currency: "",
        layout: Layout {
            source: "Monex",
            name_column: 0,
            value_column: 4,
            price_column: Some(2),
            price_unit: 10_000,
            price_part: Some(0),
            header_rows: 1,
            rows_per_record: 1,
            name_row: 0,
            value_row: 0,
            loose_numbers: false,
            quantity_column: None,
            max_records: None,
            kind_column: None,
            kinds: &[],
        },
    },
    // Rakuten Securities, the page 保有商品一覧-すべて (all holdings): the top page after logging in
    // links to it with a plain link on the same host (the pages of the company are spread over
    // several hosts). The holdings of every kind are one table, found by its heading cells (its
    // cells are outer-table cells: some hold small tables, see `browser::PageTables`). After the
    // heading row, a holding is one visible row: the kind, name, account type, units, cost, price,
    // and a cell with the evaluation (first line) and the profit. The page also has a summary
    // table by kind with hidden detail rows, which does not start with the same cells. Only the
    // units and the price are read, and the value is the units times the price per 10,000 units
    // for an investment trust row; a domestic stock row (its price is per share) reads the
    // evaluation (the first line of the evaluation cell) and records the units. The rows are told
    // apart by the kind cell, and a row of any other kind is left out and reported (#88);
    // "9,999円" and "9,999口" are numbers with a unit. The page shows amounts in yen; a holding in a
    // foreign currency is not covered. The holdings are all on the page when it loads (no paging, scrolling, or "more"
    // button), and the page shows no count to check them against (its total is in a hidden row,
    // which is not read), so the person checks the rows shown before applying them.
    Preset {
        id: "rakuten-all",
        label: "Rakuten Securities: all holdings",
        description: "Starts from the top page of Rakuten Securities (after logging in): follows \"保有商品一覧-すべて\" and reads the table of all holdings (units x price per 10,000 units). Investment trusts (units x price per 10,000 units) and domestic stocks (the evaluation) in yen are covered; other kinds are left out and reported, and so are holdings in a foreign currency.",
        links: &["保有商品一覧-すべて"],
        menu: None,
        table_starts_with: "種別 銘柄 口座",
        allowed_sites: &["rakuten-sec.co.jp"],
        start_site: "rakuten-sec.co.jp",
        following_tables: false,
        login_url: "https://www.rakuten-sec.co.jp/",
        currency: "JPY",
        layout: Layout {
            source: "Rakuten",
            name_column: 1,
            value_column: 3,
            price_column: Some(5),
            // Each kind sets its own unit (see `kinds`).
            price_unit: 1,
            price_part: None,
            header_rows: 1,
            rows_per_record: 1,
            name_row: 0,
            value_row: 0,
            loose_numbers: true,
            quantity_column: None,
            max_records: None,
            kind_column: Some(0),
            kinds: &[
                (
                    "投資信託",
                    KindRead::QuantityTimesPrice { price_unit: 10_000 },
                ),
                (
                    "国内株式",
                    KindRead::Evaluation {
                        column: 6,
                        part: Some(0),
                    },
                ),
            ],
        },
    },
    // Daiwa Connect Securities: its pages work by form posts and script links, so the menu entry
    // 国内株式 > 保有銘柄一覧 is clicked (the site is vetted) and the page must then show the heading
    // 現物取引保有銘柄一覧. The page is a heading
    // table (code / name, account, quantity, sellable quantity, cost, price, evaluation, profit,
    // trade) and then one one-row table per holding. The evaluation is read from its column and
    // the quantity is recorded next to it ("10株", "1,234円": numbers with a unit).
    Preset {
        id: "daiwa-connect-stocks",
        label: "Daiwa Connect Securities: domestic stocks",
        description: "After logging in to Daiwa Connect Securities, clicks the menu 国内株式 > 保有銘柄一覧 (the list of holdings, 現物取引保有銘柄一覧) and reads every holding row (evaluation and quantity).",
        links: &["保有銘柄一覧"],
        menu: Some(Menu {
            group: "国内株式",
            heading: "現物取引保有銘柄一覧",
        }),
        table_starts_with: "コード",
        allowed_sites: &["connect-sec.co.jp"],
        start_site: "connect-sec.co.jp",
        following_tables: true,
        login_url: "https://www.connect-sec.co.jp/jumppages/login.html",
        currency: "",
        layout: Layout {
            source: "Daiwa Connect",
            name_column: 0,
            value_column: 6,
            price_column: None,
            price_unit: 1,
            price_part: None,
            header_rows: 1,
            rows_per_record: 1,
            name_row: 0,
            value_row: 0,
            loose_numbers: true,
            quantity_column: Some(2),
            max_records: None,
            kind_column: None,
            kinds: &[],
        },
    },
    // Daiwa Connect Securities, the page 取引余力・預り資産 (the menu entry 資産照会 > 取引余力・預り資産
    // is clicked, and the page must show that heading): a table of label and
    // amount pairs, 金銭残高 (the yen balance) first and then the valuations of stocks, funds, and
    // foreign stocks and the total, and a second table (buyable and withdrawable amounts). Only
    // the first pair is read; its label is the name to link to a product (a yen product).
    Preset {
        id: "daiwa-connect-cash",
        label: "Daiwa Connect Securities: cash (yen)",
        description: "After logging in to Daiwa Connect Securities, clicks the menu 資産照会 > 取引余力・預り資産 and reads the yen balance (金銭残高).",
        links: &["取引余力・預り資産"],
        menu: Some(Menu {
            group: "資産照会",
            heading: "取引余力・預り資産",
        }),
        table_starts_with: "金銭残高",
        allowed_sites: &["connect-sec.co.jp"],
        start_site: "connect-sec.co.jp",
        following_tables: false,
        login_url: "https://www.connect-sec.co.jp/jumppages/login.html",
        currency: "",
        layout: Layout {
            source: "Daiwa Connect",
            name_column: 0,
            value_column: 1,
            price_column: None,
            price_unit: 1,
            price_part: None,
            header_rows: 0,
            rows_per_record: 1,
            name_row: 0,
            value_row: 0,
            loose_numbers: false,
            quantity_column: None,
            max_records: Some(1),
            kind_column: None,
            kinds: &[],
        },
    },
];

/// The presets, in the order they are shown.
pub fn presets() -> Vec<PresetInfo> {
    PRESETS
        .iter()
        .map(|p| PresetInfo {
            id: p.id,
            label: p.label,
            description: p.description,
            table_starts_with: p.table_starts_with,
            source: p.layout.source,
            links: p.links,
        })
        .collect()
}

/// The login page of a site (the `start_site` of a preset), if one is known.
pub fn login_url_for(site: &str) -> Option<&'static str> {
    PRESETS
        .iter()
        .find(|p| p.start_site == site && !p.login_url.is_empty())
        .map(|p| p.login_url)
}

/// The sites that presets read, with their login pages, in the order of the presets.
pub fn login_pages() -> Vec<(&'static str, &'static str)> {
    let mut out: Vec<(&str, &str)> = Vec::new();
    for p in PRESETS {
        if !p.login_url.is_empty() && !out.iter().any(|(site, _)| *site == p.start_site) {
            out.push((p.start_site, p.login_url));
        }
    }
    out
}

impl Preset {
    pub(crate) fn find(id: &str) -> Option<&'static Preset> {
        PRESETS.iter().find(|p| p.id == id)
    }

    /// The recipe this preset stands for.
    pub(crate) fn recipe(&self, account: AccountId) -> Recipe {
        let layout = &self.layout;
        Recipe {
            preset: None,
            name: self.label.to_owned(),
            links: self
                .links
                .iter()
                .map(|text| FetchLink {
                    text: (*text).to_owned(),
                    reference: None,
                    menu: self.menu.as_ref().map(|m| m.group.to_owned()),
                    heading: self.menu.as_ref().map(|m| m.heading.to_owned()),
                })
                .collect(),
            table_starts_with: self.table_starts_with.to_owned(),
            all_tables: !self.following_tables,
            following_tables: self.following_tables,
            start_site: self.start_site.to_owned(),
            allowed_sites: self.allowed_sites.iter().map(|s| (*s).to_owned()).collect(),
            // The values are in the page's currency when the preset knows it, else in the currency
            // of the product they are linked to.
            currency: self.currency.to_owned(),
            account,
            mapping: ImportMapping {
                source: layout.source.to_owned(),
                name_column: layout.name_column,
                value_column: layout.value_column,
                price_column: layout.price_column,
                price_unit: layout.price_unit,
                header_rows: layout.header_rows,
                rows_per_record: layout.rows_per_record,
                name_row: layout.name_row,
                value_row: layout.value_row,
                name_part: None,
                value_part: None,
                price_part: layout.price_part,
                loose_numbers: layout.loose_numbers,
                quantity_column: layout.quantity_column,
                quantity_part: None,
                max_records: layout.max_records,
                kind_column: layout.kind_column,
                kind_rules: layout
                    .kinds
                    .iter()
                    .map(|(kind, read)| KindRule {
                        kind: (*kind).to_owned(),
                        read: *read,
                    })
                    .collect(),
            },
        }
    }
}

impl Recipe {
    /// The definition the recipe stands for. An ordinary recipe is its own definition. A recipe
    /// made from a preset is the app's current preset (read each time, so an app update updates
    /// it) with this recipe's name, account, and the remembered destinations of its links. It is
    /// an error when the preset no longer exists; the stored recipe is then kept as it is.
    pub fn effective(&self) -> Result<Recipe, Error> {
        let Some(id) = &self.preset else {
            return Ok(self.clone());
        };
        let preset = Preset::find(id).ok_or(Error::NotFound(
            "the preset of this recipe does not exist in this version of the app",
        ))?;
        let mut recipe = preset.recipe(self.account);
        recipe.name = self.name.clone();
        recipe.preset = self.preset.clone();
        for link in recipe.links.iter_mut().filter(|l| l.menu.is_none()) {
            if let Some(saved) = self.links.iter().find(|s| s.text == link.text) {
                link.reference = saved.reference.clone();
            }
        }
        Ok(recipe)
    }

    /// What is stored of a recipe made from a preset: its name, the preset id, the account, and the
    /// links that have a remembered destination (text and destination only). The definition is
    /// not copied. `self` is an effective recipe.
    pub(crate) fn into_preset_reference(self) -> Recipe {
        Recipe {
            name: self.name,
            links: self
                .links
                .into_iter()
                .filter(|l| l.reference.is_some())
                .map(|l| FetchLink {
                    text: l.text,
                    reference: l.reference,
                    menu: None,
                    heading: None,
                })
                .collect(),
            table_starts_with: String::new(),
            all_tables: false,
            following_tables: false,
            start_site: String::new(),
            allowed_sites: Vec::new(),
            currency: String::new(),
            account: self.account,
            mapping: ImportMapping::default(),
            preset: self.preset,
        }
    }
}
