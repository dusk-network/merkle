# dusk-merkle

A sparsely populated [`MerkleTree`], parametrized over its height and arity.
```text
       o
     /   \
    o     o
   / \   / \
  o   x o   x
```
## Usage
```rust
use dusk_merkle::{MerkleTree, MerkleAggregator};

struct TestAggregator;
impl MerkleAggregator for TestAggregator {
    type Item = u8;

    fn merkle_zero(_height: u32) -> Self::Item {
        0
    }

    fn merkle_hash<'a, I>(items: I) -> Self::Item
        where
            Self::Item: 'a,
            I: IntoIterator<Item = &'a Self::Item>,
    {
        items
            .into_iter()
            .fold(0, |acc, x| u8::wrapping_add(acc, *x))
    }
}

const HEIGHT: u32 = 3;
const ARITY: usize = 2;

let mut tree = MerkleTree::<TestAggregator, HEIGHT, ARITY>::new();

// No elements have been inserted so the root is `None`.
assert!(matches!(tree.root(), None));

tree.insert(4, [&21u8]);
tree.insert(7, [&21u8]);

// After elements have been inserted, there will be a root.
assert!(matches!(tree.root(), Some(root) if *root == 42));
```
## Limitations
The tree does not keep the pre-image of its leaves. As a consequence, leaves
can only be queried for their hash.

## License

This project is licensed under the Mozilla Public License, version 2.0. See the
[license](./LICENSE) file for more details.
