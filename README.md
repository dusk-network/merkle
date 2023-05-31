# dusk-merkle

A sparsely populated Merkle [`Tree`], parametrized over its height and arity.
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
Height 2    h     x     h     x
           / \   / \   / \   / \
Height 3  h   x x   x x   h x   x
```

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

impl Aggregate<H, A> for U8 {
    const EMPTY_SUBTREES: [U8; H] = [EMPTY_ITEM; H];
    
    fn aggregate(items: [&Self; A]) -> Self
    {
        items.into_iter().fold(U8(0), |acc, n| U8(acc.0 + n.0))
    }
}

// Set the height and arity of the tree. 
const H: usize = 3;
const A: usize = 2;

let mut tree = Tree::<U8, H, A>::new();

// No elements have been inserted so the root is the first empty item.
assert_eq!(tree.root(), &U8::EMPTY_SUBTREES[0]);

tree.insert(4, 21);
tree.insert(7, 21);

// After elements have been inserted, the root will be modified.
assert_eq!(tree.root(), &U8(42));
```

## Benchmarks

An implementation of a Merkle tree using `blake3` as a hash is included with the
crate under a feature with the same name. Benchmarks are also included and can
be run using:

```shell
cargo bench --features=blake3,bench
```

This requires a nightly toolchain.

## License

This project is licensed under the Mozilla Public License, version 2.0. See the
[license](./LICENSE) file for more details.
