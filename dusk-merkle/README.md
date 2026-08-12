# dusk-merkle

A sparsely populated Merkle Tree, parametrized over its height and arity.
```text
Height 0             h
                    / \
                   /   \
                  /     \
                 /       \
                /         \
Height 1       h           h
              / \         / \
             /   \       /   \
Height 2    h     x     h     h
           / \         / \   / \
Height 3  h   x       x   h h   h
Position  0               5 6   7
```
The `Aggregate` trait defines how to calculate a parent from its children.
There is no restrictions on the way the children are aggregated, it can be done
with a hash function or any other custom aggregation.
Empty subtrees (noted as `x` in the tree above) are filled with the constant
`EMPTY_SUBTREE` from `Aggregate`.

Here an example where the parent is the sum of its children:

## Usage
```rust
use dusk_merkle::{Tree, Aggregate};

#[derive(Debug, Clone, Copy, PartialEq)]
struct U8(u8);

impl From<u8> for U8 {
    fn from(n: u8) -> Self {
        Self(n)
    }
}

const EMPTY_ITEM: U8 = U8(0);

impl Aggregate<A> for U8 {
    const EMPTY_SUBTREE: U8 = EMPTY_ITEM;

    fn aggregate(items: [&Self; A]) -> Self
    {
        items.into_iter().fold(U8(0), |acc, c| U8(acc.0 + c.0))
    }
}

// Set the height and arity of the tree. 
const H: usize = 3;
const A: usize = 2;

let mut tree = Tree::<U8, H, A>::new();

// No elements have been inserted so the root is the empty subtree.
assert_eq!(*tree.root(), U8::EMPTY_SUBTREE);

tree.insert(4, 21);
tree.insert(7, 21);

// After elements have been inserted, the root will be modified.
assert_eq!(*tree.root(), U8(42));
```

An implementation of a Merkle tree using the `blake3` hash algorithm is included
as an example.

Another implementation of a Merkle tree with the `poseidon252` hash and the
creation of the opening proof in zero-knowledge using PLONK is included as a
member of this workspace `poseidon_merkle`.

## Benchmarks

Benchmarks are also included and can be run using:

For the `blake3` tree:
```shell
cargo bench
```

For the `poseidon` tree:
```shell
cargo bench -p poseidon-merkle
```

For the opening proof creation in zero-knowledge:
```shell
cargo bench -p poseidon-merkle --features zk
```

To compare the first root read after checked RKYV deserialization with a warmed
tree after one or four leaf updates:
```shell
cargo bench -p dusk-merkle --bench blake3 \
  --features rkyv-impl,size_32 -- blake3_first_root
```
The benchmark times `root()` separately from decode and also reports the
combined decode-and-first-root cost. Tree construction, checked decode for the
root-only case, leaf updates, and destruction are outside the timed operation.

Large wasm64 page trees with 10, 50, and 100 GiB of densely packed or scattered
populated pages can be measured separately using:
```shell
cargo bench -p dusk-merkle --bench blake3 \
  --features rkyv-impl,size_32 -- blake3_wasm64_first_root
```
This benchmark materializes only the page hashes and Merkle nodes, not the
corresponding contract-memory bytes.

The cold-root cost follows the number and distribution of populated pages, not
only the declared memory length. Widely scattered pages share fewer internal
nodes than densely packed pages and can make the first root after decoding
substantially more expensive. Large wasm64 users should measure their expected
occupancy: a 100 GiB scenario can take hundreds of milliseconds to rebuild on
current desktop hardware when its pages are spread across the address space.

## Checked RKYV deserialization

With the `rkyv-impl` feature, checked tree deserialization deliberately discards
all archived internal aggregate caches. Structural validation establishes the
shape, leaves, and recorded positions of the tree, but cannot establish that an
arbitrary cached aggregate was derived from those leaves. The first aggregate
read therefore recomputes the populated internal nodes from leaf values; later
reads use the lazily rebuilt caches until an update dirties their paths.

## Implementations

A merkle tree using the poseidon hash function for aggregation and plonk to
generate an opening proof in zero-knowledge can be found in the same workspace
under 'poseidon-merkle'.

## License

This project is licensed under the Mozilla Public License, version 2.0. See the
[license](./LICENSE) file for more details.
