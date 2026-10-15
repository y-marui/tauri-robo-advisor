.PHONY: install dev lint format test build all deploy dmg update-charter

# Local settings (not committed): for example DEPLOY_DIR=/Applications in .env installs the app
# directly under /Applications instead of /Applications/My Tauri.
-include .env

# Where a self-made Tauri app is installed (kept apart from /Applications itself so
# it never collides with an App Store version), and where the DMG for
# `install-my-apps` is published.
APP_NAME ?= tauri-robo-advisor
DEPLOY_DIR ?= /Applications/My Tauri
DMG_DEST ?= $(HOME)/Library/Mobile Documents/com~apple~CloudDocs/Build/Tauri
BUNDLE_DIR ?= target/release/bundle
DEPLOY_OPEN ?= true

install:
	npm ci

dev:
	npm run tauri dev

lint:
	npx tsc --noEmit
	cargo fmt --all --check
	cargo clippy --workspace --all-targets --locked -- -D warnings

format:
	cargo fmt --all

test:
	npm test
	cargo test --workspace --locked

build:
	npm run tauri build -- --no-bundle

all: lint test

# Build the .app and install it into $(DEPLOY_DIR) (macOS only).
deploy:
	@test "$$(uname)" = "Darwin" || { echo "deploy is for macOS only."; exit 1; }
	npm run tauri build -- --bundles app
	@if pgrep -x "$(APP_NAME)" >/dev/null; then \
	  osascript -e 'tell application "$(APP_NAME)" to quit'; \
	  i=0; while pgrep -x "$(APP_NAME)" >/dev/null && [ $$i -lt 20 ]; do sleep 0.5; i=$$((i+1)); done; \
	fi
	mkdir -p "$(DEPLOY_DIR)"
	rm -rf "$(DEPLOY_DIR)/$(APP_NAME).app"
	ditto "$(BUNDLE_DIR)/macos/$(APP_NAME).app" "$(DEPLOY_DIR)/$(APP_NAME).app"
	@echo "Installed: $(DEPLOY_DIR)/$(APP_NAME).app"
	@if [ "$(DEPLOY_OPEN)" = "true" ]; then open "$(DEPLOY_DIR)/$(APP_NAME).app"; fi

# Build the DMG and publish it as $(DMG_DEST)/$(APP_NAME).dmg for install-my-apps.
dmg:
	@test "$$(uname)" = "Darwin" || { echo "dmg is for macOS only."; exit 1; }
	rm -rf "$(BUNDLE_DIR)/dmg"
	npm run tauri build -- --bundles dmg
	mkdir -p "$(DMG_DEST)"
	@set -- "$(BUNDLE_DIR)"/dmg/*.dmg; test -f "$$1" || { echo "No DMG was built."; exit 1; }; \
	  cp -f "$$1" "$(DMG_DEST)/$(APP_NAME).dmg" && echo "Published: $(DMG_DEST)/$(APP_NAME).dmg"

update-charter:
	curl -fsSL https://raw.githubusercontent.com/y-marui/dev-charter/main/scripts/install.sh | CHARTER_UPDATE_ONLY=1 bash
