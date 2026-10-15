# Changelog

## [Unreleased]

### Added

- `rebalance` crate: portfolio rebalancing calculation with exact decimal arithmetic, and the `propose_rebalance` Tauri command.
- Cargo workspace and a committed `Cargo.lock`; CI and `make` now run cargo with `--locked`.
- Tauri 2 (vanilla TypeScript) scaffold.
- AGPL v3 license, English and Japanese READMEs, and project documentation.
- CI (security scan, lint, test, build), Dependabot, and dev-charter (full) integration.

### Changed

- Changed the planned acquisition method for SBI Securities and Monex from in-app WebView login to importing files downloaded from the user's own browser session, because both brokerages require passkeys that embedded WebViews do not reliably support.
- Removed the unused `tauri-plugin-opener` plugin and its `opener:default` permission to reduce the permission surface and dependencies.
- Set a strict Content Security Policy (production `csp` and development `devCsp`) instead of the scaffold default `null`.
- Relicensed from MIT to AGPL v3 (`AGPL-3.0-only`), with contribution terms that allow future commercial licensing.

### Fixed
