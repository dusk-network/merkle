// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![doc = include_str!("../README.md")]
#![no_std]
#![deny(clippy::pedantic)]

mod opening;

extern crate alloc;

use alloc::boxed::Box;
use alloc::collections::BTreeSet;

use core::mem;

#[cfg(feature = "rkyv-impl")]
use bytecheck::CheckBytes;
#[cfg(feature = "rkyv-impl")]
use rkyv::{ser::Serializer, Archive, Deserialize, Serialize};

pub use opening::*;

/// A reducing function that takes a collection of items of a given type and
/// returns one item of the same type.
pub trait Aggregator {
    /// The item processed by the aggregator.
    type Item;

    /// Returns the zero value to be used for an item for a given tree `height`.
    fn zero_item(height: usize) -> Self::Item;

    /// Aggregates the given `items`.
    fn aggregate<'a, I>(items: I) -> Self::Item
    where
        Self::Item: 'a,
        I: IntoIterator<Item = &'a Self::Item>;
}

#[cfg_attr(
    feature = "rkyv-impl",
    derive(Archive, Serialize, Deserialize),
    archive(bound(serialize = "__S: Serializer")),
    archive_attr(derive(CheckBytes), doc(hidden))
)]
#[doc(hidden)]
pub struct Node<A: Aggregator, const HEIGHT: usize, const ARITY: usize> {
    hash: Option<A::Item>,
    #[cfg_attr(feature = "rkyv-impl", omit_bounds)]
    children: [Option<Box<Node<A, HEIGHT, ARITY>>>; ARITY],
}

impl<A, const HEIGHT: usize, const ARITY: usize> Node<A, HEIGHT, ARITY>
where
    A: Aggregator,
{
    const INIT: Option<Box<Node<A, HEIGHT, ARITY>>> = None;

    const fn new() -> Self {
        debug_assert!(HEIGHT > 0, "Height must be larger than zero");
        debug_assert!(ARITY > 0, "Arity must be larger than zero");

        Self {
            hash: None,
            children: [Self::INIT; ARITY],
        }
    }

    fn compute_hash(&mut self, height: usize) {
        let merkle_zero = A::zero_item(height);
        let hash = A::aggregate(self.children.iter().map(|c| match c {
            None => &merkle_zero,
            Some(child) => child.hash.as_ref().unwrap(),
        }));
        self.hash = Some(hash);
    }

    fn child_location(height: usize, position: u64) -> (usize, u64) {
        let child_cap = capacity(ARITY as u64, HEIGHT - height - 1);

        // Casting to a `usize` should be fine, since the index should be within
        // the `[0, ARITY[` bound anyway.
        #[allow(clippy::cast_possible_truncation)]
        let child_index = (position / child_cap) as usize;
        let child_pos = position % child_cap;

        (child_index, child_pos)
    }

    fn insert<'a, I>(&mut self, height: usize, position: u64, items: I)
    where
        A::Item: 'a,
        I: IntoIterator<Item = &'a A::Item>,
    {
        if height == HEIGHT {
            self.hash = Some(A::aggregate(items));
            return;
        }

        let (child_index, child_pos) = Self::child_location(height, position);

        let child = &mut self.children[child_index];
        if child.is_none() {
            *child = Some(Box::new(Node::new()));
        }

        // We just inserted a child at the given index.
        let child = self.children[child_index].as_mut().unwrap();
        Self::insert(child, height + 1, child_pos, items);

        self.compute_hash(height);
    }

    /// Returns the hash of the removed element, together with if there are any
    /// siblings left in the branch.
    ///
    /// # Panics
    /// If an element does not exist at the given position.
    fn remove(&mut self, height: usize, position: u64) -> (A::Item, bool) {
        if height == HEIGHT {
            let mut hash = Some(A::zero_item(height));
            mem::swap(&mut self.hash, &mut hash);
            return (
                hash.expect("There should be an item at this position"),
                false,
            );
        }

        let (child_index, child_pos) = Self::child_location(height, position);

        let child = self.children[child_index]
            .as_mut()
            .expect("There should be a child at this position");
        let (removed_hash, child_has_children) =
            Self::remove(child, height + 1, child_pos);

        if !child_has_children {
            self.children[child_index] = None;
        }

        let mut has_children = false;
        for child in &self.children {
            if child.is_some() {
                has_children = true;
                break;
            }
        }

        if has_children {
            self.compute_hash(height);
        }

        (removed_hash, has_children)
    }
}

/// Returns the capacity of a node at a given depth in the tree.
const fn capacity(arity: u64, depth: usize) -> u64 {
    // (Down)casting to a `u32` should be ok, since height shouldn't ever become
    // that large.
    #[allow(clippy::cast_possible_truncation)]
    u64::pow(arity, depth as u32)
}

/// A sparse Merkle tree.
#[cfg_attr(
    feature = "rkyv-impl",
    derive(Archive, Serialize, Deserialize),
    archive_attr(derive(CheckBytes))
)]
pub struct Tree<A: Aggregator, const HEIGHT: usize, const ARITY: usize> {
    root: Node<A, HEIGHT, ARITY>,
    positions: BTreeSet<u64>,
    len: u64,
}

impl<A: Aggregator, const HEIGHT: usize, const ARITY: usize>
    Tree<A, HEIGHT, ARITY>
{
    /// Create a new merkle tree.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            root: Node::new(),
            positions: BTreeSet::new(),
            len: 0,
        }
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
        self.root.insert(0, position, items);
        if self.positions.insert(position) {
            self.len += 1;
        }
    }

    /// Remove and return the item of the leaf at the given `position` in the
    /// tree.
    pub fn remove(&mut self, position: u64) -> Option<A::Item> {
        if !self.positions.contains(&position) {
            return None;
        }

        let (hash, _) = self.root.remove(0, position);

        self.len -= 1;
        self.positions.remove(&position);

        if self.len == 0 {
            self.root.hash = None;
        }

        Some(hash)
    }

    /// Returns the [`Opening`] for the given `position`.
    pub fn opening(&self, position: u64) -> Option<Opening<A, HEIGHT, ARITY>>
    where
        <A as Aggregator>::Item: Clone,
    {
        if !self.positions.contains(&position) {
            return None;
        }
        Some(Opening::new(self, position))
    }

    /// Get the root of the merkle tree.
    pub fn root(&self) -> Option<&A::Item> {
        self.root.hash.as_ref()
    }

    /// Returns true if the tree contains a leaf at the given `position`.
    pub fn contains(&self, position: u64) -> bool {
        self.positions.contains(&position)
    }

    /// Returns the number of elements that have been inserted into the tree.
    #[must_use]
    pub fn len(&self) -> u64 {
        self.len
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

    #[test]
    fn tree_insertion() {
        const HEIGHT: usize = 3;
        const ARITY: usize = 2;

        let mut tree = Tree::<TestAggregator, HEIGHT, ARITY>::new();

        tree.insert(5, [&42u8]);
        tree.insert(6, [&42u8]);
        tree.insert(5, [&42u8]);

        assert_eq!(
            tree.len(),
            2,
            "Three items were inserted, but one was in the same position as another"
        );
    }

    #[test]
    fn tree_deletion() {
        const HEIGHT: usize = 3;
        const ARITY: usize = 2;

        let mut tree = Tree::<TestAggregator, HEIGHT, ARITY>::new();

        tree.insert(5, [&42u8]);
        tree.insert(6, [&42u8]);
        tree.insert(5, [&42u8]);

        tree.remove(5);
        tree.remove(4);

        assert_eq!(
            tree.len(),
            1,
            "There should be one element left in the tree"
        );

        tree.remove(6);
        assert!(tree.is_empty(), "The tree should be empty");
        assert!(
            matches!(tree.root(), None),
            "Since the tree is empty the root should be `None`"
        );
    }

    #[test]
    #[should_panic]
    fn tree_insertion_out_of_bounds() {
        const HEIGHT: usize = 3;
        const ARITY: usize = 2;

        let mut tree = Tree::<TestAggregator, HEIGHT, ARITY>::new();

        tree.insert(tree.capacity(), [&42u8]);
    }
}
