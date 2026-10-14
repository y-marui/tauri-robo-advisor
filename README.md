# tauri-robo-advisor

Cross-platform desktop app that aggregates your brokerage assets and proposes a rebalancing plan toward your target portfolio. Built with [Tauri 2](https://tauri.app/).

> Status: early scaffold. No features are implemented yet.

## Goals

- No external runtime dependencies (no Python, Selenium, or separate browser install).
- Local-only: all data stays on your machine. Credentials are never stored; API keys go to the OS keychain.
- Desktop first (macOS / Windows / Linux).

## Requirements

- Node.js 20+
- Rust (stable) and the [Tauri prerequisites](https://tauri.app/start/prerequisites/) for your OS

## Development

~~~sh
npm install
npm run tauri dev
~~~

## Disclaimer

For personal use. This is not investment advice.

## License

[MIT](LICENSE)
