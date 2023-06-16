// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{init_array, Aggregate, Node, Tree};

use alloc::vec::Vec;

#[cfg(feature = "rkyv-impl")]
use bytecheck::CheckBytes;
use dusk_bytes::{DeserializableSlice, Error as BytesError, Serializable};
#[cfg(feature = "rkyv-impl")]
use rkyv::{Archive, Deserialize, Serialize};

/// An opening for a given position in a merkle tree.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "rkyv-impl",
    derive(Archive, Serialize, Deserialize),
    archive_attr(derive(CheckBytes))
)]
pub struct Opening<T, const H: usize, const A: usize> {
    root: T,
    branch: [[T; A]; H],
    positions: [usize; H],
}

impl<T, const H: usize, const A: usize> Opening<T, H, A>
where
    T: Aggregate<A> + Clone,
{
    /// # Panics
    /// If the given `position` is not in the `tree`.
    pub(crate) fn new(tree: &Tree<T, H, A>, position: u64) -> Self {
        let positions = [0; H];
        let branch = init_array(|_| init_array(|_| T::EMPTY_SUBTREE));

        let mut opening = Self {
            root: tree.root.item().clone(),
            branch,
            positions,
        };
        fill_opening(&mut opening, &tree.root, 0, position);

        opening
    }

    /// Returns the root of the opening.
    pub fn root(&self) -> &T {
        &self.root
    }

    /// Returns the branch of the opening.
    pub fn branch(&self) -> &[[T; A]; H] {
        &self.branch
    }

    /// Returns the indices for the path in the opening.
    pub fn positions(&self) -> &[usize; H] {
        &self.positions
    }

    /// Verify the given item is the leaf of the opening, and that the opening
    /// is cryptographically correct.
    pub fn verify(&self, item: impl Into<T>) -> bool
    where
        T: PartialEq,
    {
        let mut item = item.into();

        for h in (0..H).rev() {
            let level = &self.branch[h];
            let position = self.positions[h];

            // if the computed item doesn't match the stored item at the given
            // position, the opening is incorrect
            if item != level[position] {
                return false;
            }

            let empty_subtree = &T::EMPTY_SUBTREE;

            let mut item_refs = [empty_subtree; A];
            item_refs.iter_mut().zip(&self.branch[h]).for_each(
                |(r, item_ref)| {
                    *r = item_ref;
                },
            );

            item = T::aggregate(item_refs);
        }

        self.root == item
    }

    /// Serialize an [`Opening`] to a vector of bytes.
    // Once the new implementation of the `Serializable` trait becomes
    // available, we will want that instead, but for the time being we use
    // this implementation.
    pub fn to_var_bytes<const T_SIZE: usize>(&self) -> Vec<u8>
    where
        T: Serializable<T_SIZE>,
    {
        let mut bytes = Vec::with_capacity(
            (1 + H * A) * T_SIZE + H * (u32::BITS as usize / 8),
        );

        // serialize root
        bytes.extend(&self.root.to_bytes());

        // serialize branch
        for level in self.branch.iter() {
            for item in level.iter() {
                bytes.extend(&item.to_bytes());
            }
        }

        // serialize positions
        for pos in self.positions.iter() {
            // the positions will be in the range [0..A[, so casting to u32
            // is never going to be a problem
            #[allow(clippy::cast_possible_truncation)]
            bytes.extend(&(*pos as u32).to_bytes());
        }

        bytes
    }

    /// Deserialize an [`Opening`] from a slice of bytes.
    ///
    /// # Errors
    ///
    /// Will return [`dusk_bytes::Error`] in case of a deserialization error.
    // Once the new implementation of the `Serializable` trait becomes
    // available, we will want that instead, but for the time being we use
    // this implementation.
    pub fn from_slice<const T_SIZE: usize>(
        buf: &[u8],
    ) -> Result<Self, BytesError>
    where
        T: Serializable<T_SIZE>,
        <T as Serializable<T_SIZE>>::Error: dusk_bytes::BadLength,
        dusk_bytes::Error: From<<T as Serializable<T_SIZE>>::Error>,
    {
        let expected_len = (1 + H * A) * T_SIZE + H * (u32::BITS as usize / 8);
        if buf.len() != expected_len {
            return Err(BytesError::BadLength {
                found: (buf.len()),
                expected: (expected_len),
            });
        }

        let mut bytes = buf;

        // deserialize root
        let root = T::from_reader(&mut bytes)?;

        // deserialize branch
        let mut branch: [[T; A]; H] =
            init_array(|_| init_array(|_| T::EMPTY_SUBTREE));
        for level in branch.iter_mut() {
            for item in level.iter_mut() {
                *item = T::from_reader(&mut bytes)?;
            }
        }

        // deserialize positions
        let mut positions = [0usize; H];
        for pos in positions.iter_mut() {
            *pos = u32::from_reader(&mut bytes)? as usize;
        }

        Ok(Self {
            root,
            branch,
            positions,
        })
    }
}

fn fill_opening<T, const H: usize, const A: usize>(
    opening: &mut Opening<T, H, A>,
    node: &Node<T, H, A>,
    height: usize,
    position: u64,
) where
    T: Aggregate<A> + Clone,
{
    if height == H {
        return;
    }

    let (child_index, child_pos) =
        Node::<T, H, A>::child_location(height, position);
    let child = node.children[child_index]
        .as_ref()
        .expect("There should be a child at this position");

    fill_opening(opening, child, height + 1, child_pos);

    for i in 0..A {
        if let Some(child) = &node.children[i] {
            opening.branch[height][i] = child.item().clone();
        }
    }
    opening.positions[height] = child_index;
}

#[cfg(test)]
mod tests {
    use super::*;

    const H: usize = 4;
    const A: usize = 2;
    const TREE_CAP: usize = A.pow(H as u32);

    /// A string type that is on the stack, and holds a string of a size as
    /// large as the tree.
    #[derive(Clone, Copy, PartialEq)]
    struct String {
        chars: [char; TREE_CAP],
        len: usize,
    }

    impl From<char> for String {
        fn from(c: char) -> Self {
            let mut chars = ['0'; TREE_CAP];
            chars[0] = c;
            Self { chars, len: 1 }
        }
    }

    const EMPTY_ITEM: String = String {
        chars: ['0'; TREE_CAP],
        len: 0,
    };

    /// A simple aggregator that concatenates strings.
    impl Aggregate<A> for String {
        const EMPTY_SUBTREE: Self = EMPTY_ITEM;

        fn aggregate(items: [&Self; A]) -> Self {
            items.into_iter().fold(EMPTY_ITEM, |mut acc, s| {
                acc.chars[acc.len..acc.len + s.len]
                    .copy_from_slice(&s.chars[..s.len]);
                acc.len += s.len;
                acc
            })
        }
    }

    type TestTree = Tree<String, H, A>;

    #[test]
    #[allow(clippy::cast_possible_truncation)]
    fn opening_verify() {
        const LETTERS: &[char] = &[
            'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M',
            'N', 'O', 'P',
        ];

        let mut tree = TestTree::new();
        let cap = tree.capacity();

        for i in 0..cap {
            tree.insert(i, LETTERS[i as usize]);
        }

        for pos in 0..cap {
            let opening = tree
                .opening(pos)
                .expect("There must be an opening for an existing item");

            assert!(
                opening.verify(LETTERS[pos as usize]),
                "The opening should be for the item that was inserted at the given position"
            );

            assert!(
                !opening.verify(LETTERS[((pos + 1)%cap) as usize]),
                "The opening should *only* be for the item that was inserted at the given position"
            );
        }
    }
}
