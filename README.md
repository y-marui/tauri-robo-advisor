# tauri-robo-advisor

> **This is the reference (English) version.**
> The canonical (Japanese) version is [README-jp.md](README-jp.md).

[![License: AGPL v3](https://img.shields.io/badge/License-AGPL_v3-blue.svg)](LICENSE)
[![CI](https://github.com/y-marui/tauri-robo-advisor/actions/workflows/ci.yml/badge.svg)](https://github.com/y-marui/tauri-robo-advisor/actions/workflows/ci.yml)
[![Charter Check](https://github.com/y-marui/tauri-robo-advisor/actions/workflows/dev-charter-check.yml/badge.svg)](https://github.com/y-marui/tauri-robo-advisor/actions/workflows/dev-charter-check.yml)
[![GitHub Sponsors](https://img.shields.io/github/sponsors/y-marui?style=social)](https://github.com/sponsors/y-marui)
[![Buy Me a Coffee](https://img.shields.io/badge/Buy%20Me%20a%20Coffee-donate-yellow.svg)](https://www.buymeacoffee.com/y.marui)

A cross-platform desktop app that aggregates assets from Japanese brokerage accounts and proposes a rebalancing plan toward your own target portfolio, with no external runtime dependencies. Built with [Tauri 2](https://tauri.app/).

> Status: early scaffold. No features are implemented yet.

## Requirements

- Node.js 20+
- Rust (stable) and the [Tauri prerequisites](https://tauri.app/start/prerequisites/) for your OS
- Supported OS: macOS, Windows, Linux (desktop only)

## Setup

~~~sh
make install   # npm ci
make dev       # launch the app in development mode
make build     # build the app (no installer bundle)
~~~

## Project Structure

~~~text
src/         # frontend (TypeScript, CSS): display and input only
src-tauri/   # Rust: Tauri setup and thin command layer
crates/      # Rust: domain logic crates (no Tauri dependency)
docs/        # specification and knowledge base
scripts/     # check scripts distributed by dev-charter
~~~

## Usage

The app is a scaffold, so there is nothing to operate yet. Development commands:

| Command | Description |
|---|---|
| `make dev` | Run the app in development mode |
| `make lint` | Type check, `cargo fmt --check`, and `cargo clippy` |
| `make test` | Run frontend and Rust tests |
| `make build` | Build the app without bundling |
| `make all` | Lint and test |

## Supported Sources

Built-in presets read the holdings tables of a logged-in browser page (you log in yourself). "Verified" means that the page structure was checked against the real page by the owner; it is not a guarantee, because a page change can break a preset.

| Source | Reads | Status |
|---|---|---|
| SBI Securities | Domestic stocks, investment trusts | Verified |
| Monex Securities | Investment trusts | Verified |
| Rakuten Securities | Investment trusts in yen | Verified. Domestic stocks are read by the kind column (#88); they are covered by tests with made-up pages only, and the kind labels are not checked against a real page |
| Daiwa Connect Securities | Domestic stocks, cash (yen) | Verified for the stock list; the cash page is not recorded as verified |
| bitFlyer | Balances through the REST API (read-only key in the OS keychain) | Verified against the real service with a read-only key (authentication and balances). A currency without a yen market on bitFlyer Lightning is left out and reported; prices from the public price page are planned (#103) |
| PayPay Securities | Holdings cards (evaluation only; the page shows no quantity or price). Provisional preset | Structure checked on the real page (two holdings). "More" and many holdings are not checked (#76) |

Other sources: import a CSV or pasted table with your own column mapping, or enter holdings by hand.

## Design Principles

- No external runtime dependencies (no Python, Selenium, or separate browser install).
- Local-only: data never leaves your machine. Login credentials are never stored, and API keys go to the OS keychain.
- Desktop first. Mobile is out of scope.

## Documentation

- [DEVELOPING.md](DEVELOPING.md): build, test, and coding conventions
- [CONTRIBUTING.md](CONTRIBUTING.md): pull request and issue rules
- [docs/architecture.md](docs/architecture.md): module and component structure
- [docs/specification.md](docs/specification.md): specification and data flow (index; the details are in `docs/specification-*.md`)
- [docs/ui-design.md](docs/ui-design.md): UI design (the settings tab is in `docs/ui-design-settings.md`)
- [docs/design-constraints.md](docs/design-constraints.md): design constraints and project rules in full

## Disclaimer

For personal use. This is not investment advice, and rebalancing proposals are mechanical calculations. Check the terms of service of each brokerage before use.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

[AGPL v3](LICENSE). A commercial license may be offered in the future; contributions are accepted under the terms in [CONTRIBUTING.md](CONTRIBUTING.md).

---

*This document has a Japanese canonical version [README-jp.md](README-jp.md). Update both in the same commit when editing.*
