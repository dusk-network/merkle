help: ## Display this help screen
	@grep -h -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

test: ## Run poseidon-merkle tests
	@cargo test --release -p poseidon-merkle --features=zk,rkyv-impl,serde,size_32

no-std: ## Verify no_std compatibility on bare-metal target
	@rustup target add thumbv6m-none-eabi
	@cargo build --release -p poseidon-merkle --no-default-features --target thumbv6m-none-eabi

clippy: ## Run clippy
	@cargo clippy -p poseidon-merkle --all-targets --features=zk,rkyv-impl,serde,size_32 -- -D warnings
	@cargo clippy -p poseidon-merkle --all-targets --no-default-features -- -D warnings

.PHONY: help test no-std clippy
