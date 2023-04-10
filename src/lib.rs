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

/// A type that can be produced by aggregating multiple instances of itself, at
/// certain heights of the tree.
pub trait Aggregate {
    /// Aggregate `items` to produce a single one at the given `height`.
    fn aggregate<'a, I>(height: usize, items: I) -> Self
    where
        Self: 'a,
        I: ExactSizeIterator<Item = Option<&'a Self>>;
}

#[cfg_attr(
    feature = "rkyv-impl",
    derive(Archive, Serialize, Deserialize),
    archive(bound(serialize = "__S: Serializer")),
    archive_attr(derive(CheckBytes), doc(hidden))
)]
#[doc(hidden)]
pub struct Node<T, const HEIGHT: usize, const ARITY: usize> {
    item: T,
    #[cfg_attr(feature = "rkyv-impl", omit_bounds)]
    children: [Option<Box<Node<T, HEIGHT, ARITY>>>; ARITY],
}

impl<T, const HEIGHT: usize, const ARITY: usize> Node<T, HEIGHT, ARITY>
where
    T: Aggregate,
{
    const INIT: Option<Box<Node<T, HEIGHT, ARITY>>> = None;

    const fn new(item: T) -> Self {
        debug_assert!(HEIGHT > 0, "Height must be larger than zero");
        debug_assert!(ARITY > 0, "Arity must be larger than zero");

        Self {
            item,
            children: [Self::INIT; ARITY],
        }
    }

    fn compute_item(&mut self, height: usize) {
        self.item = T::aggregate(
            height,
            self.children
                .iter()
                .map(|node| node.as_ref().map(|node| &node.item)),
        );
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

    fn insert(&mut self, height: usize, position: u64, item: impl Into<T>) {
        if height == HEIGHT {
            self.item = item.into();
            return;
        }

        let (child_index, child_pos) = Self::child_location(height, position);

        let child = &mut self.children[child_index];
        if child.is_none() {
            *child =
                Some(Box::new(Node::new(T::aggregate(height, [].into_iter()))));
        }

        // We just inserted a child at the given index.
        let child = self.children[child_index].as_mut().unwrap();
        Self::insert(child, height + 1, child_pos, item);

        self.compute_item(height);
    }

    /// Returns the removed element, together with if there are any siblings
    /// left in the branch.
    ///
    /// # Panics
    /// If an element does not exist at the given position.
    fn remove(&mut self, height: usize, position: u64) -> (T, bool) {
        if height == HEIGHT {
            let mut item = T::aggregate(height, [].into_iter());
            mem::swap(&mut self.item, &mut item);
            return (item, false);
        }

        let (child_index, child_pos) = Self::child_location(height, position);

        let child = self.children[child_index]
            .as_mut()
            .expect("There should be a child at this position");
        let (removed_item, child_has_children) =
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
            self.compute_item(height);
        }

        (removed_item, has_children)
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
pub struct Tree<T, const HEIGHT: usize, const ARITY: usize> {
    root: Node<T, HEIGHT, ARITY>,
    positions: BTreeSet<u64>,
    len: u64,
}

impl<T: Aggregate, const HEIGHT: usize, const ARITY: usize>
    Tree<T, HEIGHT, ARITY>
{
    /// Create a new merkle tree with the given initial `root`.
    #[must_use]
    pub const fn new(root: T) -> Self {
        Self {
            root: Node::new(root),
            positions: BTreeSet::new(),
            len: 0,
        }
    }

    /// Insert an `item` at the given `position` in the tree.
    ///
    /// # Panics
    /// If `position >= capacity`.
    pub fn insert(&mut self, position: u64, item: impl Into<T>) {
        self.root.insert(0, position, item);
        if self.positions.insert(position) {
            self.len += 1;
        }
    }

    /// Remove and return the item at the given `position` in the tree if it
    /// exists.
    pub fn remove(&mut self, position: u64) -> Option<T> {
        if !self.positions.contains(&position) {
            return None;
        }

        let (item, _) = self.root.remove(0, position);

        self.len -= 1;
        self.positions.remove(&position);

        if self.len == 0 {
            self.root.item = T::aggregate(HEIGHT, [].into_iter());
        }

        Some(item)
    }

    /// Returns the [`Opening`] for the given `position` if it exists.
    pub fn opening(&self, position: u64) -> Option<Opening<T, HEIGHT, ARITY>>
    where
        T: Clone,
    {
        if !self.positions.contains(&position) {
            return None;
        }
        Some(Opening::new(self, position))
    }

    /// Get the root of the merkle tree.
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

    impl Aggregate for u8 {
        fn aggregate<'a, I>(_: usize, items: I) -> Self
        where
            Self: 'a,
            I: ExactSizeIterator<Item = Option<&'a Self>>,
        {
            items.into_iter().fold(0, |acc, n| match n {
                Some(n) => acc + n,
                None => acc,
            })
        }
    }

    const HEIGHT: usize = 3;
    const ARITY: usize = 2;

    type TestTree = Tree<u8, HEIGHT, ARITY>;

    #[test]
    fn tree_insertion() {
        let mut tree = TestTree::new(0);

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
        let mut tree = TestTree::new(0);

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
            *tree.root(),
            0,
            "Since the tree is empty the root should be the first passed element"
        );
    }

    #[test]
    #[should_panic]
    fn tree_insertion_out_of_bounds() {
        let mut tree = TestTree::new(0);
        tree.insert(tree.capacity(), 42);
    }
}
