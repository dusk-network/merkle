// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{MerkleAggregator, MerkleTree, Node};

use core::mem::MaybeUninit;
use core::ptr;

#[cfg(feature = "rkyv-impl")]
use bytecheck::CheckBytes;
#[cfg(feature = "rkyv-impl")]
use rkyv::{Archive, Deserialize, Serialize};

/// An opening for a given position in a merkle tree.
#[cfg_attr(
    feature = "rkyv-impl",
    derive(Archive, Serialize, Deserialize),
    archive_attr(derive(CheckBytes))
)]
#[allow(clippy::module_name_repetitions)]
pub struct MerkleOpening<
    A: MerkleAggregator,
    const HEIGHT: usize,
    const ARITY: usize,
> {
    // The root is included in the branch, and is kept at position (0, 0).
    branch: [[A::Item; ARITY]; HEIGHT],
    positions: [usize; HEIGHT],
}

impl<A: MerkleAggregator, const HEIGHT: usize, const ARITY: usize>
    MerkleOpening<A, HEIGHT, ARITY>
{
    /// # Panics
    /// If the given `position` is not in the `tree`.
    pub(crate) fn new(
        tree: &MerkleTree<A, HEIGHT, ARITY>,
        position: u64,
    ) -> Self
    where
        <A as MerkleAggregator>::Item: Clone,
    {
        let positions = [0; HEIGHT];
        let branch = zero_array(|h| zero_array(|_| A::zero_hash(h)));

        let mut opening = Self { branch, positions };
        fill_opening(&mut opening, &tree.root, HEIGHT, position);
        opening
    }
}

fn fill_opening<A: MerkleAggregator, const HEIGHT: usize, const ARITY: usize>(
    opening: &mut MerkleOpening<A, HEIGHT, ARITY>,
    node: &Node<A, HEIGHT, ARITY>,
    height: usize,
    position: u64,
) where
    <A as MerkleAggregator>::Item: Clone,
{
    // If we are at the leaf, we're already done.
    if height == HEIGHT - 1 {
        return;
    }

    let (child_index, child_pos) =
        Node::<A, HEIGHT, ARITY>::child_location(height, position);
    let child = node.children[child_index]
        .as_ref()
        .expect("There should be a child at this position");

    fill_opening(opening, child, height, child_pos);

    // The root is placed at `(0, 0)`
    if height == 0 {
        opening.branch[0][0] =
            node.hash.as_ref().expect("There should be a root").clone();
    }

    let index = height + 1;
    for (i, child) in node.children.iter().enumerate() {
        let hash = &mut opening.branch[index][i];

        match child {
            Some(c) => {
                *hash = c
                    .hash
                    .as_ref()
                    .expect("There should be a hash in the child")
                    .clone();
            }
            None => {
                *hash = A::zero_hash(height);
            }
        }
    }
    opening.positions[index] = child_index;
}

// [R, 0]                     0
// [H_ABCDEFGH, H_IJKLMNOP]   1
// [H_IJKL, H_MNOP]           0
// [H_IJ, H_KL]               1
// [H_K, H_L]                 0

fn zero_array<T, F, const N: usize>(closure: F) -> [T; N]
where
    F: Fn(usize) -> T,
{
    let mut array: [MaybeUninit<T>; N] =
        unsafe { MaybeUninit::uninit().assume_init() };

    for (i, elem) in array.iter_mut().enumerate() {
        elem.write(closure(i));
    }
    let array_ptr = array.as_ptr();

    // SAFETY: this is safe since we initialized all the array elements prior to
    // the read operation.

    unsafe { ptr::read(array_ptr.cast()) }
}
