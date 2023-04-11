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
use dusk_merkle::{Tree, Aggregate};

#[derive(Debug, PartialEq)]
struct U8(u8);

impl From<u8> for U8 {
    fn from(n: u8) -> Self {
        Self(n)
    }
}

impl Aggregate for U8 {
    fn aggregate<'a, I>(_: usize, items: I) -> Self
        where
            Self: 'a,
            I: ExactSizeIterator<Item = Option<&'a Self>>,
    {
        items.into_iter().fold(U8(0), |acc, n| match n {
            Some(n) => U8(acc.0 + n.0),
            None => acc,
        })
    }
}

const H: usize = 3;
const A: usize = 2;

let mut tree = Tree::<U8, H, A>::new();

// No elements have been inserted so the root is `None`.
assert_eq!(tree.root(), None);

tree.insert(4, 21);
tree.insert(7, 21);

// After elements have been inserted, the root will be `Some`.
assert!(matches!(tree.root(), Some(n) if n == &U8(42)));
```

## License

This project is licensed under the Mozilla Public License, version 2.0. See the
[license](./LICENSE) file for more details.
