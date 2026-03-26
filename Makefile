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

fmt: ## Format code (requires nightly)
	@cargo +nightly fmt --all

check: ## Run cargo check
	@cargo check

doc: ## Generate documentation
	@cargo doc --no-deps

clean: ## Clean build artifacts
	@cargo clean

.PHONY: help test no-std clippy fmt check doc clean
