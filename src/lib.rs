// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![doc = include_str!("../README.md")]
#![no_std]
#![deny(clippy::pedantic)]

extern crate alloc;

use alloc::boxed::Box;
use alloc::collections::BTreeSet;

use core::mem;
use core::mem::MaybeUninit;
use core::ptr;

#[cfg(feature = "rkyv-impl")]
use bytecheck::{CheckBytes, Error as BytecheckError};
#[cfg(feature = "rkyv-impl")]
use rkyv::{
    ser::Serializer, validation::ArchiveContext, Archive, Deserialize,
    Fallible, Serialize,
};

#[cfg(feature = "blake3")]
pub mod blake3;
#[cfg(feature = "poseidon")]
pub mod poseidon;

mod opening;
mod walk;

pub use opening::*;
pub use walk::*;

/// A type that can be produced by aggregating multiple instances of itself, at
/// certain heights of the tree.
pub trait Aggregate<const H: usize, const A: usize>: Copy {
    /// The items to be used for a given empty subtree at the given height.
    const EMPTY_SUBTREES: [Self; H];

    /// Aggregate the given `items` to produce a single one. The given iterator
    /// is guaranteed to produce `A` number of items, from the leftmost to the
    /// rightmost child of a tree's node.
    fn aggregate<'a, I>(items: I) -> Self
    where
        Self: 'a,
        I: Iterator<Item = &'a Self>;
}

// Implement aggregate for an item with empty data
impl<const H: usize, const A: usize> Aggregate<H, A> for () {
    const EMPTY_SUBTREES: [(); H] = [(); H];

    fn aggregate<'a, I>(_: I) -> Self
    where
        Self: 'a,
        I: Iterator<Item = &'a Self>,
    {
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "rkyv-impl",
    derive(Archive, Serialize, Deserialize),
    archive(bound(serialize = "__S: Serializer")),
    archive_attr(
        derive(CheckBytes),
        doc(hidden),
        check_bytes(
            bound = "__C: ArchiveContext, <__C as Fallible>::Error: BytecheckError"
        )
    )
)]
#[doc(hidden)]
pub struct Node<T, const H: usize, const A: usize> {
    item: T,
    #[cfg_attr(feature = "rkyv-impl", omit_bounds, archive_attr(omit_bounds))]
    children: [Option<Box<Node<T, H, A>>>; A],
}

impl<T, const H: usize, const A: usize> Node<T, H, A>
where
    T: Aggregate<H, A>,
{
    const INIT_NODE: Option<Box<Node<T, H, A>>> = None;

    const fn new(item: T) -> Self {
        debug_assert!(H > 0, "Height must be larger than zero");
        debug_assert!(A > 0, "Arity must be larger than zero");

        Self {
            item,
            children: [Self::INIT_NODE; A],
        }
    }

    fn compute_item(&mut self, height: usize) {
        let empty = &T::EMPTY_SUBTREES[height];

        self.item = T::aggregate(
            self.children
                .iter()
                .map(|node| node.as_ref().map(|node| &node.as_ref().item))
                .map(|item| item.unwrap_or(empty)),
        );
    }

    fn child_location(height: usize, position: u64) -> (usize, u64) {
        let child_cap = capacity(A as u64, H - height - 1);

        // Casting to a `usize` should be fine, since the index should be within
        // the `[0, A[` bound anyway.
        #[allow(clippy::cast_possible_truncation)]
        let child_index = (position / child_cap) as usize;
        let child_pos = position % child_cap;

        (child_index, child_pos)
    }

    fn insert(&mut self, height: usize, position: u64, item: impl Into<T>) {
        if height == H {
            self.item = item.into();
            return;
        }

        let (child_index, child_pos) = Self::child_location(height, position);

        let child = &mut self.children[child_index];
        if child.is_none() {
            *child = Some(Box::new(Node::new(T::EMPTY_SUBTREES[height])));
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
        if height == H {
            let mut item = T::EMPTY_SUBTREES[0];
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

pub(crate) fn empty_nodes<T, F, const N: usize>(closure: F) -> [T; N]
where
    F: Fn(usize) -> T,
{
    let mut array: [MaybeUninit<T>; N] =
        unsafe { MaybeUninit::uninit().assume_init() };

    let mut i = 0;
    while i < N {
        array[i].write(closure(i));
        i += 1;
    }
    let array_ptr = array.as_ptr();

    // SAFETY: this is safe since we initialized all the array elements prior to
    // the read operation.
    unsafe { ptr::read(array_ptr.cast()) }
}

/// Returns the capacity of a node at a given depth in the tree.
const fn capacity(arity: u64, depth: usize) -> u64 {
    // (Down)casting to a `u32` should be ok, since height shouldn't ever become
    // that large.
    #[allow(clippy::cast_possible_truncation)]
    u64::pow(arity, depth as u32)
}

/// A sparse Merkle tree.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "rkyv-impl",
    derive(Archive, Serialize, Deserialize),
    archive_attr(derive(CheckBytes))
)]
pub struct Tree<T, const H: usize, const A: usize> {
    root: Node<T, H, A>,
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

        fn aggregate<'a, I>(items: I) -> Self
        where
            Self: 'a,
            I: Iterator<Item = &'a Self>,
        {
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
