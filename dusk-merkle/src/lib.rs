// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![doc = include_str!("../README.md")]
#![no_std]
#![deny(clippy::pedantic)]

extern crate alloc;

mod node;
mod opening;
mod tree;
mod walk;

pub use node::*;
pub use opening::*;
pub use tree::*;
pub use walk::*;

/// A type that can be produced by aggregating `A` instances of itself.
pub trait Aggregate<const A: usize> {
    /// The value used in place of an empty subtree.
    const EMPTY_SUBTREE: Self;

    /// Aggregate the given array of item references to return a single item.
    fn aggregate(items: [&Self; A]) -> Self;
}

// Implement aggregate for an item with empty data
impl<const A: usize> Aggregate<A> for () {
    const EMPTY_SUBTREE: Self = ();
    fn aggregate(_: [&Self; A]) -> Self {}
}

/// Returns the capacity of a node at a given depth in the tree.
const fn capacity(arity: u64, depth: usize) -> u64 {
    // (Down)casting to a `u32` should be ok, since height shouldn't ever become
    // that large.
    #[allow(clippy::cast_possible_truncation)]
    u64::pow(arity, depth as u32)
}
