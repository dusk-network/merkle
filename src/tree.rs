// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use alloc::collections::BTreeSet;
use core::cell::Ref;

#[cfg(feature = "rkyv-impl")]
use bytecheck::{CheckBytes, Error as BytecheckError};
#[cfg(feature = "rkyv-impl")]
use rkyv::{
    ser::Serializer, validation::ArchiveContext, Archive, Deserialize,
    Fallible, Serialize,
};

use crate::{capacity, Aggregate, Node, Opening, Walk};

/// A sparse Merkle tree.
#[derive(Debug, Clone, PartialEq, Eq)]
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
            root: Node::new(),
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
    pub fn remove(&mut self, position: u64) -> Option<T> {
        if !self.positions.contains(&position) {
            return None;
        }

        let (item, _) = self.root.remove(0, position);
        self.positions.remove(&position);

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
    pub fn root(&self) -> Ref<T> {
        self.root.item(0)
    }

    /// Returns the root of the smallest sub-tree that holds all the leaves.
    ///
    /// Returns None when the tree is empty. In this case use
    /// `T::EMPTY_SUBTREES[H - 1]` for the root instead.
    pub fn smallest_subtree(&self) -> (Option<Ref<T>>, usize) {
        let mut smallest_node = &self.root;
        let mut height = H;
        loop {
            let mut children = smallest_node.children.iter().flatten();
            match children.next() {
                // when the root has no children, the tree is empty.
                // In this case the node that holds the smallest subtree doesn't
                // exist and we can not return it.
                None => return (None, 0),
                Some(child) => {
                    // if there is no more than one child and we are not at the
                    // end of the tree, we need to continue to traverse
                    if children.next().is_none() && height > 1 {
                        smallest_node = child;
                    }
                    // otherwise we return the item of the current node and the
                    // current height as the root and height of the smallest
                    // subtree
                    else {
                        return (Some(smallest_node.item(0)), height);
                    }
                }
            }
            height -= 1;
        }
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

    type SumTree = Tree<u8, H, A>;

    #[test]
    fn tree_insertion() {
        let mut tree = SumTree::new();

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
        let mut tree = SumTree::new();

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

        assert_eq!(*tree.root(), 42);

        tree.remove(6);
        assert!(tree.is_empty(), "The tree should be empty");
        assert_eq!(
            *tree.root(),
            u8::EMPTY_SUBTREES[0],
            "Since the tree is empty the root should be the first empty item"
        );
    }

    #[test]
    #[should_panic]
    fn tree_insertion_out_of_bounds() {
        let mut tree = SumTree::new();
        tree.insert(tree.capacity(), 42);
    }

    // create test tree for shrunken root:

    type RangeTree = Tree<Option<Range>, H, A>;

    // min and max are either
    #[derive(Debug, Clone, Copy, PartialEq)]
    struct Range {
        min: u64,
        max: u64,
    }

    impl Range {
        pub fn new(min: u64, max: u64) -> Self {
            Range { min, max }
        }
    }

    impl Aggregate<H, A> for Option<Range> {
        const EMPTY_SUBTREES: [Self; H] = [None; H];

        fn aggregate(items: [&Self; A]) -> Self {
            let mut bh_range = None;

            for item in items {
                bh_range = match (bh_range, item.as_ref()) {
                    (None, None) => None,
                    (None, Some(r)) => Some(*r),
                    (Some(r), None) => Some(r),
                    (Some(bh_range), Some(item_bh_range)) => {
                        let min =
                            core::cmp::min(item_bh_range.min, bh_range.min);
                        let max =
                            core::cmp::max(item_bh_range.max, bh_range.max);
                        Some(Range { min, max })
                    }
                };
            }

            bh_range
        }
    }

    #[test]
    fn smallest_subtree() {
        let mut tree = RangeTree::new();
        let (smallest_subtree, height) = tree.smallest_subtree();
        assert!(smallest_subtree.is_none());
        assert_eq!(height, 0);
        drop(smallest_subtree);

        tree.insert(0, Some(Range::new(0, 0)));
        let (smallest_subtree, height) = tree.smallest_subtree();
        assert_eq!(*smallest_subtree.unwrap(), Some(Range::new(0, 0)));
        assert_eq!(height, 1);

        tree.insert(1, Some(Range::new(1, 1)));
        let (smallest_subtree, height) = tree.smallest_subtree();
        assert_eq!(*smallest_subtree.unwrap(), Some(Range::new(0, 1)));
        assert_eq!(height, 1);

        tree.insert(2, Some(Range::new(2, 2)));
        let (smallest_subtree, height) = tree.smallest_subtree();
        assert_eq!(*smallest_subtree.unwrap(), Some(Range::new(0, 2)));
        assert_eq!(height, 2);

        tree.insert(3, Some(Range::new(3, 3)));
        let (smallest_subtree, height) = tree.smallest_subtree();
        assert_eq!(*smallest_subtree.unwrap(), Some(Range::new(0, 3)));
        assert_eq!(height, 2);

        tree.insert(7, Some(Range::new(7, 7)));
        let (smallest_subtree, height) = tree.smallest_subtree();
        assert_eq!(*smallest_subtree.unwrap(), Some(Range::new(0, 7)));
        assert_eq!(height, 3);

        tree.remove(0);
        tree.remove(1);
        tree.remove(2);
        let (smallest_subtree, height) = tree.smallest_subtree();
        assert_eq!(*smallest_subtree.unwrap(), Some(Range::new(3, 7)));
        assert_eq!(height, 3);

        tree.remove(3);
        tree.insert(4, Some(Range::new(4, 4)));
        let (smallest_subtree, height) = tree.smallest_subtree();
        assert_eq!(*smallest_subtree.unwrap(), Some(Range::new(4, 7)));
        assert_eq!(height, 2);

        tree.remove(4);
        let (smallest_subtree, height) = tree.smallest_subtree();
        assert_eq!(*smallest_subtree.unwrap(), Some(Range::new(7, 7)));
        assert_eq!(height, 1);

        tree.remove(7);
        let (smallest_subtree, height) = tree.smallest_subtree();
        assert!(smallest_subtree.is_none());
        assert_eq!(height, 0);
    }
}
