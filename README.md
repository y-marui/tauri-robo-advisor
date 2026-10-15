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
| `make test` | Run Rust tests |
| `make build` | Build the app without bundling |
| `make all` | Lint and test |

## Design Principles

- No external runtime dependencies (no Python, Selenium, or separate browser install).
- Local-only: data never leaves your machine. Login credentials are never stored, and API keys go to the OS keychain.
- Desktop first. Mobile is out of scope.

## Documentation

- [DEVELOPING.md](DEVELOPING.md): build, test, and coding conventions
- [CONTRIBUTING.md](CONTRIBUTING.md): pull request and issue rules
- [docs/architecture.md](docs/architecture.md): module and component structure
- [docs/specification.md](docs/specification.md): specification and data flow
- [docs/ui-design.md](docs/ui-design.md): UI design

## Disclaimer

For personal use. This is not investment advice, and rebalancing proposals are mechanical calculations. Check the terms of service of each brokerage before use.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

[AGPL v3](LICENSE). A commercial license may be offered in the future; contributions are accepted under the terms in [CONTRIBUTING.md](CONTRIBUTING.md).

---

*This document has a Japanese canonical version [README-jp.md](README-jp.md). Update both in the same commit when editing.*
