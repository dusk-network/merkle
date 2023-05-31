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

#[cfg(feature = "blake3")]
pub mod blake3;
#[cfg(feature = "poseidon")]
pub mod poseidon;

mod node;
mod opening;
mod tree;
mod walk;

pub use node::*;
pub use opening::*;
pub use tree::*;
pub use walk::*;

/// A type that can be produced by aggregating multiple instances of itself, at
/// certain heights of the tree.
pub trait Aggregate<const H: usize, const A: usize>: Copy {
    /// The items to be used for a given empty subtree at the given height.
    const EMPTY_SUBTREES: [Self; H];

    /// Aggregate the given array of item references to return a single item.
    fn aggregate(items: [&Self; A]) -> Self;
}

// Implement aggregate for an item with empty data
impl<const H: usize, const A: usize> Aggregate<H, A> for () {
    const EMPTY_SUBTREES: [(); H] = [(); H];
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
