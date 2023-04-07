# dusk-merkle

A sparsely populated Merkle [`Tree`], parametrized over its height and arity.
```text
Height 0       o
             /   \
Height 1    o     o 
           / \   / \
Height 2  o   x x   x 
```

## Usage
```rust
use dusk_merkle::{Tree, Aggregator};

struct TestAggregator;
impl Aggregator for TestAggregator {
    type Item = u8;

    fn zero_item(_height: usize) -> Self::Item {
        0
    }

    fn aggregate<'a, I>(items: I) -> Self::Item
        where
            Self::Item: 'a,
            I: IntoIterator<Item = &'a Self::Item>,
    {
        items
            .into_iter()
            .fold(0, |acc, x| u8::wrapping_add(acc, *x))
    }
}

const HEIGHT: usize = 3;
const ARITY: usize = 2;

let mut tree = Tree::<TestAggregator, HEIGHT, ARITY>::new();

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
