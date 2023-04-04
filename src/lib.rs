#![doc = include_str!("../README.md")]
#![no_std]
#![deny(clippy::pedantic)]

extern crate alloc;
use alloc::boxed::Box;

/// A reducing function that takes a collection of items of a given type and
/// returns one item of the same type.
pub trait MerkleAggregator {
    /// The item processed by the aggregator.
    type Item;

    /// Returns the zero value to be used for a hash. This value can depend on
    /// the `height` it is necessary at.
    fn merkle_zero(height: u32) -> Self::Item;

    /// Aggregates the given `items`.
    fn merkle_hash<'a, I>(items: I) -> Self::Item
    where
        Self::Item: 'a,
        I: IntoIterator<Item = &'a Self::Item>;
}

#[derive(Debug, Clone)]
struct Node<A: MerkleAggregator, const ARITY: usize> {
    num_leaves: u64,
    hash: Option<A::Item>,
    children: [Option<Box<Node<A, ARITY>>>; ARITY],
}

impl<A, const ARITY: usize> Node<A, ARITY>
where
    A: MerkleAggregator,
{
    const INIT: Option<Box<Node<A, ARITY>>> = None;

    const fn new() -> Self {
        Self {
            num_leaves: 0,
            hash: None,
            children: [Self::INIT; ARITY],
        }
    }

    fn insert<'a, I>(&mut self, height: u32, position: u64, items: I)
    where
        A::Item: 'a,
        I: IntoIterator<Item = &'a A::Item>,
    {
        if height == 1 {
            self.hash = Some(A::merkle_hash(items));
            self.num_leaves += 1;
            return;
        }

        let child_cap = capacity(ARITY as u64, height - 1);

        // Casting to a `usize` should be fine, since the index should be within
        // the `[0, ARITY[` bound anyway.
        #[allow(clippy::cast_possible_truncation)]
        let child_index = (position / child_cap) as usize;
        let child_pos = position % child_cap;

        let child = &mut self.children[child_index];
        if child.is_none() {
            *child = Some(Box::new(Node::new()));
        }

        // We just inserted a child at the given index.
        let child = self.children[child_index].as_mut().unwrap();
        Self::insert(child, height - 1, child_pos, items);

        let merkle_zero = A::merkle_zero(height);
        let hash = A::merkle_hash(self.children.iter().map(|c| match c {
            None => &merkle_zero,
            Some(child) => child.hash.as_ref().unwrap(),
        }));

        self.hash = Some(hash);
        self.num_leaves += 1;
    }
}

const fn capacity(arity: u64, height: u32) -> u64 {
    u64::pow(arity, height)
}

/// A sparse Merkle tree.
pub struct MerkleTree<
    A: MerkleAggregator,
    const HEIGHT: u32,
    const ARITY: usize,
> {
    root: Node<A, ARITY>,
}

impl<A: MerkleAggregator, const HEIGHT: u32, const ARITY: usize>
    MerkleTree<A, HEIGHT, ARITY>
{
    /// Create a new merkle tree.
    #[must_use]
    pub const fn new() -> Self {
        Self { root: Node::new() }
    }

    /// Insert an `element` at the given `position` in the tree.
    ///
    /// # Panics
    /// If `position >= capacity`.
    pub fn insert<'a, I>(&mut self, position: u64, items: I)
    where
        A::Item: 'a,
        I: IntoIterator<Item = &'a A::Item>,
    {
        self.root.insert(HEIGHT, position, items);
    }

    /// Get the root of the merkle tree.
    pub fn root(&self) -> Option<&A::Item> {
        self.root.hash.as_ref()
    }

    /// Returns the number of elements that have been inserted into the tree.
    #[must_use]
    pub fn len(&self) -> u64 {
        self.root.num_leaves
    }

    /// Returns `true` if the tree is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The maximum number of leaves in the tree, i.e. its capacity.
    #[must_use]
    pub const fn capacity(&self) -> u64 {
        capacity(ARITY as u64, HEIGHT)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn new_node() {
        let node = Node::<TestAggregator, 4>::new();

        for child in &node.children {
            assert!(child.is_none(), "All children should start as `None`");
        }

        assert_eq!(node.hash, None, "The hash value should default to `None`");
    }

    #[test]
    fn tree_insertion() {
        const HEIGHT: u32 = 3;
        const ARITY: usize = 2;

        let mut tree = MerkleTree::<TestAggregator, HEIGHT, ARITY>::new();

        tree.insert(5, [&42u8]);
        tree.insert(5, [&42u8]);

        assert_eq!(tree.len(), 2, "Two items were inserted");
    }

    #[test]
    #[should_panic]
    fn tree_insertion_panic() {
        const HEIGHT: u32 = 3;
        const ARITY: usize = 2;

        let mut tree = MerkleTree::<TestAggregator, HEIGHT, ARITY>::new();

        tree.insert(tree.capacity(), [&42u8]);
    }
}
