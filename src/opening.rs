// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{Aggregator, Node, Tree};

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
pub struct Opening<A: Aggregator, const HEIGHT: usize, const ARITY: usize> {
    // The root is included in the branch, and is kept at position (0, 0).
    root: A::Item,
    branch: [[A::Item; ARITY]; HEIGHT],
    positions: [usize; HEIGHT],
}

impl<A: Aggregator, const HEIGHT: usize, const ARITY: usize>
    Opening<A, HEIGHT, ARITY>
{
    /// # Panics
    /// If the given `position` is not in the `tree`.
    pub(crate) fn new(tree: &Tree<A, HEIGHT, ARITY>, position: u64) -> Self
    where
        <A as Aggregator>::Item: Clone,
    {
        let positions = [0; HEIGHT];
        let branch = zero_array(|h| zero_array(|_| A::zero_item(h)));
        let root = tree
            .root()
            .expect("There must be a root of the tree")
            .clone();

        let mut opening = Self {
            root,
            branch,
            positions,
        };
        fill_opening(&mut opening, &tree.root, 0, position);

        opening
    }

    /// Verify the given item is the leaf of the opening, and that the opening
    /// is cryptographically correct.
    pub fn verify<'a, I>(&self, items: I) -> bool
    where
        A::Item: 'a + PartialEq,
        I: IntoIterator<Item = &'a A::Item>,
    {
        let mut hash = A::aggregate(items);

        for h in (0..HEIGHT).rev() {
            let level = &self.branch[h];
            let position = self.positions[h];

            if hash != level[position] {
                return false;
            }

            hash = A::aggregate(&self.branch[h]);
        }

        self.root == hash
    }
}

fn fill_opening<A: Aggregator, const HEIGHT: usize, const ARITY: usize>(
    opening: &mut Opening<A, HEIGHT, ARITY>,
    node: &Node<A, HEIGHT, ARITY>,
    height: usize,
    position: u64,
) where
    <A as Aggregator>::Item: Clone,
{
    if height == HEIGHT {
        return;
    }

    let (child_index, child_pos) =
        Node::<A, HEIGHT, ARITY>::child_location(height, position);
    let child = node.children[child_index]
        .as_ref()
        .expect("There should be a child at this position");

    fill_opening(opening, child, height + 1, child_pos);

    opening.branch[height]
        .iter_mut()
        .zip(&node.children)
        .for_each(|(h, c)| {
            *h = match c {
                Some(c) => c
                    .hash
                    .as_ref()
                    .expect("There should be an item in the child")
                    .clone(),
                None => A::zero_item(height),
            }
        });
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

// [R, 0]                     0

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
    pub struct TestAggregator;
    impl Aggregator for TestAggregator {
        type Item = String;

        fn zero_item(_height: usize) -> Self::Item {
            String::new()
        }

        fn aggregate<'a, I>(items: I) -> Self::Item
        where
            Self::Item: 'a,
            I: IntoIterator<Item = &'a Self::Item>,
        {
            items.into_iter().fold(String::new(), |acc, x| acc + x)
        }
    }

    #[test]
    #[allow(clippy::cast_possible_truncation)]
    fn opening_verify() {
        const HEIGHT: usize = 4;
        const ARITY: usize = 2;

        const LETTERS: &[char] = &[
            'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M',
            'N', 'O', 'P',
        ];

        let mut tree = Tree::<TestAggregator, HEIGHT, ARITY>::new();
        let cap = tree.capacity();

        for i in 0..cap {
            tree.insert(i, [&String::from(LETTERS[i as usize])]);
        }

        for pos in 0..cap {
            let opening = tree
                .opening(pos)
                .expect("There must be an opening for an existing item");

            assert!(
                opening.verify([&String::from(LETTERS[pos as usize])]),
                "The opening should be for the item that was inserted at the given position"
            );

            assert!(
                !opening.verify([&String::from(LETTERS[((pos + 1)%cap) as usize])]),
                "The opening should *only* be for the item that was inserted at the given position"
            );
        }
    }
}
