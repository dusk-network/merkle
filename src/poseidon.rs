// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#[cfg(feature = "zk")]
mod zk;

mod zero_items;
use zero_items::zero_items;

use dusk_bls12_381::BlsScalar;
use dusk_poseidon::sponge::hash as poseidon_hash;

use crate::Aggregate;

#[cfg(feature = "rkyv-impl")]
use bytecheck::CheckBytes;
#[cfg(feature = "rkyv-impl")]
use rkyv::{Archive, Deserialize, Serialize};

/// The Poseidon Merkle Tree
pub type Tree<T, const H: usize, const A: usize> = crate::Tree<Item<T>, H, A>;

/// The Poseidon Merkle Opening
pub type Opening<T, const H: usize, const A: usize> =
    crate::Opening<Item<T>, H, A>;

/// The Poseidon Node type used for the poseidon merkle tree
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
#[cfg_attr(
    feature = "rkyv-impl",
    derive(Archive, Serialize, Deserialize),
    archive_attr(derive(CheckBytes))
)]
pub struct Item<T> {
    pub hash: BlsScalar,
    pub data: T,
}

impl<T> Item<T> {
    /// Create a new Item for the merkle tree
    pub fn new(hash: BlsScalar, data: T) -> Self {
        Self { hash, data }
    }
}

impl<T, const H: usize, const A: usize> Aggregate<H, A> for Item<T>
where
    T: Aggregate<H, A>,
{
    const EMPTY_SUBTREES: [Self; H] = {
        // initialize array for the empty subtrees
        let mut array = [Item {
            hash: BlsScalar::zero(),
            data: T::EMPTY_SUBTREES[0],
        }; H];

        // fill the array for the empty subtrees
        let hash_array = zero_items::<H, A>();
        let mut i = 0;
        while i < H {
            array[i] = Item {
                hash: hash_array[i],
                data: T::EMPTY_SUBTREES[i],
            };
            i += 1;
        }
        array
    };

    fn aggregate<'a, I>(items: I) -> Self
    where
        Self: 'a,
        I: Iterator<Item = &'a Self>,
    {
        let mut level_hashes = [BlsScalar::zero(); A];
        let mut level_data = [T::EMPTY_SUBTREES[0]; A];
        // grab hashes and data
        items.enumerate().for_each(|(i, item)| {
            level_hashes[i] = item.hash;
            level_data[i] = item.data;
        });

        // create new aggregated item
        Item {
            hash: poseidon_hash(&level_hashes),
            data: T::aggregate(level_data.iter()),
        }
    }
}
