// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{Aggregate, Node, Tree};

use core::mem::MaybeUninit;
use core::ptr;

#[cfg(feature = "rkyv-impl")]
use bytecheck::CheckBytes;
#[cfg(feature = "rkyv-impl")]
use rkyv::{Archive, Deserialize, Serialize};

/// An opening for a given position in a merkle tree.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
    T: Aggregate<H, A>,
{
    /// # Panics
    /// If the given `position` is not in the `tree`.
    pub(crate) fn new(tree: &Tree<T, H, A>, position: u64) -> Self {
        let positions = [0; H];
        let branch = zero_array(|h| zero_array(|_| T::EMPTY_SUBTREES[h]));

        let mut opening = Self {
            root: tree.root.item,
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

            item = T::aggregate(self.branch[h].iter());
        }

        self.root == item
    }
}

fn fill_opening<T, const H: usize, const A: usize>(
    opening: &mut Opening<T, H, A>,
    node: &Node<T, H, A>,
    height: usize,
    position: u64,
) where
    T: Aggregate<H, A>,
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
            opening.branch[height][i] = child.item;
        }
    }
    opening.positions[height] = child_index;
}

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

// R
//
// [H_ABCDEFGH, H_IJKLMNOP]   1
// [H_IJKL, H_MNOP]           0
// [H_IJ, H_KL]               1
// [H_K, H_L]                 0

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
    impl Aggregate<H, A> for String {
        const EMPTY_SUBTREES: [Self; H] = [EMPTY_ITEM; H];

        fn aggregate<'a, I>(items: I) -> Self
        where
            Self: 'a,
            I: Iterator<Item = &'a Self>,
        {
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
