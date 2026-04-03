help: ## Display this help screen
	@grep -h -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

test: ## Run all tests
	@$(MAKE) -C ./dusk-merkle $@
	@$(MAKE) -C ./poseidon-merkle $@

no-std: ## Verify no_std compatibility on bare-metal target
	@$(MAKE) -C ./dusk-merkle $@
	@$(MAKE) -C ./poseidon-merkle $@

clippy: ## Run clippy
	@$(MAKE) -C ./dusk-merkle $@
	@$(MAKE) -C ./poseidon-merkle $@

cq: ## Run code quality checks (formatting + clippy)
	@$(MAKE) fmt CHECK=1
	@$(MAKE) clippy

fmt: ## Format code (requires nightly)
	@rustup component add --toolchain nightly rustfmt 2>/dev/null || true
	@cargo +nightly fmt --all $(if $(CHECK),-- --check,)

check: ## Run cargo check
	@cargo check

doc: ## Generate documentation
	@cargo doc --no-deps

clean: ## Clean build artifacts
	@cargo clean

.PHONY: help test no-std clippy cq fmt check doc clean
