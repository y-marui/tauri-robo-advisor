# Contributing

## How to Contribute

For large changes (new features, design changes), please open an issue before submitting a PR.
Small bug fixes and typos can be submitted directly as a PR.

## Development Flow

1. Create an Issue describing the task (for non-trivial work).
2. Create a short-lived working branch from `main` (`scripts/new-branch.sh <name>`). Direct commits and pushes to `main` are blocked.
3. Implement, and verify locally with `make all` and `pre-commit run --all-files`.
4. Open a Pull Request against `main` once the work is complete. Do not open PRs for unfinished work.
5. Merge after CI passes and all conversations are resolved.

## Development Setup

See [README.md](README.md) for setup and [DEVELOPING.md](DEVELOPING.md) for build, test, and coding conventions.

## Code Style

Follow [CODE_STYLE.md](docs/dev-charter/CODE_STYLE.md).

## Commit Messages

Use [Conventional Commits](https://www.conventionalcommits.org/) format (e.g. `fix: ...`, `feat: ...`). Do not commit non-working code.

## Pull Request Checklist

See `.github/pull_request_template.md` for the current checklist.

## Pre-commit Hooks

Security and quality checks run via `.pre-commit-config.yaml` and again in CI. Never commit secrets, `.env` files, or real account data.
