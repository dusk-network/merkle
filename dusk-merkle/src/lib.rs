// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![doc = include_str!("../README.md")]
#![no_std]
#![deny(clippy::pedantic)]

extern crate alloc;

use core::mem::MaybeUninit;
use core::ptr;

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

pub(crate) fn init_array<T, F, const N: usize>(closure: F) -> [T; N]
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
