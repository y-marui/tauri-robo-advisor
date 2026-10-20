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
| `make deploy` | Build the `.app` and install it into `/Applications/My Tauri` (override with `DEPLOY_DIR`, on the command line or in `.env`; `DEPLOY_DIR=/Applications` installs directly under `/Applications`; `DEPLOY_OPEN=false` skips launching it; `DEPLOY_DMG=true` also runs `make dmg` afterwards, like the `swift-*` repositories; it waits for a running copy to quit); macOS only |
| `make bundle-app` | Build the `.app` and verify its signature (used by `deploy` and `dmg`); see Code signing below; macOS only |
| `make dmg` | Build the DMG; when `DMG_DEST` is set (in `.env`, see `.env.example`), publish it there as `<app>.dmg` for `install-my-apps`, normally the iCloud `Build/Tauri` folder; macOS only |
| `make all` | `lint` and `test` |
| `make update-charter` | Update the dev-charter subtree |

### Code signing (macOS)

`make deploy` and `make dmg` sign the app with a `Developer ID Application` certificate found in the login keychain (`security find-identity -v -p codesigning`, filtered by `DEVELOPER_NAME` and `TEAM_ID`, as in the `swift-*` repositories; `scripts/sign-identity.sh`). It is passed to Tauri as `APPLE_SIGNING_IDENTITY`, so Tauri signs the bundle (no notarization unless `APPLE_ID` and friends are set, which this repository does not do). The DMG is signed with the same identity. The signature is then checked with `codesign --verify --deep --strict` (`scripts/verify-signature.sh`), which also prints whether it is ad-hoc.

Why: macOS ties permissions (App Management, keychain access) to the code signature. An ad-hoc signature changes with every build, so permissions are asked again after each deploy; the certificate-based designated requirement stays the same (`codesign -d -r- <app>`).

- `TEAM_ID` and `DEVELOPER_NAME` are `?=` variables in the Makefile (not secret); override them in `.env`. Set `SIGN_IDENTITY` in `.env` to use one certificate explicitly.
- Without a matching certificate the build does not fail: the app is signed ad-hoc and the message says so.
- No key, password, or certificate file belongs in the repository; the private key stays in the keychain. The first signing may show a keychain prompt: choose "Always Allow".

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
