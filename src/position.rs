// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#[cfg(feature = "rkyv-impl")]
use bytecheck::CheckBytes;
#[cfg(feature = "rkyv-impl")]
use rkyv::{Archive, Deserialize, Serialize};

/// An unsigned integer type capable of representing all positions in a tree
/// with a given height and arity.
///
/// Additions and subtractions are intentionally wrapped around the tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(
    feature = "rkyv-impl",
    derive(Archive, Serialize, Deserialize),
    archive_attr(derive(CheckBytes))
)]
#[allow(clippy::module_name_repetitions)]
pub struct TreePosition<const H: usize, const A: usize> {
    indices: [usize; H],
}

impl<const H: usize, const A: usize> TreePosition<H, A> {
    /// The minimum position in a tree.
    pub const MIN: Self = min();
    /// The maximum position in a tree.
    pub const MAX: Self = max();

    /// Returns the positions of siblings in each node, starting from the root.
    pub(crate) const fn indices(&self) -> &[usize; H] {
        &self.indices
    }
}

macro_rules! impl_int {
    ($($int:tt)*) => {
        $(
        impl<const H: usize, const A: usize> From<$int> for TreePosition<H, A> {
            fn from(n: $int) -> Self {
                #[allow(clippy::unnecessary_cast)]
                let mut n = n as usize;

                let mut indices = [0; H];
                for i in (0..H).rev() {
                    indices[i] = n % A;
                    n /= A;
                }

                Self { indices }
            }
        }
        )*
    };
}

impl_int!(
    u8 u16 u32 u64 u128 usize
    i8 i16 i32 i64 i128 isize
);

impl<const H: usize, const A: usize> Default for TreePosition<H, A> {
    fn default() -> Self {
        Self::MIN
    }
}

const fn max<const H: usize, const A: usize>() -> TreePosition<H, A> {
    let mut indices = [0; H];

    let mut h = 0;
    while h < H {
        indices[h] = A - 1;
        h += 1;
    }

    TreePosition { indices }
}

const fn min<const H: usize, const A: usize>() -> TreePosition<H, A> {
    TreePosition { indices: [0; H] }
}
