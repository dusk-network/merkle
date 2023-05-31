// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use alloc::collections::BTreeSet;

#[cfg(feature = "rkyv-impl")]
use bytecheck::{CheckBytes, Error as BytecheckError};
#[cfg(feature = "rkyv-impl")]
use rkyv::{
    ser::Serializer, validation::ArchiveContext, Archive, Deserialize,
    Fallible, Serialize,
};

use crate::{capacity, Aggregate, Node, Opening, Walk};

/// A sparse Merkle tree.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "rkyv-impl",
    derive(Archive, Serialize, Deserialize),
    archive_attr(derive(CheckBytes))
)]
pub struct Tree<T, const H: usize, const A: usize> {
    pub(crate) root: Node<T, H, A>,
    positions: BTreeSet<u64>,
}

impl<T, const H: usize, const A: usize> Tree<T, H, A>
where
    T: Aggregate<H, A>,
{
    /// Create a new merkle tree with the given initial `root`.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            root: Node::new(T::EMPTY_SUBTREES[0]),
            positions: BTreeSet::new(),
        }
    }

    /// Insert an `item` at the given `position` in the tree.
    ///
    /// # Panics
    /// If `position >= capacity`.
    pub fn insert(&mut self, position: u64, item: impl Into<T>) {
        self.root.insert(0, position, item);
        self.positions.insert(position);
    }

    /// Remove and return the item at the given `position` in the tree if it
    /// exists.
    // Allowing for missing docs on panic, since panic is impossible. See
    // comment below.
    #[allow(clippy::missing_panics_doc)]
    pub fn remove(&mut self, position: u64) -> Option<T> {
        if !self.positions.contains(&position) {
            return None;
        }

        let (item, _) = self.root.remove(0, position);

        self.positions.remove(&position);
        if self.positions.is_empty() {
            self.root.item = T::EMPTY_SUBTREES[0];
        }

        Some(item)
    }

    /// Returns the [`Opening`] for the given `position` if it exists.
    pub fn opening(&self, position: u64) -> Option<Opening<T, H, A>> {
        if !self.positions.contains(&position) {
            return None;
        }
        Some(Opening::new(self, position))
    }

    /// Returns a [`Walk`] through the tree, proceeding according to the
    /// `walker` function.
    ///
    /// A walk starts from the root of the tree, and "drills down" according to
    /// the output of the walker function. The function should return `true` or
    /// `false`, indicating whether the iterator should continue along the
    /// tree's path.
    pub fn walk<W>(&self, walker: W) -> Walk<T, W, H, A>
    where
        W: Fn(&T) -> bool,
    {
        Walk::new(self, walker)
    }

    /// Get the root of the merkle tree.
    ///
    /// It is none if the tree is empty.
    pub fn root(&self) -> &T {
        &self.root.item
    }

    /// Returns true if the tree contains a leaf at the given `position`.
    pub fn contains(&self, position: u64) -> bool {
        self.positions.contains(&position)
    }

    /// Returns the number of elements that have been inserted into the tree.
    #[must_use]
    pub fn len(&self) -> u64 {
        self.positions.len() as u64
    }

    /// Returns `true` if the tree is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The maximum number of leaves in the tree, i.e. its capacity.
    #[must_use]
    pub const fn capacity(&self) -> u64 {
        capacity(A as u64, H)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl Aggregate<H, A> for u8 {
        const EMPTY_SUBTREES: [Self; H] = [0; H];

        fn aggregate(items: [&Self; A]) -> Self {
            items.into_iter().sum()
        }
    }

    const H: usize = 3;
    const A: usize = 2;

    type TestTree = Tree<u8, H, A>;

    #[test]
    fn tree_insertion() {
        let mut tree = TestTree::new();

        tree.insert(5, 42);
        tree.insert(6, 42);
        tree.insert(5, 42);

        assert_eq!(
            tree.len(),
            2,
            "Three items were inserted, but one was in the same position as another"
        );
    }

    #[test]
    fn tree_deletion() {
        let mut tree = TestTree::new();

        tree.insert(5, 42);
        tree.insert(6, 42);
        tree.insert(5, 42);

        tree.remove(5);
        tree.remove(4);

        assert_eq!(
            tree.len(),
            1,
            "There should be one element left in the tree"
        );

        tree.remove(6);
        assert!(tree.is_empty(), "The tree should be empty");
        assert_eq!(
            tree.root(),
            &u8::EMPTY_SUBTREES[0],
            "Since the tree is empty the root should be the first empty item"
        );
    }

    #[test]
    #[should_panic]
    fn tree_insertion_out_of_bounds() {
        let mut tree = TestTree::new();
        tree.insert(tree.capacity(), 42);
    }
}
