.PHONY: help deps deps-check build build-backend build-backend-release build-ui run run-release doctor import test test-backend test-ui test-parity lint lint-backend lint-ui fmt check e2e clean

CONFIG ?= examples/domers.toml
BIND ?= 127.0.0.1:3000
FEATURES ?=
SPECTRUM_XML ?= fixtures/config/spectrum_default_config.xml
OUTPUT_CONFIG ?= domers.toml

CARGO_FEATURE_FLAGS := $(if $(strip $(FEATURES)),--features $(FEATURES),)

help:
	@echo "domers make targets"
	@echo ""
	@echo "Setup:"
	@echo "  make deps-check              Check local Rust, UI, Python, and Madmom deps"
	@echo "  make deps                    Install/check local development deps"
	@echo ""
	@echo "Build:"
	@echo "  make build                   Build frontend and backend"
	@echo "  make build-ui                Build React/TypeScript UI assets"
	@echo "  make build-backend           Build Rust workspace"
	@echo "  make build-backend-release   Build Rust domers binary in release mode"
	@echo ""
	@echo "Run:"
	@echo "  make run                     Run domers with CONFIG and BIND"
	@echo "  make run-release             Build release binary, then run it"
	@echo "  make doctor                  Validate CONFIG, BIND, OPC, and sidecars"
	@echo "  make import                  Import SPECTRUM_XML to OUTPUT_CONFIG"
	@echo ""
	@echo "Test and lint:"
	@echo "  make test                    Run backend and UI tests/checks"
	@echo "  make test-backend            Run Cargo workspace tests"
	@echo "  make test-ui                 Run UI typecheck and smoke check"
	@echo "  make test-parity             Run Spectrum visualizer parity gates"
	@echo "  make lint                    Run Rust fmt/clippy and UI checks"
	@echo "  make lint-backend            Run cargo fmt check and clippy"
	@echo "  make lint-ui                 Run UI typecheck and smoke check"
	@echo "  make fmt                     Format Rust code"
	@echo "  make check                   Run build, test, and lint"
	@echo "  make e2e                     Run full local/CI gate including parity"
	@echo ""
	@echo "Variables:"
	@echo "  CONFIG=$(CONFIG)"
	@echo "  BIND=$(BIND)"
	@echo "  FEATURES=$(FEATURES)"
	@echo "  SPECTRUM_XML=$(SPECTRUM_XML)"
	@echo "  OUTPUT_CONFIG=$(OUTPUT_CONFIG)"

deps-check:
	tools/install_dev_deps.sh --check
	cd ui && bun install --frozen-lockfile

deps:
	tools/install_dev_deps.sh
	cd ui && bun install

build: build-ui build-backend

build-backend:
	cargo build --workspace $(CARGO_FEATURE_FLAGS)

build-backend-release:
	cargo build --release --bin domers $(CARGO_FEATURE_FLAGS)

build-ui:
	cd ui && bun install --frozen-lockfile && bun run build

run:
	cargo run --bin domers $(CARGO_FEATURE_FLAGS) -- run --config "$(CONFIG)" --bind "$(BIND)"

run-release: build-backend-release
	./target/release/domers run --config "$(CONFIG)" --bind "$(BIND)"

doctor:
	cargo run --bin domers $(CARGO_FEATURE_FLAGS) -- doctor --config "$(CONFIG)" --bind "$(BIND)"

import:
	cargo run --bin domers $(CARGO_FEATURE_FLAGS) -- import-spectrum-xml "$(SPECTRUM_XML)" "$(OUTPUT_CONFIG)"

test: test-backend test-ui

test-backend:
	cargo test --workspace $(CARGO_FEATURE_FLAGS)

test-ui: build-ui
	cd ui && bun run check && bun run smoke

test-parity:
	python3 tools/check_visualizer_goldens.py
	cargo test -p domers-visualizers rust_visualizer_hashes_match_spectrum_csharp_goldens -- --ignored --nocapture

lint: lint-backend lint-ui

lint-backend:
	cargo fmt --all -- --check
	cargo clippy --workspace --all-targets $(CARGO_FEATURE_FLAGS) -- -D warnings

lint-ui:
	cd ui && bun run check && bun run smoke

fmt:
	cargo fmt --all

check: build test lint

e2e: check test-parity

clean:
	cargo clean
	rm -rf ui/dist
