# Changelog

## [Unreleased]

### Added

- `bitflyer` crate: read-only bitFlyer API client that signs requests, fetches balances, and values them in JPY from the public ticker; API credentials are stored in the OS keychain and never returned to the frontend.
- `rebalance` crate: portfolio rebalancing calculation with exact decimal arithmetic, and the `propose_rebalance` Tauri command.
- Cargo workspace and a committed `Cargo.lock`; CI and `make` now run cargo with `--locked`.
- Tauri 2 (vanilla TypeScript) scaffold.
- AGPL v3 license, English and Japanese READMEs, and project documentation.
- CI (security scan, lint, test, build), Dependabot, and dev-charter (full) integration.

### Changed

- Documented the dashboard design: portfolio model, account roles, four suggestion modes, a generic table import with saved column mappings (instead of per-broker parsers), JSON storage, and the three-panel layout.
- Changed the planned acquisition method for SBI Securities and Monex from in-app WebView login to importing files downloaded from the user's own browser session, because both brokerages require passkeys that embedded WebViews do not reliably support.
- Removed the unused `tauri-plugin-opener` plugin and its `opener:default` permission to reduce the permission surface and dependencies.
- Set a strict Content Security Policy (production `csp` and development `devCsp`) instead of the scaffold default `null`.
- Relicensed from MIT to AGPL v3 (`AGPL-3.0-only`), with contribution terms that allow future commercial licensing.

### Fixed
