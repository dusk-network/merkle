// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#[cfg(feature = "zk")]
mod zk;

use dusk_bls12_381::BlsScalar;
use dusk_poseidon::sponge::hash as poseidon_hash;

use crate::Aggregate;

/// An alias for a tree containing `Item<T>`.
pub type Tree<T, const H: usize, const A: usize> = crate::Tree<Item<T>, H, A>;

/// An alias for an opening of a tree containing `Item<T>`.
pub type Opening<T, const H: usize, const A: usize> =
    crate::Opening<Item<T>, H, A>;

/// A type that wraps a piece of data `T` together with a poseidon hash - i.e. a
/// [`BlsScalar`].
///
/// It implements [`Aggregate`] for any `T` that also implements the trait,
/// allowing for the construction of a poseidon tree without the need to define
/// where the aggregation of hashes is predefined.
///
/// # Example
/// ```rust
/// use dusk_bls12_381::BlsScalar;
/// use dusk_merkle::poseidon::{Item, Tree as PoseidonTree};
/// use dusk_poseidon::sponge;
/// use dusk_merkle::Aggregate;
///
/// struct Data(BlsScalar);
///
/// impl From<Data> for Item<Data> {
///     fn from(data: Data) -> Self {
///         Item {
///             hash: sponge::hash(&[data.0]),
///             data,
///         }
///     }
/// }
///
/// impl<const A: usize> Aggregate<A> for Data {
///     const EMPTY_SUBTREE: Data = Data(BlsScalar::zero());
///
///     fn aggregate(items: [&Self; A]) -> Self {
///         Self(items.iter().map(|d| d.0).sum())
///     }
/// }
///
/// const H: usize = 17;
/// const A: usize = 4;
/// type Tree = PoseidonTree<Data, H, A>;
///
/// let mut tree = Tree::new();
/// tree.insert(42, Data(BlsScalar::one()));
/// tree.insert(7, Data(BlsScalar::one()));
/// tree.insert(0xbeef, Data(BlsScalar::one()));
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

impl<T, const A: usize> Aggregate<A> for Item<T>
where
    T: Aggregate<A>,
{
    const EMPTY_SUBTREE: Self = Item {
        hash: BlsScalar::zero(),
        data: T::EMPTY_SUBTREE,
    };

    fn aggregate(items: [&Self; A]) -> Self {
        let empty = &T::EMPTY_SUBTREE;

        let mut level_hashes = [BlsScalar::zero(); A];
        let mut level_data = [empty; A];

        // grab hashes and data
        items.into_iter().enumerate().for_each(|(i, item)| {
            level_hashes[i] = item.hash;
            level_data[i] = &item.data;
        });

        // create new aggregated item
        Item {
            hash: poseidon_hash(&level_hashes),
            data: T::aggregate(level_data),
        }
    }
}
