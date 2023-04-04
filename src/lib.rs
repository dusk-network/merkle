//! A sparsely populated [`MerkleTree`], parametrized over its height and arity.
//!
//! ```text
//!        o
//!      /   \
//!     o     o
//!    / \   / \
//!   o   x o   x
//! ```
//!
//! # Usage
//!
//! ```rust
//! use dusk_merkle::{MerkleTree, ToMerkleInputs, MerkleHash};
//!
//! struct TestHash;
//! impl MerkleHash for TestHash {
//!     type Hash = u8;
//!
//!     fn merkle_hash<I, const N: usize>(inputs: I) -> Self::Hash
//!     where
//!         I: ToMerkleInputs<Self::Hash, N>,
//!     {
//!         // Adding numbers is not a cryptographic hash
//!         let inputs = inputs.to_merkle_inputs();
//!         inputs.into_iter().fold(0, u8::wrapping_add)
//!     }
//! }
//!
//! const HEIGHT: u32 = 3;
//! const ARITY: usize = 2;
//!
//! let mut tree = MerkleTree::<TestHash, HEIGHT, ARITY>::new();
//!
//! // No elements have been inserted so the root is `None`.
//! assert!(matches!(tree.root(), None));
//!
//! tree.insert(4, &21u8);
//! tree.insert(7, &21u8);
//!
//! println!("{:?}", tree.root());
//! // After elements have been inserted, there will be a root.
//! assert!(matches!(tree.root(), Some(root) if *root == 42));
//! ```
//!
//! # Limitations
//!
//! The tree does not keep the pre-image of its leaves. As a consequence, leaves
//! can only be queried for their hash.

#![no_std]
#![deny(clippy::pedantic)]

extern crate alloc;
use alloc::boxed::Box;

/// A type that can be decomposed into multiple inputs to a hash.
pub trait ToMerkleInputs<T, const N: usize> {
    /// Returns the hash inputs associated with the type.
    fn to_merkle_inputs(&self) -> [T; N];
}

impl<T: Clone, const N: usize> ToMerkleInputs<T, N> for [T; N] {
    fn to_merkle_inputs(&self) -> [T; N] {
        self.clone()
    }
}

impl<T: Clone> ToMerkleInputs<T, 1> for T {
    fn to_merkle_inputs(&self) -> [T; 1] {
        [self.clone()]
    }
}

pub trait MerkleHash {
    type Hash;

    fn merkle_hash<I, const N: usize>(inputs: I) -> Self::Hash
    where
        I: ToMerkleInputs<Self::Hash, N>;
}

#[derive(Debug, Clone)]
struct Node<H: MerkleHash, const ARITY: usize> {
    num_leaves: u64,
    hash: Option<H::Hash>,
    children: [Option<Box<Node<H, ARITY>>>; ARITY],
}

impl<H, const ARITY: usize> Node<H, ARITY>
where
    H: MerkleHash,
    <H as MerkleHash>::Hash: Default + Copy,
{
    const INIT: Option<Box<Node<H, ARITY>>> = None;

    const fn new() -> Self {
        Self {
            num_leaves: 0,
            hash: None,
            children: [Self::INIT; ARITY],
        }
    }

    fn insert<T, const N: usize>(
        &mut self,
        height: u32,
        position: u64,
        element: &T,
    ) where
        T: ToMerkleInputs<H::Hash, N>,
    {
        if height == 1 {
            let inputs = element.to_merkle_inputs();
            self.hash = Some(H::merkle_hash(inputs));
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
        Self::insert(child, height - 1, child_pos, element);

        let mut hashes = [H::Hash::default(); ARITY];
        for (i, child) in self.children.iter().enumerate() {
            if let Some(child) = child {
                hashes[i] = child.hash.unwrap();
            }
        }

        self.hash = Some(H::merkle_hash(hashes));
        self.num_leaves += 1;
    }
}

const fn capacity(arity: u64, height: u32) -> u64 {
    u64::pow(arity, height)
}

/// A sparse Merkle tree.
pub struct MerkleTree<H: MerkleHash, const HEIGHT: u32, const ARITY: usize> {
    root: Node<H, ARITY>,
}

impl<H, const HEIGHT: u32, const ARITY: usize> MerkleTree<H, HEIGHT, ARITY>
where
    H: MerkleHash,
    <H as MerkleHash>::Hash: Default + Copy,
{
    /// Create a new merkle tree
    #[must_use]
    pub const fn new() -> Self {
        Self { root: Node::new() }
    }

    /// Insert an `element` at the given `position` in the tree.
    ///
    /// # Panics
    /// If `position >= capacity`.
    pub fn insert<T, const N: usize>(&mut self, position: u64, element: &T)
    where
        T: ToMerkleInputs<H::Hash, N>,
    {
        self.root.insert(HEIGHT, position, element);
    }

    /// Get the root of the merkle tree.
    pub fn root(&self) -> Option<&H::Hash> {
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

    struct TestHash;
    impl MerkleHash for TestHash {
        type Hash = u8;

        fn merkle_hash<I, const N: usize>(inputs: I) -> Self::Hash
        where
            I: ToMerkleInputs<Self::Hash, N>,
        {
            let inputs = inputs.to_merkle_inputs();
            inputs.into_iter().fold(0, u8::wrapping_add)
        }
    }

    #[test]
    fn new_node() {
        let node = Node::<TestHash, 4>::new();

        for child in &node.children {
            assert!(child.is_none(), "All children should start as `None`");
        }

        assert_eq!(node.hash, None, "The hash value should default to `None`");
    }

    #[test]
    fn tree_insertion() {
        const HEIGHT: u32 = 3;
        const ARITY: usize = 2;

        let mut tree = MerkleTree::<TestHash, HEIGHT, ARITY>::new();

        tree.insert(5, &42u8);
        tree.insert(5, &42u8);
    }

    #[test]
    #[should_panic]
    fn tree_insertion_panic() {
        const HEIGHT: u32 = 3;
        const ARITY: usize = 2;

        let mut tree = MerkleTree::<TestHash, HEIGHT, ARITY>::new();

        tree.insert(tree.capacity(), &42u8);
    }
}
