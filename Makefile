.PHONY: bundle-app install dev lint format test build all deploy dmg unlock-keychain update-charter

# Local settings (not committed): for example DEPLOY_DIR=/Applications in .env installs the app
# directly under /Applications instead of /Applications/My Tauri.
-include .env

# Where a self-made Tauri app is installed (kept apart from /Applications itself so
# it never collides with an App Store version; set DEPLOY_DIR=/Applications in .env to install
# directly under it).
# The name of the app bundle and the DMG (the productName in src-tauri/tauri.conf.json).
APP_NAME ?= Robo Advisor
# The process name of the running app (the executable inside the bundle keeps the Cargo package name).
PROCESS_NAME ?= tauri-robo-advisor
DEPLOY_DIR ?= /Applications/My Tauri
BUNDLE_DIR ?= target/release/bundle
DEPLOY_OPEN ?= true
# Where the DMG is mounted while `make deploy` installs from it.
DEPLOY_MOUNT ?= /tmp/robo-advisor-deploy
# Like the swift-* repositories: set DEPLOY_DMG=true and DMG_DEST in .env to publish the DMG for
# `install-my-apps` and install from it in `make deploy` (see .env.example). A leading ~/ in DMG_DEST is expanded.
DEPLOY_DMG ?= false
DMG_DEST ?=
DMG_DIR = $(patsubst ~/%,$(HOME)/%,$(DMG_DEST))
# Code signing, like the swift-* repositories: a Developer ID Application certificate of this team found
# in the keychain signs the app (a stable signature, so macOS keeps the permissions across deploys).
# Override TEAM_ID / DEVELOPER_NAME in .env, or set SIGN_IDENTITY to use one certificate explicitly.
# With no certificate the app is signed ad-hoc and a message says so (the build does not fail).
TEAM_ID ?= 7TEQWKRRX7
DEVELOPER_NAME ?= Yukihiro Marui
SIGN_IDENTITY ?= $(shell TEAM_ID="$(TEAM_ID)" DEVELOPER_NAME="$(DEVELOPER_NAME)" bash scripts/sign-identity.sh 2>/dev/null)

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

# Build the .app (signed with $(SIGN_IDENTITY) when there is one, otherwise ad-hoc) and verify it.
bundle-app:
	@if [ -n "$(SIGN_IDENTITY)" ]; then echo "Signing with: $(SIGN_IDENTITY)"; \
	else echo "No Developer ID Application certificate found: signing ad-hoc (permissions are asked again after every build). Set TEAM_ID / DEVELOPER_NAME or SIGN_IDENTITY in .env."; fi
	APPLE_SIGNING_IDENTITY="$(or $(SIGN_IDENTITY),-)" npm run tauri build -- --bundles app
	bash scripts/verify-signature.sh "$(BUNDLE_DIR)/macos/$(APP_NAME).app"

# Build the .app and install it into $(DEPLOY_DIR) (macOS only).
# With DEPLOY_DMG=true the app is installed from the DMG that `make dmg` just built (like the swift-*
# repositories), so the installed .app has the same timestamp as the one in the published DMG and
# `install-my-apps` sees it as already up to date instead of reinstalling it.
deploy:
	@test "$$(uname)" = "Darwin" || { echo "deploy is for macOS only."; exit 1; }
ifeq ($(DEPLOY_DMG),true)
	$(MAKE) -s dmg
else
	$(MAKE) bundle-app
endif
	@if pgrep -x "$(PROCESS_NAME)" >/dev/null; then \
	  osascript -e 'tell application "$(APP_NAME)" to quit'; \
	  i=0; while pgrep -x "$(PROCESS_NAME)" >/dev/null && [ $$i -lt 20 ]; do sleep 0.5; i=$$((i+1)); done; \
	fi
ifeq ($(DEPLOY_DMG),true)
	@if mount | grep -Fq "on $(DEPLOY_MOUNT) "; then hdiutil detach "$(DEPLOY_MOUNT)" -quiet; fi
	hdiutil attach "$(BUNDLE_DIR)/dmg/$(APP_NAME).dmg" -mountpoint "$(DEPLOY_MOUNT)" -quiet -nobrowse
	bash scripts/install-app.sh "$(DEPLOY_MOUNT)/$(APP_NAME).app" "$(DEPLOY_DIR)" "$(APP_NAME)"; \
	status=$$?; hdiutil detach "$(DEPLOY_MOUNT)" -quiet; exit $$status
else
	bash scripts/install-app.sh "$(BUNDLE_DIR)/macos/$(APP_NAME).app" "$(DEPLOY_DIR)" "$(APP_NAME)"
endif
	@if [ "$(DEPLOY_OPEN)" = "true" ]; then open "$(DEPLOY_DIR)/$(APP_NAME).app"; fi

# Build the DMG (scripts/build-dmg.sh: hdiutil, no Finder scripting). When DMG_DEST is set, publish
# it as $(DMG_DEST)/$(APP_NAME).dmg for install-my-apps.
dmg:
	@test "$$(uname)" = "Darwin" || { echo "dmg is for macOS only."; exit 1; }
	$(MAKE) bundle-app
	SIGN_IDENTITY="$(SIGN_IDENTITY)" bash scripts/build-dmg.sh "$(BUNDLE_DIR)/macos/$(APP_NAME).app" "$(BUNDLE_DIR)/dmg/$(APP_NAME).dmg"
	@if [ -n "$(DMG_DIR)" ]; then \
	  mkdir -p "$(DMG_DIR)" && cp -f "$(BUNDLE_DIR)/dmg/$(APP_NAME).dmg" "$(DMG_DIR)/$(APP_NAME).dmg" && echo "Published: $(DMG_DIR)/$(APP_NAME).dmg"; \
	else \
	  echo "DMG_DEST is not set, so the DMG was not published (set it in .env)."; \
	fi

# Unlock the login keychain so codesign can use the signing key (errSecInternalComponent means it is locked).
unlock-keychain:
	security unlock-keychain ~/Library/Keychains/login.keychain-db

update-charter:
	curl -fsSL https://raw.githubusercontent.com/y-marui/dev-charter/main/scripts/install.sh | CHARTER_UPDATE_ONLY=1 bash
