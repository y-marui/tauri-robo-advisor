.PHONY: install dev lint format test build all update-charter

install:
	npm ci

dev:
	npm run tauri dev

lint:
	npx tsc --noEmit
	cargo fmt --manifest-path src-tauri/Cargo.toml --check
	cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings

format:
	cargo fmt --manifest-path src-tauri/Cargo.toml

test:
	cargo test --manifest-path src-tauri/Cargo.toml

build:
	npm run tauri build -- --no-bundle

all: lint test

update-charter:
	curl -fsSL https://raw.githubusercontent.com/y-marui/dev-charter/main/scripts/install.sh | CHARTER_UPDATE_ONLY=1 bash
