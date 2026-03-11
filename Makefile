SHELL := /bin/sh

COMPONENTS := indexer matcher router
DIST_DIR := dist

.PHONY: build test fmt clippy wasm clean pack all

all: wasm pack

build:
	cargo build --workspace

test:
	cargo test --workspace

fmt:
	cargo fmt --all

clippy:
	cargo clippy --workspace --all-targets --all-features -- -D warnings

wasm:
	@if ! cargo component --version >/dev/null 2>&1; then \
		echo "cargo-component is required to produce valid component@0.6.0 wasm"; \
		echo "install with: cargo install cargo-component --locked"; \
		exit 1; \
	fi
	@mkdir -p $(DIST_DIR)
	@for comp in $(COMPONENTS); do \
		echo "Building $$comp..."; \
		RUSTFLAGS= CARGO_ENCODED_RUSTFLAGS= cargo component build --release --target wasm32-wasip2 -p $$comp; \
		WASM_SRC=""; \
		for cand in \
			"target/wasm32-wasip2/release/$${comp}.wasm" \
			"target/wasm32-wasip2/release/$$(echo $$comp | tr '-' '_').wasm"; do \
			if [ -f "$$cand" ]; then WASM_SRC="$$cand"; break; fi; \
		done; \
		if [ -z "$$WASM_SRC" ]; then \
			echo "unable to locate wasm build artifact for $$comp"; \
			exit 1; \
		fi; \
		cp "$$WASM_SRC" $(DIST_DIR)/$$comp.wasm; \
	done
	@echo "WASM components built in $(DIST_DIR)/"

pack:
	@mkdir -p packs/fast2flow/components
	@for comp in $(COMPONENTS); do \
		if [ -f "$(DIST_DIR)/$$comp.wasm" ]; then \
			cp "$(DIST_DIR)/$$comp.wasm" packs/fast2flow/components/; \
		fi; \
	done
	@if command -v greentic-pack >/dev/null 2>&1; then \
		greentic-pack build --in packs/fast2flow --gtpack-out $(DIST_DIR)/fast2flow.gtpack; \
	else \
		echo "greentic-pack not found, skipping pack build"; \
	fi

clean:
	cargo clean
	rm -rf $(DIST_DIR)
	rm -rf packs/fast2flow/components
