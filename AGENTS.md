# Merkle

Sparse Merkle tree implementation for the Dusk network. Workspace with two `no_std` crates: a generic tree with openings (proofs), and a Poseidon-hashed variant with optional ZK circuit support.

## Repository Map

```
merkle/
├── dusk-merkle/       # dusk-merkle — generic sparse Merkle tree with openings
├── poseidon-merkle/   # poseidon-merkle — Poseidon-hashed tree (arity 4), ZK circuits
└── Makefile           # Root Makefile delegating to member crates
```

### `dusk-merkle/` (`dusk-merkle`)

Generic sparse Merkle tree parameterized by leaf type `T`, height `H`, and arity `A`:

- **`src/tree.rs`** — `Tree<T, H, A>` struct with sparse storage and lazy hash computation
- **`src/opening.rs`** — Merkle opening (proof) generation and verification
- **`src/node.rs`** — Internal node with lazy hash computation

### `poseidon-merkle/` (`poseidon-merkle`)

Poseidon-hashed Merkle tree (arity 4) built on `dusk-merkle`:

- **`src/lib.rs`** — Type aliases for Poseidon-specific tree and opening
- **`src/zk.rs`** — ZK circuit gadgets for Merkle proof verification (feature-gated behind `zk`)

## Commands

```bash
make test          # Run all tests (dusk-merkle + poseidon-merkle, --release)
make no-std        # Verify bare-metal target compatibility (thumbv6m-none-eabi)
make clippy        # Run clippy on all crates
make fmt           # Format code (requires nightly toolchain)
make check         # Run cargo check
make doc           # Generate documentation
make clean         # Clean build artifacts
```

Tests use `--release` because `poseidon-merkle` depends on `dusk-plonk` (via the `zk` feature) — debug builds take extremely long for PLONK proofs.

## Feature Flags

### `dusk-merkle`

| Feature    | Description                                    | Default |
|------------|------------------------------------------------|---------|
| `rkyv-impl`| rkyv serialization with validation and alloc  | No      |
| `size_16`  | rkyv 16-bit pointer size (mutually exclusive) | No      |
| `size_32`  | rkyv 32-bit pointer size (mutually exclusive) | No      |
| `size_64`  | rkyv 64-bit pointer size (mutually exclusive) | No      |

### `poseidon-merkle`

| Feature    | Description                                    | Default |
|------------|------------------------------------------------|---------|
| `zk`       | PLONK circuit support via `dusk-plonk`        | No      |
| `rkyv-impl`| rkyv serialization (enables on bls12_381 and dusk-merkle too) | No |
| `size_16`  | rkyv 16-bit pointer size (mutually exclusive) | No      |
| `size_32`  | rkyv 32-bit pointer size (mutually exclusive) | No      |
| `size_64`  | rkyv 64-bit pointer size (mutually exclusive) | No      |

**Note:** The `size_*` features are mutually exclusive — `--all-features` will fail. Use specific feature combinations (e.g. `--features=rkyv-impl,size_32`).

## Architecture

### Sparse Merkle Tree

The tree is a **sparse** data structure — only populated leaves and their ancestor nodes are stored. Empty subtrees use a default hash. The tree supports:

- **Variable height and arity**: Parameterized at the type level (`H` for height, `A` for arity)
- **Lazy hash computation**: Internal nodes recompute hashes only when children change
- **Openings (proofs)**: A Merkle opening contains the sibling hashes along the path from a leaf to the root, enabling verification that a leaf belongs to the tree

### Poseidon Variant

`poseidon-merkle` specializes the generic tree with:

- **Poseidon hash** over the BLS12-381 scalar field (via `dusk-poseidon`)
- **Arity 4** — each internal node has 4 children
- **ZK circuit gadgets** (behind `zk` feature) for in-circuit Merkle proof verification using `dusk-plonk`

### Key Dependencies

- `dusk-bls12_381` — BLS12-381 scalar field (leaf type for Poseidon tree)
- `dusk-poseidon` — Poseidon hash function
- `dusk-plonk` — PLONK proving system (optional, `zk` feature)
- `dusk-bytes` — Canonical byte serialization
- `rkyv` — Zero-copy deserialization (optional, `rkyv-impl` feature)

## Conventions

- **`no_std`**: Both crates are `no_std`. Do not add `std` dependencies.
- **Serialization**: Use `dusk-bytes` for canonical byte encoding, `rkyv` for zero-copy deserialization (feature-gated).
- **Field ordering**: Do not reorder fields in `rkyv`-serializable structs — it breaks archive compatibility.
- **Edition 2024**: The workspace uses Rust edition 2024 with MSRV 1.85.
- **`--release` for tests**: Always use `--release` when running tests that exercise PLONK proofs (`poseidon-merkle` with `zk` feature).

## Elevated Care Zones

`poseidon-merkle` with the `zk` feature enabled is cryptographic code — Merkle proof verification circuits must be correct for the integrity of the Dusk note tree. Changes to `src/zk.rs` or the hash computation in either crate require careful review.

## Change Propagation

| Changed crate     | Also verify                                |
|--------------------|--------------------------------------------|
| `dusk-merkle`      | `poseidon-merkle`, `piecrust`, `rusk`      |
| `poseidon-merkle`  | `phoenix`, `rusk`                          |

## Git Conventions

- Default branch: `main`
- License: MPL-2.0

### Commit messages

Format: `<scope>: <Description>` — imperative mood, capitalize first word after colon.

**One commit per crate per concern.** Each commit touches exactly one crate and one logical concern. Never bundle changes to different crates in one commit, and don't mix unrelated changes within the same crate either. Order commits bottom-up through the dependency chain (`dusk-merkle` before `poseidon-merkle`).

Canonical scopes:

| Scope | Crate/Directory |
|-------|----------------|
| `dusk-merkle` | `dusk-merkle/` |
| `poseidon-merkle` | `poseidon-merkle/` |
| `workspace` | Root `Cargo.toml`, root Makefile |
| `ci` | `.github/workflows/` |
| `chore` | Makefile, rustfmt, etc. |

Examples:
- `dusk-merkle: Add rkyv support for Opening`
- `poseidon-merkle: Fix ZK circuit witness generation`
- `workspace: Update dusk dependencies`

### Changelog

Both crates have a `CHANGELOG.md`. Add entries under `[Unreleased]` using [keep-a-changelog](https://keepachangelog.com/) format. If a change traces to a GitHub issue, reference it as a link: `[#42](https://github.com/dusk-network/merkle/issues/42)`. Only link to GitHub issues — do not reference any other tracking system.
