# Ember Workspace — commandes de développement courantes.
# Usage : `make` (aide) ou `make <target>`.

CARGO  := cargo
GAMES  := pong breakout snake asteroids bullet-hell minesweeper simon memory freecell air-hockey zhuo-ji
CRATES := ember-core ember-stdlib ember-editor

.PHONY: help check fmt test test-crate test-one test-game build run ci clean

help:
	@echo "Cibles disponibles :"
	@echo "  make check                          # cargo clippy --workspace --all-targets -- -D warnings"
	@echo "  make test                           # cargo test --workspace"
	@echo "  make test-crate [FILTER=<f>]        # cargo test -p ember-stdlib [filtre]"
	@echo "  make test-one GAME=<n> [FILTER=<f>] # cargo test -p <n> [filtre]"
	@echo "  make build                          # cargo build --workspace"
	@echo "  make run GAME=<n>                   # cargo run -p <n>"
	@echo "  make fmt                            # cargo fmt --all"
	@echo "  make ci                             # check + test (à lancer avant commit)"
	@echo "  make clean                          # cargo clean"
	@echo ""
	@echo "Jeux  : $(GAMES)"
	@echo "Crates: $(CRATES)"

check:
	$(CARGO) clippy --workspace --all-targets -- -D warnings

fmt:
	$(CARGO) fmt --all

test:
	$(CARGO) test --workspace

test-crate:
	$(CARGO) test -p ember-stdlib $(FILTER)

test-one:
ifndef GAME
	$(error Il manque GAME=<jeu-ou-crate>. Ex: make test-one GAME=zhuo-ji)
endif
	$(CARGO) test -p $(GAME) $(FILTER)

test-game: test-one

build:
	$(CARGO) build --workspace

run:
ifndef GAME
	$(error Il manque GAME=<jeu>. Ex: make run GAME=zhuo-ji)
endif
	$(CARGO) run -p $(GAME)

ci: check test

clean:
	$(CARGO) clean