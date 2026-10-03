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

/// Returns the capacity of a tree of `height` and `arity`, or `None` when
/// the height is zero or above `u32::MAX`, the arity is below 2, or the
/// capacity does not fit a `u64`, which the index arithmetic relies on. With
/// an arity of at least 2, the capacity bound also caps the height at 63.
const fn checked_capacity(arity: usize, height: usize) -> Option<u64> {
    if height == 0 || arity < 2 || height > u32::MAX as usize {
        return None;
    }
    #[allow(clippy::cast_possible_truncation)] // bounded above
    let height = height as u32;
    (arity as u64).checked_pow(height)
}

/// Returns the capacity of a node at a given depth in the tree.
const fn capacity(arity: u64, depth: usize) -> u64 {
    // (Down)casting to a `u32` should be ok, since height shouldn't ever become
    // that large.
    #[allow(clippy::cast_possible_truncation)]
    u64::pow(arity, depth as u32)
}

#[cfg(test)]
mod tests {
    use super::checked_capacity;

    #[test]
    fn checked_capacity_bounds() {
        assert_eq!(checked_capacity(4, 17), Some(1 << 34));
        assert_eq!(checked_capacity(3, 40), Some(3u64.pow(40)));
        assert_eq!(checked_capacity(3, 41), None);
        assert_eq!(checked_capacity(2, 63), Some(1 << 63));
        assert_eq!(checked_capacity(2, 64), None);
        assert_eq!(checked_capacity(0, 17), None);
        assert_eq!(checked_capacity(1, 17), None);
        assert_eq!(checked_capacity(4, 0), None);
        #[cfg(target_pointer_width = "64")]
        assert_eq!(checked_capacity(2, u32::MAX as usize + 1), None);
    }
}
