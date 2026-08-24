// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use alloc::vec::Vec;

use dusk_bytes::{DeserializableSlice, Error as BytesError, Serializable};
#[cfg(feature = "rkyv-impl")]
use rkyv::{Archive, Deserialize, Serialize};

use crate::{Aggregate, Node, Tree, init_array};

/// An opening for a given position in a merkle tree.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "rkyv-impl", derive(Archive, Serialize, Deserialize))]
pub struct Opening<T, const H: usize, const A: usize> {
    root: T,
    branch: [[T; A]; H],
    positions: [usize; H],
}

#[cfg(feature = "rkyv-impl")]
mod rkyv_impl {
    use core::{fmt, ptr};

    use bytecheck::{CheckBytes, ErrorBox, StructCheckError};
    use rkyv::{Archive, Archived};

    use super::ArchivedOpening;

    #[derive(Debug)]
    struct PositionOutOfRange {
        index: usize,
        position: u128,
        arity: u128,
    }

    impl fmt::Display for PositionOutOfRange {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(
                f,
                "position at index {} is {}, but arity is {}",
                self.index, self.position, self.arity
            )
        }
    }

    impl core::error::Error for PositionOutOfRange {}

    impl<C, T, const H: usize, const A: usize> CheckBytes<C>
        for ArchivedOpening<T, H, A>
    where
        C: ?Sized,
        T: Archive,
        Archived<T>: CheckBytes<C>,
        Archived<[[T; A]; H]>: CheckBytes<C>,
        Archived<[usize; H]>: CheckBytes<C>,
    {
        type Error = StructCheckError;

        unsafe fn check_bytes<'a>(
            value: *const Self,
            context: &mut C,
        ) -> Result<&'a Self, Self::Error> {
            // SAFETY: The caller guarantees that `value` is aligned and points
            // to enough bytes for the archived opening, so each field pointer
            // is aligned and points to enough bytes for its field.
            unsafe {
                Archived::<T>::check_bytes(
                    ptr::addr_of!((*value).root),
                    context,
                )
            }
            .map_err(|error| StructCheckError {
                field_name: "root",
                inner: ErrorBox::new(error),
            })?;
            unsafe {
                Archived::<[[T; A]; H]>::check_bytes(
                    ptr::addr_of!((*value).branch),
                    context,
                )
            }
            .map_err(|error| StructCheckError {
                field_name: "branch",
                inner: ErrorBox::new(error),
            })?;
            let positions = unsafe {
                Archived::<[usize; H]>::check_bytes(
                    ptr::addr_of!((*value).positions),
                    context,
                )
            }
            .map_err(|error| StructCheckError {
                field_name: "positions",
                inner: ErrorBox::new(error),
            })?;

            for (index, position) in positions.iter().enumerate() {
                let position = u128::from(rkyv::FixedUsize::from(*position));
                let arity = A as u128;
                if position >= arity {
                    return Err(StructCheckError {
                        field_name: "positions",
                        inner: ErrorBox::new(PositionOutOfRange {
                            index,
                            position,
                            arity,
                        }),
                    });
                }
            }

            // SAFETY: All fields were structurally validated above.
            let checked = unsafe { &*value };
            let Self {
                root: _,
                branch: _,
                positions: _,
            } = checked;
            Ok(checked)
        }
    }
}

impl<T, const H: usize, const A: usize> Opening<T, H, A>
where
    T: Aggregate<A> + Clone,
{
    pub(crate) fn new(tree: &Tree<T, H, A>, position: u64) -> Option<Self> {
        let positions = [0; H];
        let branch = init_array(|_| init_array(|_| T::EMPTY_SUBTREE));

        let mut opening = Self {
            root: T::EMPTY_SUBTREE,
            branch,
            positions,
        };
        fill_opening(&mut opening, &tree.root, 0, position)?;
        opening.root = tree.root.item(0).clone();

        Some(opening)
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

            // Reject out-of-range positions and computed items that do not
            // match the stored item at the given position.
            if position >= A || item != level[position] {
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
        for level in &self.branch {
            for item in level {
                bytes.extend(&item.to_bytes());
            }
        }

        // serialize positions
        for pos in self.positions {
            // the positions will be in the range [0..A[, so casting to u32
            // is never going to be a problem
            #[allow(clippy::cast_possible_truncation)]
            bytes.extend(&(pos as u32).to_bytes());
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
        for level in &mut branch {
            for item in &mut *level {
                *item = T::from_reader(&mut bytes)?;
            }
        }

        // deserialize positions
        let mut positions = [0usize; H];
        for pos in &mut positions {
            let p = u32::from_reader(&mut bytes)? as usize;
            if p >= A {
                return Err(BytesError::InvalidData);
            }
            *pos = p;
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
) -> Option<()>
where
    T: Aggregate<A> + Clone,
{
    if height == H {
        return node.has_cached_item().then_some(());
    }

    let (child_index, child_pos) =
        Node::<T, H, A>::child_location(height, position);
    let child = node.children.get(child_index)?.as_ref()?;

    fill_opening(opening, child, height + 1, child_pos)?;

    for i in 0..A {
        if let Some(child) = &node.children[i]
            && let Some(item) = child.populated_item(height + 1)
        {
            opening.branch[height][i] = item.clone();
        }
    }
    opening.positions[height] = child_index;
    Some(())
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
                !opening.verify(LETTERS[((pos + 1) % cap) as usize]),
                "The opening should *only* be for the item that was inserted at the given position"
            );
        }
    }

    #[test]
    fn opening_verify_rejects_out_of_range_position() {
        let mut tree = TestTree::new();
        tree.insert(0, 'A');

        let mut opening = tree
            .opening(0)
            .expect("There must be an opening for an existing item");
        assert!(opening.verify('A'));

        opening.positions[H - 1] = A;

        assert!(!opening.verify('A'));
    }

    #[cfg(feature = "rkyv-impl")]
    mod rkyv_tests {
        extern crate std;

        use rkyv::{AlignedVec, Archived};

        use self::std::panic::{AssertUnwindSafe, catch_unwind};
        use super::*;

        type RkyvOpening = Opening<u8, H, A>;

        fn opening() -> (RkyvOpening, u8) {
            let mut tree = Tree::<u8, H, A>::new();
            let leaf = 42;
            tree.insert(5, leaf);
            (tree.opening(5).expect("the leaf has an opening"), leaf)
        }

        fn position_offset(bytes: &[u8], index: usize) -> usize {
            let archived = rkyv::check_archived_root::<RkyvOpening>(bytes)
                .expect("the unmodified opening is valid");
            let position = archived.positions.as_ptr().wrapping_add(index);

            // SAFETY: Both pointers refer to the same archive allocation.
            unsafe {
                position.cast::<u8>().offset_from(bytes.as_ptr()) as usize
            }
        }

        fn overwrite_position(
            bytes: &mut AlignedVec,
            index: usize,
            position: rkyv::FixedUsize,
        ) {
            let offset = position_offset(bytes, index);
            let archived_position: Archived<usize> = position.into();

            // SAFETY: The offset was obtained from this aligned archive's
            // positions array, and `archived_position` has the field's type.
            unsafe {
                bytes
                    .as_mut_ptr()
                    .add(offset)
                    .cast::<Archived<usize>>()
                    .write(archived_position);
            }
        }

        fn archived_position(bytes: &[u8], index: usize) -> rkyv::FixedUsize {
            // SAFETY: `overwrite_position` only changes an archived `usize`,
            // for which every bit pattern is structurally valid.
            let archived = unsafe { rkyv::archived_root::<RkyvOpening>(bytes) };
            rkyv::FixedUsize::from(archived.positions[index])
        }

        #[test]
        fn valid_opening_roundtrips_and_verifies() {
            let (opening, leaf) = opening();
            assert!(opening.verify(leaf));

            let bytes = rkyv::to_bytes::<_, 256>(&opening)
                .expect("archiving an opening should succeed");
            let roundtrip = rkyv::from_bytes::<RkyvOpening>(&bytes)
                .expect("a valid opening should deserialize");

            assert_eq!(roundtrip, opening);
            assert!(roundtrip.verify(leaf));
        }

        #[test]
        fn archived_positions_at_or_above_arity_are_rejected_without_unwind() {
            let (opening, _) = opening();
            let bytes = rkyv::to_bytes::<_, 256>(&opening)
                .expect("archiving an opening should succeed");
            rkyv::check_archived_root::<RkyvOpening>(&bytes)
                .expect("the unmodified archive should be valid");

            // Include a value whose lower half is zero so narrowing to a
            // smaller host `usize` cannot accidentally make it valid.
            let high_bits_only =
                (1u128 << (rkyv::FixedUsize::BITS / 2)) as rkyv::FixedUsize;
            let invalid_positions = [
                A as rkyv::FixedUsize,
                (A + 1) as rkyv::FixedUsize,
                high_bits_only,
                rkyv::FixedUsize::MAX,
            ];
            for index in 0..H {
                for position in invalid_positions {
                    let mut malformed = bytes.clone();
                    overwrite_position(&mut malformed, index, position);
                    assert_eq!(archived_position(&malformed, index), position);

                    let result = catch_unwind(AssertUnwindSafe(|| {
                        rkyv::from_bytes::<RkyvOpening>(&malformed)
                    }));
                    let result = result.expect(
                        "checked deserialization must not unwind for an out-of-range position",
                    );
                    assert!(
                        result.is_err(),
                        "position {position} at index {index} should be rejected"
                    );
                }
            }
        }
    }
}
