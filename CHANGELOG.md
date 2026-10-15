# Changelog

## [Unreleased]

### Added

- Browser fetch in the table import (#28): choose Chrome, Edge, or Brave, open it with a profile of its own, log in by hand, read the tables of the page, and pick one to go through the same column choice, alias linking, and replace-holdings flow as a file or pasted text. New commands `list_browsers`, `open_browser`, `browser_pages`, `fetch_browser_tables`, `use_fetched_table`, `close_browser`; the browser is closed on request and when the app quits.
- `browser` crate (first step of the optional browser fetch, #28): finds Chrome, Edge, or Brave, starts it with a profile of its own, and reads pages over the DevTools protocol on localhost. The user logs in by hand; nothing is typed, clicked, or stored. New dependencies: `tungstenite` (WebSocket client, without TLS) with `sha1` and `data-encoding`.
- Table import: a record may span several rows (a name row and a row of numbers, as brokerage holdings pages show them), and the value can be quantity x price / price unit (1 or 10,000, for fund prices) computed with exact decimals. Saved mappings from before keep working.
- Generic table import (#9): load a CSV file (UTF-8 or Shift-JIS, detected automatically) or pasted text, choose the name and value columns (saved per source and reused), link names to products through aliases the user confirms, and replace an account's holdings. New `tabular` crate (no brokerage-specific code) and a new dependency, `encoding_rs`; the import result view is shared with the bitFlyer card.
- bitFlyer card: save and delete the API key (never shown again), fetch balances, link each currency to a product through an alias the user confirms, and replace an account's holdings with the linked balances. New `portfolio` pieces: the `ReplaceHoldings` edit and `plan_import` (also the base for the generic import).
- Account tabs panel: one tab per account with the products inside the account compared with their weights (short or over, in yen) and a suggestion for adding to that account. The new read-only `get_account_balance` command supplies the figures from Rust; the suggestion box is now shared with the overall balance panel.
- Holdings panel: holding records grouped by category with subtotal rows (value, share of the total, target, and difference in points), edited in place. `get_overview` now also returns each holding's JPY value and share, and one valuation is fetched per refresh for all panels.
- Overall balance panel: target-vs-current bars and a composition pie in hand-written SVG, the rebalancing-account selector, and the four suggestion modes shown in the app. The new read-only `get_overview` command supplies the values and shares from Rust.
- Full-record export: an Export button writes CSV files (holdings, categories, products, aliases, accounts, settings), the complete data as JSON, and a README into a new owner-only folder in Downloads, for use in other analysis software. Names that could run as spreadsheet formulas are written as text.
- `suggest` crate with the four suggestion modes (fixed add, buy-only to target, full rebalance, fixed sell): water-filling over categories and products, account rules (sellable, tradable products, rounding unit), exact sums after rounding, and the read-only `suggest_rebalance` command.
- Manual entry screen (vanilla TypeScript, English and Japanese): categories, products, accounts, a holdings table, and settings, with every edit validated and saved by Rust. Frontend unit tests run with `npm test`.
- `portfolio` crate with the portfolio model (categories, products, accounts, holdings, targets), validated edits, and integrity checks, plus JSON storage with atomic writes and the `get_portfolio` / `apply_portfolio_edit` commands.
- `bitflyer` crate: read-only bitFlyer API client that signs requests, fetches balances, and values them in JPY from the public ticker; API credentials are stored in the OS keychain and never returned to the frontend.
- `rebalance` crate: portfolio rebalancing calculation with exact decimal arithmetic, and the `propose_rebalance` Tauri command.
- Cargo workspace and a committed `Cargo.lock`; CI and `make` now run cargo with `--locked`.
- Tauri 2 (vanilla TypeScript) scaffold.
- AGPL v3 license, English and Japanese READMEs, and project documentation.
- CI (security scan, lint, test, build), Dependabot, and dev-charter (full) integration.

### Changed

- Removed the scaffold demo (the `greet` command and the sample UI).
- Documented the dashboard design: portfolio model, account roles, four suggestion modes, a generic table import with saved column mappings (instead of per-broker parsers), JSON storage, and the three-panel layout.
- Changed the planned acquisition method for SBI Securities and Monex from in-app WebView login to importing files downloaded from the user's own browser session, because both brokerages require passkeys that embedded WebViews do not reliably support.
- Removed the unused `tauri-plugin-opener` plugin and its `opener:default` permission to reduce the permission surface and dependencies.
- Set a strict Content Security Policy (production `csp` and development `devCsp`) instead of the scaffold default `null`.
- Relicensed from MIT to AGPL v3 (`AGPL-3.0-only`), with contribution terms that allow future commercial licensing.

### Fixed
