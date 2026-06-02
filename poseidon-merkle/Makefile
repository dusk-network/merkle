help: ## Display this help screen
	@grep -h -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

BLST_CC ?= clang
BLST_CFLAGS ?= --target=thumbv6m-none-eabi -mthumb

test: ## Run poseidon-merkle tests
	@cargo test --release -p poseidon-merkle --features=bls-backend-blst,zk,rkyv-impl,size_32

no-std: ## Verify no_std compatibility on bare-metal target
	@rustup target add thumbv6m-none-eabi
	@CC_thumbv6m_none_eabi=$(BLST_CC) CFLAGS_thumbv6m_none_eabi="$(BLST_CFLAGS)" cargo build --release -p poseidon-merkle --no-default-features --features=bls-backend-blst --target thumbv6m-none-eabi

clippy: ## Run clippy
	@cargo clippy -p poseidon-merkle --features=bls-backend-blst,zk,rkyv-impl,size_32 -- -D warnings
	@cargo clippy -p poseidon-merkle --no-default-features --features=bls-backend-blst -- -D warnings

.PHONY: help test no-std clippy
