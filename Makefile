.PHONY: help install build test bench fmt clippy clean watch play lint pre-commit ci info

RUSTFLAGS := RUSTFLAGS="-C target-cpu=native"

.DEFAULT_GOAL := install

install: ## Build with native CPU optimizations and install to ~/.cargo/bin
	$(RUSTFLAGS) cargo install --path .
	@which chess

build: ## Build release with native CPU optimizations (no install)
	$(RUSTFLAGS) cargo build --release

dev: ## Build development version
	cargo build

test: ## Run all tests
	cargo test

bench: ## Run benchmarks
	cargo bench

fmt: ## Format code
	cargo fmt

fmt-check: ## Check formatting
	cargo fmt -- --check

clippy: ## Run clippy
	cargo clippy -- -D warnings

lint: fmt-check clippy ## Run format check and clippy

clean: ## Clean build artifacts
	cargo clean

watch: install ## Watch engine play itself
	chess watch --depth 4

play: install ## Play against engine
	chess play --color white --depth 4

benchmark-alpha-beta: install ## Run alpha-beta benchmark
	chess benchmark-alpha-beta --depth 4

profile: ## Profile with flamegraph (requires sudo)
	sudo cargo flamegraph --bench pvp_benchmark

pre-commit: lint test ## Run pre-commit checks

ci: lint test ## Run CI checks

info: ## Show installed binary info
	@which chess
	@du -h $$(which chess)

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}'
