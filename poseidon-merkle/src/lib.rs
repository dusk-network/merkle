// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![doc = include_str!("../README.md")]
#![no_std]
#![deny(clippy::pedantic)]

#[cfg(feature = "zk")]
pub mod zk;

use dusk_bls12_381::BlsScalar;
use dusk_bytes::Serializable;
use dusk_merkle::Aggregate;
use dusk_poseidon::{Domain, Hash};

pub const ARITY: usize = 4;

/// An alias for a tree containing `Item<T>`.
pub type Tree<T, const H: usize> = dusk_merkle::Tree<Item<T>, H, ARITY>;

/// An alias for an opening of a tree containing `Item<T>`.
pub type Opening<T, const H: usize> = dusk_merkle::Opening<Item<T>, H, ARITY>;

/// A type that wraps a piece of data `T` together with a poseidon hash - i.e. a
/// [`BlsScalar`].
///
/// It implements [`Aggregate`] for any `T` that also implements the trait,
/// allowing for the construction of a poseidon tree without the need to define
/// where the aggregation of hashes is predefined.
///
/// # Example
/// ```rust
/// use std::cmp::{max, min};
///
/// use dusk_bls12_381::BlsScalar;
/// use dusk_merkle::Aggregate;
/// use dusk_poseidon::{Domain, Hash};
/// use poseidon_merkle::{ARITY, Item, Tree as PoseidonTree};
///
/// const H: usize = 17;
///
/// // Leaf struct that carries some data and the current block-height.
/// struct Leaf {
///     leaf_data: BlsScalar,
///     bh: usize,
/// }
///
/// // A node of the merkle tree that keeps track of the min and max
/// // block-height of all of it's children nodes.
/// struct BHRange {
///     min: Option<usize>,
///     max: Option<usize>,
/// }
///
/// // Implement `Aggragate` only for the `BHRange`
/// impl Aggregate<ARITY> for BHRange {
///     const EMPTY_SUBTREE: BHRange = BHRange {
///         min: None,
///         max: None,
///     };
///
///     fn aggregate(items: [&Self; ARITY]) -> Self {
///         let mut parent = Self::EMPTY_SUBTREE;
///
///         for child in items {
///             parent.min = match (parent.min, child.min) {
///                 (Some(parent_min), Some(child_min)) => {
///                     Some(min(parent_min, child_min))
///                 }
///                 (Some(parent_min), None) => Some(parent_min),
///                 (None, Some(child_min)) => Some(child_min),
///                 (None, None) => None,
///             };
///             parent.max = match (parent.max, child.max) {
///                 (Some(parent_max), Some(child_max)) => {
///                     Some(max(parent_max, child_max))
///                 }
///                 (Some(parent_max), None) => Some(parent_max),
///                 (None, Some(child_max)) => Some(child_max),
///                 (None, None) => None,
///             }
///         }
///
///         parent
///     }
/// }
///
/// // Create a merkle tree using the poseidon-hash for each level
/// type Tree = PoseidonTree<BHRange, H>;
/// let mut tree = Tree::new();
///
/// let leaf = Leaf {
///     leaf_data: BlsScalar::from(42),
///     bh: 42,
/// };
///
/// let item = Item {
///     hash: Hash::digest(Domain::Other, &[leaf.leaf_data])[0],
///     data: BHRange {
///         min: Some(leaf.bh),
///         max: Some(leaf.bh),
///     },
/// };
/// tree.insert(42, item);
/// ```
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
#[cfg_attr(
    feature = "rkyv-impl",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    archive_attr(derive(bytecheck::CheckBytes))
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

impl<T> Aggregate<ARITY> for Item<T>
where
    T: Aggregate<ARITY>,
{
    const EMPTY_SUBTREE: Self = Item {
        hash: BlsScalar::zero(),
        data: T::EMPTY_SUBTREE,
    };

    fn aggregate(items: [&Self; ARITY]) -> Self {
        let empty = &T::EMPTY_SUBTREE;

        let mut level_hashes = [BlsScalar::zero(); ARITY];
        let mut level_data = [empty; ARITY];

        // grab hashes and data
        items.into_iter().enumerate().for_each(|(i, item)| {
            level_hashes[i] = item.hash;
            level_data[i] = &item.data;
        });

        // create new aggregated item with the hash being the poseidon hash of
        // the previous level
        Item {
            hash: Hash::digest(Domain::Merkle4, &level_hashes)[0],
            data: T::aggregate(level_data),
        }
    }
}

impl Serializable<32> for Item<()> {
    type Error = <BlsScalar as Serializable<32>>::Error;

    fn from_bytes(buf: &[u8; 32]) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        Ok(Item {
            hash: <BlsScalar as Serializable<32>>::from_bytes(buf)?,
            data: (),
        })
    }

    fn to_bytes(&self) -> [u8; 32] {
        self.hash.to_bytes()
    }
}
