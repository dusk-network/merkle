// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{Aggregate, Node, Tree, TreePosition};

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
    branch: [[Option<T>; A]; H],
    positions: [usize; H],
}

impl<T: Aggregate, const H: usize, const A: usize> Opening<T, H, A> {
    /// # Panics
    /// If the given `position` is not in the `tree`.
    pub(crate) fn new(
        tree: &Tree<T, H, A>,
        position: &TreePosition<H, A>,
    ) -> Self
    where
        T: Clone,
    {
        let positions = [0; H];
        let branch = zero_array(|_| zero_array(|_| None));
        let root = &tree.root;

        let mut opening = Self {
            root: root.item.clone(),
            branch,
            positions,
        };
        fill_opening(&mut opening, root, 0, position);

        opening
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

            if Some(item) != level[position] {
                return false;
            }

            let null = T::NULL;

            item = T::aggregate(
                self.branch[h]
                    .iter()
                    .map(|item| item.as_ref().unwrap_or(&null)),
            );
        }

        self.root == item
    }
}

fn fill_opening<T, const H: usize, const A: usize>(
    opening: &mut Opening<T, H, A>,
    node: &Node<T, H, A>,
    height: usize,
    position: &TreePosition<H, A>,
) where
    T: Aggregate + Clone,
{
    if height == H {
        return;
    }

    let child_index = position.indices()[height];
    let child = node.children[child_index]
        .as_ref()
        .expect("There should be a child at this position");

    fill_opening(opening, child, height + 1, position);

    opening.branch[height]
        .iter_mut()
        .zip(&node.children)
        .for_each(|(h, c)| *h = c.as_ref().map(|node| node.item.clone()));
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

    extern crate alloc;
    use alloc::string::String;

    /// A simple aggregator that concatenates strings.
    impl Aggregate for String {
        const NULL: Self = String::new();

        fn aggregate<'a, I>(items: I) -> Self
        where
            Self: 'a,
            I: ExactSizeIterator<Item = &'a Self>,
        {
            items.into_iter().fold(Self::NULL, |acc, s| acc + s)
        }
    }

    const H: usize = 4;
    const A: usize = 2;

    type TestTree = Tree<String, H, A>;

    #[test]
    #[allow(clippy::cast_possible_truncation)]
    fn opening_verify() {
        const LETTERS: &[char] = &[
            'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M',
            'N', 'O', 'P',
        ];

        let mut tree = TestTree::new();
        let cap = A.pow(H as u32);

        for (i, l) in LETTERS.iter().enumerate() {
            tree.insert(i, *l);
        }

        for (i, _) in LETTERS.iter().enumerate() {
            let opening = tree
                .opening(i)
                .expect("There must be an opening for an existing item");

            assert!(
                opening.verify(LETTERS[i]),
                "The opening should be for the item that was inserted at the given position"
            );

            assert!(
                !opening.verify(LETTERS[((i + 1)%cap)]),
                "The opening should *only* be for the item that was inserted at the given position"
            );
        }
    }
}
