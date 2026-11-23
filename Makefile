# Ember Workspace — development workflow.
# Usage: `make` (help) or `make <target> [GAME=<name>] [CRATE=<name>] [FILTER=<test>]`

CARGO   ?= cargo
PYTHON  ?= python3
GAMES   := $(notdir $(wildcard games/*))
ENGINE  := ember-core ember-macros ember-servers ember-render-macroquad ember-audio-macroquad \
           ember-input ember-scene ember-ui ember-assets ember-editor ember ember-stdlib
WEB_GAMES := $(patsubst games/%/tools/build-web.sh,%,$(wildcard games/*/tools/build-web.sh))
VAULT   := ember-vault
PORT    ?= 8000
ENGINE_FLAGS := $(addprefix -p ,$(ENGINE))

.DEFAULT_GOAL := help
.PHONY: help setup hooks build release run run-release example ui-demo \
        test test-engine test-games test-crate test-game test-one test-count \
        check fix fmt fmt-check layers macroquad gates ci pre-push \
        doc web web-serve vault vault-check docs-guard \
        status branches stats outdated clean

## —— Help ———————————————————————————————————————————————————————————————
help: ## Show this help
	@awk 'BEGIN{FS=":.*## "} /^## ——/{sub(/^## /,"");printf "\n\033[1m%s\033[0m\n",$$0} /^[a-zA-Z_-]+:.*## /{printf "  \033[36m%-14s\033[0m %s\n",$$1,$$2}' $(MAKEFILE_LIST)
	@echo ""
	@echo "Games : $(GAMES)"
	@echo "Web   : $(WEB_GAMES)"
	@echo "Engine: $(ENGINE)"

## —— Setup ——————————————————————————————————————————————————————————————
setup: hooks ## Install toolchain components (clippy, rustfmt, wasm32) + git hooks
	rustup component add clippy rustfmt
	rustup target add wasm32-unknown-unknown

hooks: ## Enable the commit-msg hook (enforces type(scope): Description)
	git config core.hooksPath .githooks
	@echo "commit-msg hook enabled — format: ember-vault/70 Workflow/Commit & Push.md"

## —— Build & run ————————————————————————————————————————————————————————
build: ## cargo build --workspace
	$(CARGO) build --workspace

release: ## cargo build --workspace --release
	$(CARGO) build --workspace --release

run: _need-game ## Run a game: make run GAME=pong
	$(CARGO) run -p $(GAME)

run-release: _need-game ## Run a game in release mode: make run-release GAME=pagoda
	$(CARGO) run --release -p $(GAME)

example: ## Run the engine demo (crates/ember/examples/hello_scene.rs)
	$(CARGO) run -p ember --example hello_scene

ui-demo: ## Run the UI demo (crates/ember/examples/ui_demo.rs)
	$(CARGO) run -p ember --example ui_demo

## —— Tests ——————————————————————————————————————————————————————————————
test: ## All tests, no fail-fast (workspace)
	$(CARGO) test --workspace --no-fail-fast

test-engine: ## Tests of the 12 engine crates only
	$(CARGO) test $(ENGINE_FLAGS) --no-fail-fast $(FILTER)

test-games: ## Tests of every game crate
	$(CARGO) test $(addprefix -p ,$(GAMES)) --no-fail-fast

test-crate: _need-crate ## One engine crate: make test-crate CRATE=ember-scene [FILTER=signal]
	$(CARGO) test -p $(CRATE) $(FILTER)

test-game: _need-game ## One game: make test-game GAME=ember-wars [FILTER=catapult]
	$(CARGO) test -p $(GAME) $(FILTER)

test-one: test-game ## Alias of test-game (kept for old habits)

test-count: ## Count passed/failed tests across the workspace
	@$(CARGO) test --workspace --no-fail-fast 2>&1 | awk '/^test result:/{p+=$$4; f+=$$6} END{printf "passed: %d  failed: %d  total: %d\n", p, f, p+f}'

## —— Quality gates ——————————————————————————————————————————————————————
check: ## Clippy, warnings as errors (the quality gate)
	$(CARGO) clippy --workspace --all-targets -- -D warnings

fix: ## Apply clippy + rustfmt auto-fixes (review the diff!)
	$(CARGO) clippy --workspace --all-targets --fix --allow-dirty --allow-staged
	$(CARGO) fmt --all

fmt: ## Format all code
	$(CARGO) fmt --all

fmt-check: ## Fail if code is not formatted
	$(CARGO) fmt --all -- --check

layers: ## xtask: engine crates only depend on lower layers
	$(CARGO) run -q -p xtask -- check-layers

macroquad: ## xtask: macroquad only imported by backend crates (+ allowlist)
	$(CARGO) run -q -p xtask -- check-macroquad

gates: layers macroquad ## Both architecture checks

ci: fmt-check check gates docs-guard test ## Everything CI should run

pre-push: ci vault-check ## Run before every push / handover
	@echo "✅ ready to push"

## —— Docs (Obsidian vault) ——————————————————————————————————————————————
vault: ## Open the vault in Obsidian (macOS)
	@open -a Obsidian "$(VAULT)" 2>/dev/null || { echo "Obsidian not found — install it (brew install --cask obsidian), then: Open folder as vault → $(VAULT)"; exit 1; }

vault-check: ## Every [[wikilink]] in the vault resolves
	$(PYTHON) scripts/check_vault.py --links-only

docs-guard: ## Fail if a .md other than README.md exists outside the vault
	$(PYTHON) scripts/check_vault.py --guard-only

doc: ## Build and open rustdoc for the engine crates
	$(CARGO) doc --no-deps $(ENGINE_FLAGS) --open

## —— Web (WASM) —————————————————————————————————————————————————————————
web: _need-game ## Build a game for the browser: make web GAME=pagoda
	@test -x games/$(GAME)/tools/build-web.sh || { echo "No web build for $(GAME). Available: $(WEB_GAMES)"; exit 1; }
	cd games/$(GAME) && ./tools/build-web.sh

web-serve: _need-game ## Serve games/<GAME>/dist/web on http://localhost:$(PORT)
	cd games/$(GAME)/dist/web && $(PYTHON) -m http.server $(PORT)

## —— Repo ———————————————————————————————————————————————————————————————
status: ## Git status + unpushed commits on every local branch
	@git status --short --branch
	@echo ""; echo "Unpushed / unmerged branches:"
	@git for-each-ref --format='  %(refname:short) %(upstream:track)' refs/heads

branches: ## Local branches not merged into main
	@git branch --no-merged main

stats: ## Lines of Rust per engine crate and per game
	@for d in crates/* crates/ember-render/* crates/ember-audio/* games/*; do \
	  [ -d $$d/src ] && printf "  %-34s %6s lines\n" $$d $$(find $$d -name '*.rs' -not -path '*/target/*' | xargs cat | wc -l); \
	done; true

outdated: ## Show outdated dependencies (needs: cargo install cargo-outdated)
	$(CARGO) outdated --workspace --root-deps-only

clean: ## cargo clean
	$(CARGO) clean

# —— internal guards ——
_need-game:
	@test -n "$(GAME)" || { echo "Missing GAME=<name>. Games: $(GAMES)"; exit 1; }
_need-crate:
	@test -n "$(CRATE)" || { echo "Missing CRATE=<name>. Crates: $(ENGINE)"; exit 1; }
