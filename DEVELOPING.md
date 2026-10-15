# Developing

Build, test, and coding conventions for contributors.

## Prerequisites

- Node.js 20+ and npm
- Rust (stable) with the [Tauri prerequisites](https://tauri.app/start/prerequisites/) for your OS
- [pre-commit](https://pre-commit.com/) and [gitleaks](https://github.com/gitleaks/gitleaks)

On Linux, Tauri also needs system libraries (for Debian/Ubuntu: `libwebkit2gtk-4.1-dev`, `libssl-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`, `libxdo-dev`, `build-essential`). The CI workflow lists the exact set.

## Commands

| Command | Description |
|---|---|
| `make install` | Install frontend dependencies (`npm ci`) |
| `make dev` | Run the app in development mode |
| `make lint` | `tsc --noEmit`, `cargo fmt --all --check`, `cargo clippy --workspace -D warnings` |
| `make format` | `cargo fmt` |
| `make test` | `npm test` (frontend unit tests) and `cargo test` |
| `make build` | Build the app without bundling |
| `make deploy` | Build the `.app` and install it into `/Applications/My Tauri` (override with `DEPLOY_DIR`; `DEPLOY_OPEN=false` skips launching it); macOS only |
| `make dmg` | Build the DMG and publish it to the iCloud `Build/Tauri` folder as `<app>.dmg` for `install-my-apps` (override with `DMG_DEST`); macOS only |
| `make all` | `lint` and `test` |
| `make update-charter` | Update the dev-charter subtree |

The keychain round-trip test in `src-tauri/src/secrets.rs` touches the real OS keychain (with dummy values only), so it is ignored by default. Run it on a desktop with `cargo test -p tauri-robo-advisor -- --ignored`.

Frontend tests are `src/*.test.ts`, run by Node's built-in test runner (no extra dependency). They are excluded from `tsc --noEmit` because that would need `@types/node`.

## Project Layout

| Path | Role |
|---|---|
| `src/` | Frontend (TypeScript, CSS) |
| `src-tauri/` | Tauri setup and thin command layer (Rust) |
| `crates/` | Domain logic crates in the Cargo workspace (no Tauri dependency) |
| `docs/` | Specification and knowledge base |
| `docs/dev-charter/` | Development charter (git subtree; never edit directly) |
| `scripts/` | Check scripts distributed by dev-charter |

## Conventions

- Keep logic in Rust. The TypeScript frontend only handles display and input. Tauri commands stay thin and delegate to domain logic that does not depend on Tauri. See [docs/architecture.md](docs/architecture.md) for when to split cohesive logic into its own Rust repository.
- Rust: `cargo fmt` and `cargo clippy` clean. Use a decimal type for money; never floating point.
- TypeScript: strict mode, no `any` without a reason noted in a comment.
- Comments explain why, not what ([CODE_STYLE.md](docs/dev-charter/CODE_STYLE.md)).
- UI text is English by default; follow [LOCALIZATION_POLICY.md](docs/dev-charter/LOCALIZATION_POLICY.md) when adding languages.
- Do not hardcode local absolute paths. Do not store credentials; keep API keys in the OS keychain.
- Add or update documents under `docs/` in the same change that alters behavior or structure.

## Dependencies

Keep dependencies minimal. Before adding one, check whether the standard library or an existing dependency already covers the need, and open an issue for review.

## Releases

Record user-facing changes in the `[Unreleased]` section of [CHANGELOG.md](CHANGELOG.md).
